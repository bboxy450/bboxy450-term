#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Unattended broken controls for contract weakening and structured evidence."""
import copy
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tomllib
from types import SimpleNamespace
import unittest
from unittest import mock

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("protocol_contracts", ROOT / "scripts/check-protocol-contracts.py")
CHECK = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECK)
BASELINE = ROOT / "verification/protocol-contracts.toml"


class ContractControls(unittest.TestCase):
    def setUp(self):
        self.baseline = tomllib.loads(BASELINE.read_text())["contracts"]
        self.original = copy.deepcopy(self.baseline[0])

    def test_dropped_result_fixture_is_rejected_by_the_real_command(self):
        result = subprocess.run([
            sys.executable, "-I", str(ROOT / "scripts/check-protocol-contracts.py"),
            "--baseline", str(BASELINE), "--current",
            str(ROOT / "verification/fixtures/protocol-contracts-dropped-result.json"),
        ], cwd=ROOT, text=True, capture_output=True, check=False)
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn("required_ids removed: ['RESULT-001']", result.stderr)
        self.assertIn("--- baseline", result.stdout)
        self.assertIn("+++ current contract", result.stdout)

    def test_each_narrowing_and_added_assumption_fails(self):
        mutations = [
            ("required_ids", []),
            ("require_reachable", []),
            ("domains", {"Id": ["A"]}),
            ("bounds", {**self.original["bounds"], "desired": {"min": 0, "max": 1}}),
            ("state_limit", 10),
            ("assumptions", self.original["assumptions"] + ["dependency never fails"]),
            ("fairness_assumed", ["weak(Complete(A)::current)"]),
            ("waivers", ["skip RESULT-001"]),
        ]
        for field, value in mutations:
            with self.subTest(field=field):
                current = {**self.original, field: value}
                self.assertTrue(CHECK.compare(self.baseline, [current]))
                # Changing the relevant baseline is explicit review, never an
                # unrelated textual amendment that bypasses the comparison.
                self.assertEqual(CHECK.compare([current], [current]), [])

    def test_growth_and_fewer_assumptions_are_not_weakening(self):
        current = copy.deepcopy(self.original)
        current["domains"]["Id"].append("C")
        current["bounds"]["desired"]["max"] = 2
        current["state_limit"] += 1
        current["assumptions"] = []
        self.assertEqual(CHECK.compare(self.baseline, [current]), [])
        self.assertIn("contract removed", CHECK.compare(self.baseline, [])[0])
        with self.assertRaises(ValueError):
            CHECK.indexed([current, current])

    def test_retroactive_baseline_only_amendment_fails_same_commit_rule(self):
        original_text = BASELINE.read_text()
        previous_text = original_text.replace('required_ids = ["RESULT-001"]', 'required_ids = ["RESULT-001", "RESULT-002"]')

        def run(_root, *args):
            if args[0] == "rev-parse":
                return SimpleNamespace(returncode=0, stdout=str(ROOT) + "\n")
            if args[0] == "show" and args[1].startswith("HEAD:"):
                return SimpleNamespace(returncode=0, stdout=original_text)
            if args[0] == "log":
                return SimpleNamespace(returncode=0, stdout="fixture-amendment\n")
            if args[0] == "show":
                return SimpleNamespace(returncode=0, stdout=previous_text)
            return SimpleNamespace(returncode=0, stdout="verification/protocol-contracts.toml\n")

        with mock.patch.object(CHECK, "git", side_effect=run):
            errors = CHECK.amendment_errors(ROOT, BASELINE, self.baseline)
        self.assertIn("same commit", errors[0])

        def together(root, *args):
            result = run(root, *args)
            if args[0] == "diff-tree":
                result.stdout += self.original["source"] + "\n"
            return result

        with mock.patch.object(CHECK, "git", side_effect=together):
            self.assertEqual(CHECK.amendment_errors(ROOT, BASELINE, self.baseline), [])

    def test_missing_git_history_cannot_authorize_an_amendment(self):
        with mock.patch.object(CHECK, "git", return_value=SimpleNamespace(returncode=128, stdout="")):
            with self.assertRaisesRegex(ValueError, "Git history"):
                CHECK.amendment_errors(ROOT, BASELINE, self.baseline)

    def test_golden_retains_independent_result_axes(self):
        result = json.loads((ROOT / "verification/golden/latest-result.json").read_text())
        self.assertEqual(result["outcome"], "established")
        self.assertEqual(result["method"], "exhaustive_finite_model")
        self.assertEqual(result["implementation"], "generated_core")
        self.assertEqual((result["states"], result["transitions"]), (21, 105))
        self.assertEqual(result["scope"]["fairness_assumed"], [])
        self.assertTrue(result["provenance"]["source_hash"].startswith("fnv1a64:"))
        self.assertEqual(result["counterexample"], None)


if __name__ == "__main__":
    unittest.main()
