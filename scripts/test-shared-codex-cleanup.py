#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Private fixture/process witnesses; no app, Codex, external model or operator.

Finite custody projection: one root, owned/foreign, two live/dead owner bits.
Init: root present with bulky home. Atomic Reclaim may remove only owned roots
with both owners dead. NextRun must reclaim that case. Duplicate Reclaim is safe.
PID reuse is conservatively live; no fairness or arbitrary-filesystem-race proof.
"""
import importlib.util
import itertools
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch


SOURCE = Path(__file__).resolve().parent / "e2e/shared_codex.py"
spec = importlib.util.spec_from_file_location("shared_codex_cleanup", SOURCE)
shared = importlib.util.module_from_spec(spec)
spec.loader.exec_module(shared)
native_spec = importlib.util.spec_from_file_location("native_strip_cleanup", SOURCE.with_name("native_strip.py"))
native = importlib.util.module_from_spec(native_spec)
native_spec.loader.exec_module(native)


class CleanupTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="bbt-cleanup-test-", dir="/tmp")
        self.addCleanup(self.temporary.cleanup)
        self.parent = Path(self.temporary.name).resolve()
        child = subprocess.Popen([sys.executable, "-c", "pass"])
        self.dead_pid = child.pid
        child.wait(timeout=10)

    def exercise(self, fail=False, artifacts=False):
        fixture = shared.FixtureRoot(self.parent)
        child = None

        def body(options, binary, codex, owner):
            nonlocal child
            self.assertIs(owner, fixture)
            home = owner.root / "home/.codex/packages"
            home.mkdir(parents=True)
            (home / "large").write_bytes(b"x" * (1024 * 1024))
            (owner.root / "frames").mkdir()
            (owner.root / "frames/frame.bin").write_bytes(b"x" * 1024)
            (owner.root / "codex-stream.bin").write_bytes(b"stream")
            (owner.root / "app.log").write_bytes(b"a" * (shared.DIAGNOSTIC_BYTES + 100))
            (owner.root / "unexpected.bin").write_bytes(b"private large artifact")
            child = owner.track(subprocess.Popen([sys.executable, "-c", "import signal; signal.pause()"] ))
            if fail:
                raise RuntimeError("deliberate fixture failure")

        destination = self.parent / "explicit-output"
        arguments = [str(SOURCE), "/usr/bin/true", "--codex", "/usr/bin/true"]
        if artifacts:
            destination.mkdir()
            arguments += ["--artifacts", str(destination)]
        try:
            with patch.object(shared, "FixtureRoot", return_value=fixture), \
                 patch.object(shared, "run_fixture", side_effect=body), \
                 patch.object(sys, "argv", arguments):
                if fail:
                    with self.assertRaisesRegex(RuntimeError, "deliberate fixture failure"):
                        shared.main()
                else:
                    shared.main()
            if fail:
                self.assertEqual({p.name for p in fixture.root.iterdir()},
                                 {"app.log", "cleanup-owner.json", "fixture-error.log"})
                self.assertLessEqual((fixture.root / "app.log").stat().st_size, shared.DIAGNOSTIC_BYTES)
                self.assertIn("deliberate fixture failure", (fixture.root / "fixture-error.log").read_text())
                self.assertEqual(json.loads((fixture.root / "cleanup-owner.json").read_text())["script"],
                                 shared.OWNER_TAG)
            else:
                self.assertFalse(fixture.root.exists(), "successful fixture root leaked")
            self.assertIsNotNone(child.poll(), "owned child outlived fixture")
            if artifacts:
                self.assertEqual((destination / "codex-stream.bin").read_bytes(), b"stream")
                self.assertEqual((destination / "frames/frame.bin").stat().st_size, 1024)
                self.assertFalse((destination / "home").exists())
        finally:
            if child is not None and child.poll() is None:
                child.kill()
                child.wait(timeout=10)

    def test_success_removes_root_after_owned_child_exits(self):
        self.exercise()

    def test_failure_keeps_only_bounded_diagnostics(self):
        self.exercise(fail=True)

    def test_cli_hands_off_only_requested_artifacts(self):
        self.exercise(artifacts=True)

    def test_removed_cleanup_fails_the_success_witness(self):
        with patch.object(shared.FixtureRoot, "finish", return_value=None):
            with self.assertRaisesRegex(AssertionError, "successful fixture root leaked"):
                self.exercise()

    def test_next_run_reclaims_dead_and_preserves_live_or_foreign_roots(self):
        states = itertools.product([False, True], [False, True], [False, True],
                                   ["logic", "supervisor", "frontend", "desktop"])
        for owned, runner_live, auxiliary_live, producer in states:
            with self.subTest(owned=owned, runner_live=runner_live,
                              auxiliary_live=auxiliary_live, producer=producer):
                root = shared.FixtureRoot(self.parent).root
                marker = root / "cleanup-owner.json"
                record = json.loads(marker.read_text())
                record["owner_pids"] = [shared.os.getpid() if runner_live else self.dead_pid]
                if not owned:
                    record["script"] = "unrelated script"
                marker.write_text(json.dumps(record))
                auxiliary = root / "run" / (producer + ".owner.json")
                auxiliary.parent.mkdir()
                auxiliary.write_text(json.dumps({"process": {
                    "pid": shared.os.getpid() if auxiliary_live else self.dead_pid}}))
                new = shared.FixtureRoot(self.parent)
                self.assertEqual(root.exists(), not (owned and not runner_live and not auxiliary_live))
                shared.reclaim_roots(self.parent)  # duplicate reclamation changes no live root
                self.assertTrue(new.root.exists())
                new.finish(True)

    def test_unknown_corrupt_and_symlink_custody_is_untouched(self):
        unknown = self.parent / "bbt-shared-codex-unmarked"
        unknown.mkdir()
        corrupt = self.parent / "bbt-shared-codex-corrupt"
        corrupt.mkdir()
        (corrupt / "cleanup-owner.json").write_text("{broken")
        outside = self.parent / "outside"
        outside.mkdir()
        (outside / "keep").write_text("unchanged")
        alias = self.parent / "bbt-shared-codex-alias"
        alias.symlink_to(outside, target_is_directory=True)
        linked = self.parent / "bbt-shared-codex-linked-marker"
        linked.mkdir()
        (linked / "cleanup-owner.json").symlink_to(outside / "keep")
        shared.reclaim_roots(self.parent)
        self.assertTrue(all(p.exists() for p in [unknown, corrupt, alias, linked]))
        self.assertEqual((outside / "keep").read_text(), "unchanged")

    def test_explicit_artifacts_survive_success_without_retaining_the_fixture(self):
        fixture = shared.FixtureRoot(self.parent)
        (fixture.root / "frames").mkdir()
        (fixture.root / "frames/frame.bin").write_bytes(b"frame")
        (fixture.root / "codex-stream.bin").write_bytes(b"stream")
        (fixture.root / "home").mkdir()
        destination = self.parent / "caller-owned-recording"
        destination.mkdir()
        shared.export_artifacts(fixture.root, destination)
        self.assertTrue((fixture.root / "codex-stream.bin").exists())
        fixture.finish(True)
        self.assertFalse(fixture.root.exists())
        self.assertEqual((destination / "frames/frame.bin").read_bytes(), b"frame")
        self.assertEqual((destination / "codex-stream.bin").read_bytes(), b"stream")
        self.assertFalse((destination / "home").exists())
        self.assertEqual({p.name for p in destination.iterdir()}, {"frames", "codex-stream.bin"})
        with self.assertRaises(ValueError):
            shared.export_artifacts(fixture.root, destination)
        with self.assertRaises(ValueError):
            shared.export_artifacts(destination, destination)

    def test_live_auxiliary_owner_prevents_retirement_without_a_signal(self):
        fixture = shared.FixtureRoot(self.parent)
        home = fixture.root / "home/.codex/app-server-daemon"
        home.mkdir(parents=True)
        peer = subprocess.Popen([sys.executable, "-c", "import signal; signal.pause()"])
        try:
            (home / "daemon.pid").write_text(json.dumps({"pid": peer.pid}))
            with self.assertRaisesRegex(RuntimeError, "owners still alive"):
                fixture.finish(False)
            self.assertTrue(home.exists())
            self.assertIsNone(peer.poll())
        finally:
            peer.terminate()
            peer.wait(timeout=10)
        fixture.finish(False)
        self.assertEqual({p.name for p in fixture.root.iterdir()}, {"cleanup-owner.json"})

    def test_native_caller_owns_and_retires_handoff_on_success_and_failure(self):
        for fail in (False, True):
            with self.subTest(fail=fail):
                fixture = shared.FixtureRoot(self.parent)
                (fixture.root / "frames").mkdir()
                (fixture.root / "frames/frame.bin").write_bytes(b"frame")
                data = b"W" * 8192 + b"\x1b[?2026h\x1b[?2026l" * 20
                (fixture.root / "codex-stream.bin").write_bytes(data)
                handed = []
                original_mkdtemp = tempfile.mkdtemp

                def directory(*args, **kwargs):
                    if kwargs.get("prefix") == "bbt-strip-night-":
                        kwargs["dir"] = self.parent
                    return original_mkdtemp(*args, **kwargs)

                def child(command, **kwargs):
                    destination = Path(command[command.index("--artifacts") + 1])
                    handed.append(destination)
                    shared.export_artifacts(fixture.root, destination)
                    fixture.finish(True)
                    if fail:
                        raise RuntimeError("failure after handoff")
                    return subprocess.CompletedProcess(command, 0)

                def replay(binary, stream, control=None, **kwargs):
                    self.assertEqual(stream.read_bytes(), data)
                    if control:
                        raise native.StripDetected(dict(frames=64, failures=[dict(stage=control)]))
                    return dict(passed=True)

                with patch.object(native.tempfile, "mkdtemp", side_effect=directory), \
                     patch.object(native.subprocess, "run", side_effect=child), \
                     patch.object(native, "check", return_value=dict(samples=[], frames=64)), \
                     patch.object(native, "replay", side_effect=replay):
                    if fail:
                        with self.assertRaisesRegex(RuntimeError, "failure after handoff"):
                            native.night(Path("/usr/bin/true"))
                    else:
                        native.night(Path("/usr/bin/true"))
                self.assertFalse(fixture.root.exists())
                self.assertEqual(len(handed), 1)
                self.assertFalse(handed[0].exists(), "caller leaked the transferred recording")


if __name__ == "__main__":
    unittest.main(verbosity=2)
