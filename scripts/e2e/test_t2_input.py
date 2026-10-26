"""Falsify the echo assertion independently of any ctl transport."""
import copy
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parent))
from t2_input import assert_echo


class EchoFrames(unittest.TestCase):
    def test_late_catch_up_missing_text_and_a_different_generation_cannot_pass(self):
        view = {"seat": "seat", "generation": 2, "view_generation": "view"}
        source = {
            "view": view, "sequence": 8, "lines": ["T2_ECHO actual"],
            "ingested": {"view": view, "sequence": 8, "after_frame_seq": 16,
                         "first_frame_seq": 17, "first_drawn_view": view,
                         "first_drawn_sequence": 8},
        }
        layout = {"focused": "seat", "panes": [
            {"seat": "seat", "focused": True, "rendered": source}]}
        echo = {"sequence": 7, "generation": 2}
        assert_echo(layout, "seat", echo, "actual")
        for key, value in (("first_drawn_sequence", 6), ("first_frame_seq", 18),
                           ("first_drawn_view", dict(view, generation=1))):
            bad = copy.deepcopy(layout)
            bad["panes"][0]["rendered"]["ingested"][key] = value
            with self.subTest(key=key), self.assertRaises(AssertionError):
                assert_echo(bad, "seat", echo, "actual")
        for lines in (None, [], ["T2_ECHO different"]):
            bad = copy.deepcopy(layout)
            bad["panes"][0]["rendered"]["lines"] = lines
            with self.subTest(lines=lines), self.assertRaises(AssertionError):
                assert_echo(bad, "seat", echo, "actual")


if __name__ == "__main__":
    unittest.main()
