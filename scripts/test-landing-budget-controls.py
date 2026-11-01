#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Remove one d5002 guard, require its named failure, then restore and pass."""
import hashlib
import argparse
import json
from pathlib import Path
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def once(text, before, after):
    if text.count(before) != 1:
        raise ValueError("control anchor drift: " + before)
    return text.replace(before, after)


def broad_fixtures(text):
    a, b = text.index("def landing_machinery("), text.index("def load_average(")
    return text[:a] + 'def landing_machinery(paths, root=None):\n    return any(p.startswith("scripts/") for p in paths)\n\n\n' + text[b:]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rule", action="append", default=[])
    parser.add_argument("--out", type=Path)
    args = parser.parse_args()
    out = (args.out or ROOT / "target/landing-budget-controls").resolve()
    out.mkdir(parents=True, exist_ok=True)
    cases = [
        ("R0", "landing.py", "test-landing.py", "BudgetClockTests.test_missing_substep_is_unclassified_and_advisory",
         "missing phase was silently classified", lambda s: once(s,
          'active = stage if stage != "formal" or substep else None', 'active = stage')),
        ("R1", "invariants.sh", "test-landing.py", "BudgetPresubmitTests.test_observed_ctl_runs_once",
         "ctl binary ran twice", lambda s: once(s, 'return results == ["ok"]', 'return False')),
        ("R1-filtered", "invariants.sh", "test-landing.py", "BudgetPresubmitTests.test_filtered_invariant_still_runs",
         "filtered invariant was skipped", lambda s: once(s, 'return results == ["ok"]', 'return results in (["ok"], [])')),
        ("R1-ignored", "invariants.sh", "test-landing.py", "BudgetPresubmitTests.test_ignored_invariant_still_runs",
         "ignored result suppressed the invariant", lambda s: once(s,
          'return results == ["ok"]', 'return results in (["ok"], ["ignored"])')),
        ("R2", "landing.py", "test-landing.py", "BudgetPresubmitTests.test_brand_cut_skips_landing_fixtures",
         "unrelated script ran landing fixtures", broad_fixtures),
        ("R2-copy-tuple", "landing.py", "test-landing.py", "BudgetPresubmitTests.test_added_fixture_copy_automatically_triggers",
         "added fixture copy did not trigger", lambda s: once(s, 'names.update(fixture_copy_scripts(root))', 'None  # deliberately ignore fixture tuple')),
        ("R3", "formal.py", "test-formal.py", "ModelReuseTests.test_changed_tla_rechecks_only_the_changed_model",
         "changed TLA reused stale evidence", lambda s: once(s,
          'suffixes = (".tla", ".cfg", "Graph.cfg", ".states", ".edges")',
          'suffixes = (".cfg", "Graph.cfg", ".states", ".edges")')),
        ("R3-postsubmit", "formal.py", "test-formal.py", "ModelReuseTests.test_postsubmit_rechecks_and_records_no_reuse",
         "postsubmit reused a verdict", lambda s: once(s, 'if reuse else None', 'if True else None')),
        ("R3-checker", "formal.py", "test-formal.py", "ModelReuseTests.test_changed_checker_harness_rechecks_every_model",
         "changed checker harness reused evidence", lambda s: once(s,
          'paths = ("scripts/formal.py", "scripts/test-formal.py", "crates/bboxy450-tlc", "crates/bbt-yarn")',
          'paths = ("scripts/test-formal.py", "scripts/test-formal.py", "crates/bboxy450-tlc", "crates/bbt-yarn")')),
        ("CI-controls", "ci-local.sh", "test-landing.py", "GateReceiptTests.test_real_ci_runs_budget_controls_and_propagates_failure",
         "budget controls did not block full CI", lambda s: once(s,
          '    ledger integration python3 -I scripts/test-landing-budget-controls.py --out "$BBOXY_CARGO_TARGET_DIR/landing-budget-controls"\n', '')),
        ("R4", "landing.py", "test-landing.py", "BudgetPresubmitTests.test_allocation_overrun_is_advisory",
         "stage advisory blocked presubmit", lambda s: once(s,
          'record["findings"] = budget_findings(record, load_start, record["load_average_end"])',
          'record["findings"] = budget_findings(record, load_start, record["load_average_end"])\n'
          '    if record["findings"]:\n        code = 124\n        record.update(exit=124, verdict="red")')),
        ("R4-total", "landing.py", "test-landing.py", "BudgetClockTests.test_stage_allocation_is_advisory_but_900_seconds_still_blocks",
         "900 second run bound was bypassed", lambda s: once(s,
          'if totals["run_seconds"] >= run_bound:', 'if False and totals["run_seconds"] >= run_bound:')),
    ]
    cases += [
        ("R5-exact-only", "landing.py", "test-landing.py", "LandTests.test_records_rebase_accepts_gate_and_runs_only_records_checks",
         "records-only gate was refused", lambda s: once(s, 'delta = records_gate_delta(root, source, sha)',
          'raise Refusal("gate_log_source_mismatch: deliberately exact-only")')),
        ("R5-outside", "landing.py", "test-landing.py", "LandTests.test_records_gate_refuses_a_byte_outside_records",
         "non-records delta was accepted", lambda s: once(s, 'if different.returncode:', 'if False:')),
        ("R5-named", "landing.py", "test-landing.py", "LandTests.test_records_gate_refuses_a_record_named_by_code",
         "code-named record was accepted", lambda s: once(s, 'if named.returncode != 1:', 'if False:')),
        ("R5-checks", "landing.py", "test-landing.py", "LandTests.test_records_gate_refuses_failed_records_checks",
         "failed records checks were accepted", lambda s: once(s, 'records_checks(check_root, delta, env=check_environment)', 'None  # deliberately omit records checks')),
        ("R5-snapshot", "landing.py", "test-landing.py", "LandTests.test_records_rebase_accepts_gate_and_runs_only_records_checks",
         "records-only gate was refused", lambda s: once(s, '            check_root, check_environment = candidate_environment(root, sha)',
          '            check_root, check_environment = root, None')),
        ("R5-secrets", "landing.py", "test-landing.py", "LandTests.test_records_gate_refuses_secret_like_changed_record",
         "secret-like record was accepted", lambda s: once(s, 'if name in tracked and secret.search(', 'if False and secret.search(')),
    ]
    if args.rule:
        unknown = set(args.rule) - {case[0] for case in cases}
        if unknown:
            parser.error("unknown rule: " + ", ".join(sorted(unknown)))
        cases = [case for case in cases if case[0] in args.rule]
    # Mutations never touch the author's checkout or a parent CI shell's open
    # script. Every subprocess sees this disposable snapshot of the signed HEAD.
    with tempfile.TemporaryDirectory(prefix="control-checkout-", dir=out) as directory:
        checkout = Path(directory) / "tree"
        subprocess.run(["git", "worktree", "add", "--detach", "-q", str(checkout), "HEAD"], cwd=ROOT, check=True)
        try:
            run_controls(checkout, out, cases)
        finally:
            subprocess.run(["git", "worktree", "remove", "--force", str(checkout)], cwd=ROOT, check=True)


