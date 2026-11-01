#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Remove one d5002 guard, require its named failure, then restore and pass."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]


def once(text, before, after):
    if text.count(before) != 1:
        raise ValueError("control anchor drift: " + before)
    return text.replace(before, after)


def broad_fixtures(text):
    a, b = text.index("def landing_machinery("), text.index("def load_average(")
    return text[:a] + 'def landing_machinery(paths):\n    return any(p.startswith("scripts/") for p in paths)\n\n\n' + text[b:]


def main():
    out = ROOT / "target/landing-budget-controls"
    out.mkdir(parents=True, exist_ok=True)
    cases = [
        ("R0", "landing.py", "test-landing.py", "BudgetClockTests.test_missing_substep_is_unclassified_and_advisory",
         "missing phase was silently classified", lambda s: once(s,
          'active = stage if stage != "formal" or substep else None', 'active = stage')),
        ("R1", "invariants.sh", "test-landing.py", "BudgetPresubmitTests.test_observed_ctl_runs_once",
         "ctl binary ran twice", lambda s: once(s, 'return results == ["ok"]', 'return False')),
        ("R1-filtered", "invariants.sh", "test-landing.py", "BudgetPresubmitTests.test_filtered_invariant_still_runs",
         "filtered invariant was skipped", lambda s: once(s, 'return results == ["ok"]', 'return True')),
        ("R1-ignored", "landing.py", "test-landing.py", "BudgetPresubmitTests.test_ignored_invariant_still_runs",
         "ignored invariant was skipped", lambda s: once(s,
          '"--covered-tests", str(coverage_path)]', '"--covered-tests", str(coverage_path), "--no-cargo"]')),
        ("R2", "landing.py", "test-landing.py", "BudgetPresubmitTests.test_harness_helper_cut_skips_landing_fixtures",
         "unrelated script ran landing fixtures", broad_fixtures),
        ("R3", "formal.py", "test-formal.py", "ModelReuseTests.test_changed_tla_rechecks_only_the_changed_model",
         "changed TLA reused stale evidence", lambda s: once(s,
          'suffixes = (".tla", ".cfg", "Graph.cfg", ".states", ".edges")',
          'suffixes = (".cfg", "Graph.cfg", ".states", ".edges")')),
        ("R3-postsubmit", "formal.py", "test-formal.py", "ModelReuseTests.test_postsubmit_rechecks_and_records_no_reuse",
         "postsubmit reused a verdict", lambda s: once(s, 'if reuse else None', 'if True else None')),
        ("R4", "landing.py", "test-landing.py", "BudgetPresubmitTests.test_allocation_overrun_is_advisory",
         "stage advisory blocked presubmit", lambda s: once(s,
          'record["findings"] = budget_findings(record, load_start, record["load_average_end"])',
          'record["findings"] = budget_findings(record, load_start, record["load_average_end"])\n'
          '    if record["findings"]:\n        code = 124\n        record.update(exit=124, verdict="red")')),
        ("R4-total", "landing.py", "test-landing.py", "BudgetClockTests.test_stage_allocation_is_advisory_but_900_seconds_still_blocks",
         "900 second run bound was bypassed", lambda s: once(s,
          'if totals["run_seconds"] >= run_bound:', 'if False and totals["run_seconds"] >= run_bound:')),
    ]
    records = []
    for name, source, suite, test, assertion, mutate in cases:
        file = ROOT / "scripts" / source
        original = file.read_text()
        command = [sys.executable, "-I", str(ROOT / "scripts" / suite), test]
        row = {"rule": name, "source": str(file.relative_to(ROOT)),
               "source_sha256": hashlib.sha256(original.encode()).hexdigest(),
               "command": command, "assertion": assertion}
        try:
            file.write_text(mutate(original))
            result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=90)
            (out / (name + "-red.log")).write_text(result.stdout + result.stderr)
            row["red_exit"] = result.returncode
            if result.returncode != 1 or assertion not in result.stdout + result.stderr:
                raise ValueError("control did not reach its named assertion: " + name)
        finally:
            file.write_text(original)
        result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=90)
        (out / (name + "-green.log")).write_text(result.stdout + result.stderr)
        row["green_exit"] = result.returncode
        if result.returncode:
            raise ValueError("restored control failed: " + name)
        records.append(row)
        (out / "summary.json").write_text(json.dumps(records, indent=2) + "\n")
        print(f"{name}: {row['red_exit']} -> {row['green_exit']}", flush=True)


if __name__ == "__main__":
    main()
