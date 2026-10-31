#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Negative controls for false-green formal verification infrastructure."""
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest import mock
import sys
sys.dont_write_bytecode = True

spec = importlib.util.spec_from_file_location("formal", Path(__file__).with_name("formal.py"))
formal = importlib.util.module_from_spec(spec)
spec.loader.exec_module(formal)


publisher_spec = importlib.util.spec_from_file_location("publisher", Path(__file__).with_name("crates-publisher.py"))
publisher = importlib.util.module_from_spec(publisher_spec)
publisher_spec.loader.exec_module(publisher)


class GateTests(unittest.TestCase):
    def test_frontend_inventory_keeps_project_contracts_out_of_standalone_checker(self):
        root = Path(__file__).resolve().parents[1]
        rows = formal.tomllib.loads((root / "verification/corpus.toml").read_text())["model"]
        # Standalone exports omit frontend labels because only checker cases remain.
        if not (root / "crates/bboxy450-term").exists():
            return
        self.assertEqual({row["frontend"] for row in rows}, {"checker", "yarn", "project"})
        project = {n for c in formal.contracts(root, {r["name"] for r in rows}) for n in c["models"]}
        self.assertEqual({r["name"] for r in rows if r["frontend"] == "project"}, project)
        self.assertEqual(len([r for r in rows if r["frontend"] == "checker"]), 12)

    def test_project_contracts_require_executed_pairs_and_real_evidence_paths(self):
        with tempfile.TemporaryDirectory() as own:
            root = Path(own)
            (root / "verification").mkdir()
            (root / "source.rs").touch()
            (root / "witness.rs").touch()
            path = root / "verification/contracts.toml"
            record = ('[[contract]]\nname="Boundary"\ntitle="Boundary"\n'
                      'models=["Boundary","BrokenBoundary"]\nsources=["source.rs"]\n'
                      'implementation_tests=["witness.rs"]\nassumptions="one job"\n'
                      'property="current completion"\nchecks=["make test-formal","make test"]\n')
            path.write_text(record)
            self.assertEqual(len(formal.contracts(root, {"Boundary", "BrokenBoundary"})), 1)
            with self.assertRaisesRegex(ValueError, "pair"):
                formal.contracts(root, {"Boundary"})
            path.write_text(record + record)
            with self.assertRaisesRegex(ValueError, "duplicate"):
                formal.contracts(root, {"Boundary", "BrokenBoundary"})
            path.write_text(record)
            (root / "witness.rs").unlink()
            with self.assertRaisesRegex(ValueError, "implementation_tests"):
                formal.contracts(root, {"Boundary", "BrokenBoundary"})

    def test_contract_metadata_cannot_hide_bounds_or_claim_rust_refinement(self):
        with tempfile.TemporaryDirectory() as own:
            root = Path(own)
            path = root / "Boundary.metadata.json"
            data = {"variables": [{"min": 0, "initial": 0, "max": 1}],
                    "actions": ["Complete"], "invariants": ["Current"],
                    "implementation_refinement_proven": False}
            path.write_text(json.dumps(data))
            formal.contract_metadata(root, "Boundary")
            for changed in [{**data, "variables": []},
                            {**data, "implementation_refinement_proven": True},
                            {**data, "variables": [{"min": 0, "initial": 2, "max": 1}]}]:
                path.write_text(json.dumps(changed))
                with self.assertRaises(ValueError):
                    formal.contract_metadata(root, "Boundary")

    def test_publisher_refuses_missing_wrong_and_stale_identity_without_any_upload(self):
        token = "private-fixture-token"
        digest = formal.hashlib.sha256(token.encode()).hexdigest()
        receipt = {"publisher": "bboxy450", "source": "operator-confirmed", "token_sha256": digest}
        publisher.verify_identity(token, receipt)
        for changed in [{}, {**receipt, "publisher": "other"}, {**receipt, "source": "agent-guessed"},
                        {**receipt, "token_sha256": "stale"}]:
            with self.assertRaises(ValueError):
                publisher.verify_identity(token, changed)

    def test_publication_order_includes_renamed_dev_and_platform_dependencies(self):
        def package(name, dependencies=(), publish=None):
            return {"id": name, "name": name, "version": "0.1.1", "publish": publish,
                    "dependencies": [{"name": dep} for dep in dependencies]}
        data = {"workspace_members": ["app", "render", "firehose", "private"], "packages": [
            package("app", ["render"]), package("render", ["firehose"]),
            package("firehose"), package("private", publish=[])]}
        self.assertEqual([p["name"] for p in publisher.publication_order(data)],
                         ["firehose", "render", "app"])
        self.assertEqual([p["name"] for p in publisher.publication_order(data, ["app", "firehose"])],
                         ["firehose", "app"])
        with self.assertRaisesRegex(ValueError, "publishable"):
            publisher.publication_order(data, ["private"])
        data["packages"][2]["dependencies"] = [{"name": "app"}]
        with self.assertRaisesRegex(ValueError, "cycle"):
            publisher.publication_order(data)

    def test_rate_limit_retries_without_a_count_limit_and_other_errors_fail(self):
        limited = subprocess.CompletedProcess([], 101, "", 'HTTP 429\nRetry-After: 2')
        success = subprocess.CompletedProcess([], 0, "uploaded", "")
        with mock.patch.object(publisher.subprocess, "run", side_effect=[limited] * 5 + [success]) as run, \
                mock.patch.object(publisher.time, "sleep") as sleep:
            publisher.run_retry(["cargo", "publish"])
        self.assertEqual(run.call_count, 6)
        self.assertEqual(sleep.call_args_list, [mock.call(2)] * 5)
        with mock.patch.object(publisher.subprocess, "run", return_value=limited.__class__([], 101, "", "build failed")):
            with self.assertRaises(subprocess.CalledProcessError):
                publisher.run_retry(["cargo", "publish"])
        self.assertEqual(publisher.retry_delay({}, "retry in 3 minutes"), 180)
        self.assertEqual(publisher.retry_delay({"Retry-After": "7"}, ""), 7)
        self.assertEqual(publisher.retry_delay({}, "Try again after 2026-10-31T18:00:00Z", now=1793469540), 60)
        message = ('You have published too many new crates in a short period of time. '
                   'Please try again after Sat, 31 Oct 2026 17:34:35 GMT and see '
                   'https://crates.io/docs/rate-limits for more details.')
        self.assertEqual(publisher.retry_delay({}, message, now=1793468040), 35)

    def test_publication_skips_existing_versions_and_waits_before_the_dependent(self):
        packages = [{"name": n, "version": "0.1.1"} for n in ["base", "app"]]
        events = []
        with tempfile.TemporaryDirectory() as own, \
                mock.patch.object(publisher, "version_exists", side_effect=[True, False]), \
                mock.patch.object(publisher, "wait_index", side_effect=lambda p: events.append(("index", p["name"]))), \
                mock.patch.object(publisher, "run_retry", side_effect=lambda argv, **kw: events.append(tuple(argv))), \
                mock.patch.object(publisher, "verify_artifact", return_value="fixture-sha256"):
            rows = publisher.publish_packages(packages, Path(own), {})
        self.assertEqual(events[0], ("index", "base"))
        self.assertEqual(events[1][0], "make")
        self.assertIn("CRATE=app", events[1])
        self.assertEqual(events[2][:2], ("cargo", "publish"))
        self.assertIn("--registry", events[2])
        self.assertEqual(events[3], ("index", "app"))
        self.assertEqual([r["status"] for r in rows], ["already-published", "published"])

    def test_registry_429_retries_and_index_waits_for_the_exact_live_version(self):
        limited = publisher.urllib.error.HTTPError("https://fixture.invalid", 429, "limited",
                                                  {"Retry-After": "3"}, io.BytesIO(b"limited"))
        response = mock.MagicMock()
        response.__enter__.return_value.read.return_value = b"indexed"
        with mock.patch.object(publisher.urllib.request, "urlopen", side_effect=[limited, response]), \
                mock.patch.object(publisher.time, "sleep") as sleep:
            self.assertEqual(publisher.registry_read("https://fixture.invalid"), b"indexed")
        sleep.assert_called_once_with(3)
        rows = [None, b'{"vers":"0.1.0","yanked":false}',
                b'{"vers":"0.1.1","yanked":true}', b'{"vers":"0.1.1","yanked":false}']
        with mock.patch.object(publisher, "registry_read", side_effect=rows), \
                mock.patch.object(publisher.time, "sleep") as sleep:
            publisher.wait_index({"name": "bboxy450-term", "version": "0.1.1"})
        self.assertEqual(sleep.call_count, 3)

    def test_publish_refuses_an_invalid_signature_before_upload(self):
        with tempfile.TemporaryDirectory() as own, \
                mock.patch.object(publisher, "version_exists", return_value=False), \
                mock.patch.object(publisher, "run_retry") as run, \
                mock.patch.object(publisher, "verify_artifact", side_effect=ValueError("bad signature")):
            with self.assertRaisesRegex(ValueError, "signature"):
                publisher.publish_packages([{"name": "app", "version": "0.1.1"}], Path(own), {})
        self.assertEqual(run.call_count, 1)
        self.assertEqual(run.call_args.args[0][0], "make")

    def test_workspace_publish_make_gate_requires_exact_normal_clean_bboxy_identity(self):
        root = Path(__file__).resolve().parents[1]
        if not (root / "crates/bboxy450-term").exists():
            return  # The standalone checker has no workspace publication target.
        with tempfile.TemporaryDirectory() as own:
            own = Path(own)
            (own / "bin").mkdir()
            (own / "scripts").mkdir()
            (own / "scripts/cache-env.sh").write_text(":\n")
            fixtures = {
                "gh": 'printf "%s\\n" "$FIXTURE_ACTOR"',
                "git": 'case "$1" in status) printf "%s" "$FIXTURE_DIRTY";; rev-parse) echo fixture-head;; esac',
                "python3": 'echo invoked > "$FIXTURE_CALLED"',
            }
            for name, body in fixtures.items():
                path = own / "bin" / name
                path.write_text("#!/bin/sh\n" + body + "\n")
                path.chmod(0o700)
            called, log = own / "called", own / "ci.log"
            env = {k: v for k, v in publisher.os.environ.items() if k not in {"MAKEFLAGS", "MFLAGS", "MAKELEVEL"}}
            env.update(PATH=str(own / "bin") + ":" + env["PATH"], FIXTURE_CALLED=str(called))
            for actor, dirty, receipt, accepted in [
                ("bboxy450", "", "fixture-head normal 0", True),
                ("other", "", "fixture-head normal 0", False),
                ("bboxy450", " M file", "fixture-head normal 0", False),
                ("bboxy450", "", "old-head normal 0", False),
                ("bboxy450", "", "fixture-head extended 0", False),
            ]:
                called.unlink(missing_ok=True)
                log.write_text("ci-local: result " + receipt + "\n")
                result = subprocess.run(["make", "-f", str(root / "Makefile"), "publish-crates", f"CI_LOG={log}"],
                                        cwd=own, env={**env, "FIXTURE_ACTOR": actor, "FIXTURE_DIRTY": dirty},
                                        capture_output=True, text=True)
                self.assertEqual(result.returncode == 0, accepted, result.stderr)
                self.assertEqual(called.exists(), accepted)

    def test_tool_errors_and_incomplete_success_are_not_property_failures(self):
        for code, output in [(0, "Parsing complete"), (12, "Syntax error"),
                             (13, "internal failure"), (1, "Temporal properties were violated")]:
            with self.assertRaises(ValueError):
                formal.verdict(subprocess.CompletedProcess([], code, output))

    def test_checksum_refuses_an_existing_tampered_tool(self):
        with tempfile.TemporaryDirectory() as own:
            path = Path(own) / "tool"
            path.write_bytes(b"wrong artifact")
            with self.assertRaisesRegex(ValueError, "checksum"):
                formal.artifact({"sha256": "0" * 64}, path)

    def test_workspace_crate_cannot_omit_coverage(self):
        with tempfile.TemporaryDirectory() as own:
            root = Path(own)
            (root / "verification").mkdir()
            (root / "crate").mkdir()
            (root / "Cargo.toml").write_text('[workspace]\nmembers=["crate"]\n')
            (root / "crate/Cargo.toml").write_text('[package]\nname="new-crate"\n')
            (root / "verification/coverage.toml").write_text('crate=[]\n')
            with self.assertRaisesRegex(ValueError, "missing"):
                formal.coverage(root, set())

    def test_graph_parser_rejects_incomplete_or_unknown_states(self):
        with tempfile.TemporaryDirectory() as own:
            dot = Path(own) / "states.dot"
            dot.write_text('digraph G {\n1 [label="v0 = 1"];\n}\n')
            self.assertEqual(formal.dot_graph(dot, 1)[0], {(1,)})
            with self.assertRaisesRegex(ValueError, "unrecognized"):
                formal.dot_graph(dot, 2)
            dot.write_text('digraph G {}')
            with self.assertRaisesRegex(ValueError, "did not emit"):
                formal.dot_graph(dot, 1)


if __name__ == "__main__":
    unittest.main()
