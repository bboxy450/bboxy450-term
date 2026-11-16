#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""scripts/kb_record.py reads a host-written Den record as the host does."""
import importlib.util
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("kb_record", HERE / "kb_record.py")
kb = importlib.util.module_from_spec(spec)
spec.loader.exec_module(kb)

# The Den source the host rendered for a task in
# crates/bbt-mux/src/knowledge/roots_tests.rs (den_record_lowers_...), plus a
# nested block and prose lines that must not read as fields.
HOST_TASK = r'''module tasks.lowered

task Lowered {
  title "Lower one task"
  verify $ make check ~> exit 0
  paths "src/a.rs"
  staff {
    status = "not a field"
  }
  brief """
    Intent: cross the boundary as Markdown.
    status Bogus
    """

  managed {
    authorship = json"{\"kind\":\"seat\",\"signer\":\"seat:00000000000000000000000019\"}"
    id = json"\"0000000000000000000000044J\""
    number = json"7"
    status = Open
    other = json"{\"bad-key\":true}"
    body_tail = json"\"\\n\""
  }
}
'''


class KbRecordTests(unittest.TestCase):
    def test_host_den_task_fields(self):
        fields = kb.den_fields(HOST_TASK)
        self.assertEqual(fields["type"], "task")
        self.assertEqual(fields["title"], "Lower one task")
        self.assertEqual(fields["verify"], "make check")
        self.assertEqual(fields["id"], "0000000000000000000000044J")
        self.assertEqual(fields["number"], "7")
        self.assertEqual(fields["status"], "open")
        self.assertEqual(fields["bad-key"], "true")
        self.assertNotIn("body_tail", fields)

    def test_authored_status_and_markdown_and_machine_spec(self):
        self.assertEqual(kb.den_fields('module a.b\n\ntask B {\n  status Blocked on CI\n}\n')["status"], "blocked")
        self.assertEqual(kb.markdown_fields('---\nstatus: "done"\nnumber: 3\n---\nbody\n'),
                         {"status": "done", "number": "3"})
        self.assertEqual(kb.den_fields("module s.M\n\nspec M {\n  state x : Bool = false\n}\n"), {})

    def test_cli_and_records(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "a.den").write_text(HOST_TASK)
            (root / "b.md").write_text("---\nnumber: 9\n---\n")
            (root / "c.txt").write_text("")
            self.assertEqual([p.name for p in kb.records(root)], ["a.den", "b.md"])
            out = subprocess.run([sys.executable, "-I", str(HERE / "kb_record.py"), "number",
                                  str(root / "a.den"), str(root / "b.md")],
                                 capture_output=True, text=True, check=True).stdout
            self.assertEqual(out, "7\n9\n")


if __name__ == "__main__":
    unittest.main()
