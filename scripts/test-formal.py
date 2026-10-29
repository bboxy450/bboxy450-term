#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Negative controls for false-green formal verification infrastructure."""
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
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
