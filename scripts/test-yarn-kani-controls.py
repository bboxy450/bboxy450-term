#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
# Inspired by Kani's structured verification results; independently written controls.
# https://github.com/model-checking/kani/tree/kani-0.68.0/tests/json-handler
"""Refuse false green, vacuity and unrelated failures in the Kani proof runner."""

import contextlib
import copy
import importlib.util
import io
import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("yarn_kani", Path(__file__).with_name("test-yarn-kani.py"))
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


class VerdictControls(unittest.TestCase):
    def record(self, broken=False):
        name = runner.PROOFS[-1] if broken else runner.PROOFS[0]
        return {
            "verification_results": {
                "summary": {"status": "completed", "total_harnesses": 1, "executed": 1,
                            "successful": int(not broken), "failed": int(broken)},
                "results": [{"harness_id": "kani::proofs::" + name,
                             "status": "Failure" if broken else "Success",
                             "checks": [{"status": "Failure" if broken else "Success",
                                         "category": "assertion",
                                         "description": "RESULT-001 broken inductive step"}]}],
            }
        }, name, int(broken)

    def validate(self, data, name, code):
        with tempfile.TemporaryDirectory(prefix="yarn-kani-control-") as own:
            result = Path(own) / "result.json"
            result.write_text(json.dumps(data))
            with contextlib.redirect_stdout(io.StringIO()):
                runner.validate(result, name, code)

    def test_completed_positive_and_exact_negative(self):
        self.validate(*self.record())
        self.validate(*self.record(broken=True))

    def test_unwinding_unsupported_solver_and_wrong_assertion_are_not_controls(self):
        original, name, code = self.record(broken=True)
        for category in ("unwind", "unsupported", "solver", "assertion"):
            data = copy.deepcopy(original)
            check = data["verification_results"]["results"][0]["checks"][0]
            check.update(category=category, description="another error")
            with self.subTest(category=category), self.assertRaises(ValueError):
                self.validate(data, name, code)

    def test_zero_harnesses_and_incomplete_runs_are_not_proofs(self):
        original, name, code = self.record()
        for change in ({"executed": 0}, {"total_harnesses": 0}, {"status": "timeout"}):
            data = copy.deepcopy(original)
            data["verification_results"]["summary"].update(change)
            with self.subTest(change=change), self.assertRaises(ValueError):
                self.validate(data, name, code)

    def test_exit_and_unproved_checks_cannot_be_hidden(self):
        data, name, _ = self.record()
        with self.assertRaises(ValueError):
            self.validate(data, name, 1)
        data["verification_results"]["results"][0]["checks"][0]["status"] = "Undetermined"
        with self.assertRaises(ValueError):
            self.validate(data, name, 0)
        data, name, _ = self.record(broken=True)
        with self.assertRaises(ValueError):
            self.validate(data, name, 0)


if __name__ == "__main__":
    unittest.main()
