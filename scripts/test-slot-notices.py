"""Slot events use a private stub messenger; no daemon, account or real slot."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("landing", Path(__file__).with_name("landing.py"))
landing = importlib.util.module_from_spec(spec)
spec.loader.exec_module(landing)


class SlotNotices(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory(prefix="slot-notice-")
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name).resolve()
        self.log = self.root / "notices.jsonl"
        binary = self.root / "bin"
        binary.mkdir()
        icq = binary / "icq"
        icq.write_text(f"#!{sys.executable}\n" + '''import json,os,pathlib,sys
with pathlib.Path(os.environ["SLOT_NOTICE_LOG"]).open("a") as stream:
 stream.write(json.dumps({"args":sys.argv[1:],"claimed":pathlib.Path(os.environ["BBOXY_GATE_SLOT_ROOT"]).exists()})+"\\n")
raise SystemExit(int(os.environ.get("SLOT_NOTICE_FAIL","0")))
''')
        icq.chmod(0o700)
        self.sha = "a" * 40
        self.addCleanup(patch.stopall)
        patch.object(landing, "repository", return_value=self.root).start()
        patch.object(landing, "common_repository", return_value=self.root / ".git").start()
        patch.object(landing, "git", return_value=self.sha).start()
        patch.dict(os.environ, {"PATH": str(binary), "SLOT_NOTICE_LOG": str(self.log),
                                "BBOXY_LANDING_STATE_DIR": str(self.root / "state")}).start()

    def events(self, exit_code):
        rows = [json.loads(line) for line in self.log.read_text().splitlines()]
        self.assertEqual([r["args"] for r in rows], [
            ["send", "#bboxy450-term", f"SLOT 1 TAKEN {os.getpid()} {self.sha}"],
            ["send", "#bboxy450-term", f"SLOT 1 RELEASED {os.getpid()} {exit_code}"],
        ])
        self.assertEqual([r["claimed"] for r in rows], [True, False])

    # keeps: landing §4; the lease, not a model's report, emits each event
    def test_success_posts_once_at_claim_and_release_for_both_drivers(self):
        for label in ("land", "ci-local"):
            self.log.unlink(missing_ok=True)
            with landing.Slot(label, preferred=1):
                pass
            self.events(0)

    # keeps: landing §4; preserve command and refusal exit codes
    def test_return_and_exception_codes_are_the_actual_release_verdict(self):
        with landing.Slot(preferred=1) as slot:
            self.assertEqual(slot.result(17), 17)
        self.events(17)
        self.log.unlink()
        with self.assertRaises(landing.Refusal):
            with landing.Slot(preferred=1):
                raise landing.Refusal("fixture", 23)
        self.events(23)

    # keeps: landing §4; communication never gates capacity
    def test_failed_missing_and_timed_out_icq_do_not_fail_a_claim(self):
        with patch.dict(os.environ, {"SLOT_NOTICE_FAIL": "9"}):
            with landing.Slot(preferred=1):
                pass
        self.events(0)
        with patch.dict(os.environ, {"PATH": ""}):
            with landing.Slot(preferred=1):
                pass
        with patch.object(landing.subprocess, "run", side_effect=subprocess.TimeoutExpired("icq", 1)) as run:
            with landing.Slot(preferred=1):
                pass
            self.assertEqual(run.call_count, 2)
        self.assertFalse((self.root / "state/slots/bboxy450-gate-slot-1").exists())


if __name__ == "__main__":
    unittest.main()