def run_controls(root, out, cases):
    records = []
    for name, source, suite, test, assertion, mutate in cases:
        file = root / "scripts" / source
        original = file.read_text()
        command = [sys.executable, "-I", str(root / "scripts" / suite), test]
        row = {"rule": name, "source": str(file.relative_to(root)),
               "source_sha256": hashlib.sha256(original.encode()).hexdigest(),
               "command": command, "assertion": assertion}
        try:
            file.write_text(mutate(original))
            result = subprocess.run(command, cwd=root, capture_output=True, text=True, timeout=90)
            (out / (name + "-red.log")).write_text(result.stdout + result.stderr)
            row["red_exit"] = result.returncode
            if result.returncode != 1 or assertion not in result.stdout + result.stderr:
                raise ValueError("control did not reach its named assertion: " + name)
        finally:
            file.write_text(original)
        result = subprocess.run(command, cwd=root, capture_output=True, text=True, timeout=90)
        (out / (name + "-green.log")).write_text(result.stdout + result.stderr)
        row["green_exit"] = result.returncode
        if result.returncode:
            raise ValueError("restored control failed: " + name)
        records.append(row)
        (out / "summary.json").write_text(json.dumps(records, indent=2) + "\n")
        print(f"{name}: {row['red_exit']} -> {row['green_exit']}", flush=True)


if __name__ == "__main__":
    main()
