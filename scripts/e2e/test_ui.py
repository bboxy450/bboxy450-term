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

    def test_budget_and_page_observer_cannot_pass_unobserved_or_oversized_values(self):
        good={"available":True,"complete":True,"omitted_panes":0,"retained_history_bytes":150,
              "panes":[{"retained_history_bytes":100,"budget_bytes":4_000_000,
                        "pages_resident":1,"pages_fetched":2,"pages_evicted":1}]}
        class Metrics:
            def ctl(self,*args,**kw): return {"ui":{"available":True,"history":good}}
        ui.history_sample(Metrics(),"good")
        for field,value in [("retained_history_bytes",None),("retained_history_bytes",4_000_001),("pages_resident",2),("pages_evicted",3)]:
            old=good["panes"][0][field]
            good["panes"][0][field]=value
            with self.assertRaises(AssertionError): ui.history_sample(Metrics(),"bad")
            good["panes"][0][field]=old

    def test_every_script_header_matches_its_map_row(self):
        mapping = (ROOT.parent.parent / ".bboxy/specs/e2e-script-map.md").read_text()
        for name in ["S1", "S2", "S3", "S4", "K2", "L1"]:
            header = (ROOT / f"{name}.sh").read_text().splitlines()[0]
            row_prefix = f"| {name} | `scripts/e2e/{name}.sh` | "
            rows = [line for line in mapping.splitlines() if line.startswith(row_prefix)]
            self.assertEqual(len(rows), 1, f"{name} needs exactly one coverage map row")
            coverage = rows[0][len(row_prefix):].split(" | ", 1)[0]
            self.assertIn(coverage, ("full", "partial"), name)
            if coverage == "full":
                self.assertEqual(header, "# coverage: full", name)
            else:
                self.assertTrue(header.startswith("# coverage: partial; missing: "), name)
                missing = header.split("missing: ", 1)[1]
                self.assertTrue(missing, name)
                self.assertIn(row_prefix + f"partial | {missing} |", mapping)


if __name__ == "__main__":
    unittest.main()
