"""Falsification checks for the night assertions; no app or OS input is injected."""
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("ui", ROOT / "ui.py")
ui = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ui)


class Pages:
    """A protocol witness with a deliberately short final viewport overlap."""
    def __init__(self, work, fault=None):
        self.work, self.fault, self.sequence = work, fault, 0

    def spawn(self, _name):
        return "seat"

    def fixture(self, _seat, count):
        assert count == 20_000

    def ctl(self, *args, **_kwargs):
        assert args[:2] == ("ui", "scroll")
        self.sequence += 1
        at = 0 if args[3] == "top" else min(int(args[4]), 19_994) if args[3] == "row" else 19_994
        if self.fault == "skip" and at > 0:
            at += 1
        return {"frame_seq": 1 if self.fault == "stale" else self.sequence,
                "panes": [{"seat": "seat", "rows": 13,
                           "scroll": {"top_row": at, "live": at == 19_994}}]}

    def capture(self, *_args):
        pass

    def line(self, *_args):
        pass

    def wait(self, *_args):
        pass


class Assertions(unittest.TestCase):
    def test_coordinate_intervals_cover_exactly_once_including_tail_overlap(self):
        with tempfile.TemporaryDirectory() as d, contextlib.redirect_stdout(io.StringIO()):
            ui.s1(Pages(Path(d)))
            intervals = [json.loads(x) for x in (Path(d) / "s1-traversal.jsonl").read_text().splitlines()]
            actual = [row for page in intervals for row in range(page["from"], page["to_exclusive"])]
            self.assertEqual(actual, list(range(20_000)))

    def test_skipped_page_cannot_pass(self):
        with tempfile.TemporaryDirectory() as d:
            with self.assertRaisesRegex(AssertionError, "skipped"):
                ui.s1(Pages(Path(d), "skip"))

    def test_stale_frame_cannot_pass(self):
        with tempfile.TemporaryDirectory() as d:
            with self.assertRaisesRegex(AssertionError, "older produced frame"):
                ui.s1(Pages(Path(d), "stale"))

    def test_missing_observers_fail_instead_of_passing_no_assertions(self):
        for name, reason in [("S2", "fixture_unavailable"), ("S3", "metrics_unavailable")]:
            result = subprocess.run(["sh", str(ROOT / f"{name}.sh")],
                                    capture_output=True, text=True, stdin=subprocess.DEVNULL)
            self.assertEqual(result.returncode, 1)
            self.assertIn(reason, result.stderr)

    def test_every_script_header_matches_its_map_row(self):
        mapping = (ROOT.parent.parent / ".bboxy/specs/e2e-script-map.md").read_text()
        for name in ["S1", "S2", "S3", "S4", "K2", "L1"]:
            header = (ROOT / f"{name}.sh").read_text().splitlines()[0]
            self.assertTrue(header.startswith("# coverage: partial; missing: "))
            missing = header.split("missing: ", 1)[1]
            self.assertIn(f"| {name} | `scripts/e2e/{name}.sh` | partial | {missing} |", mapping)


if __name__ == "__main__":
    unittest.main()
