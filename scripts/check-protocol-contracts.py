#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
# Inspired by TLA-Verus statement preservation: https://arxiv.org/abs/2610.14923
# Independent implementation; no source copied.
"""Compare generated verification configurations with the reviewed contract baseline."""
import argparse
import difflib
import json
from pathlib import Path
import subprocess
import sys
import tomllib

sys.dont_write_bytecode = True


def snapshot(result):
    scope = result["scope"]
    return {
        "name": result["contract"],
        "required_ids": result["required_ids"],
        "require_reachable": result["require_reachable"],
        "domains": {d["name"]: d["variants"] for d in scope["domains"]},
        "bounds": {b["name"]: {"min": b["min"], "max": b["max"]} for b in scope["bounds"]},
        "state_limit": scope["state_limit"],
        "assumptions": scope["assumptions"],
        "fairness_assumed": scope["fairness_assumed"],
        "waivers": scope["waivers"],
    }


def weakening(before, after):
    """Return explicit weakening reasons, independent of textual TOML changes."""
    reasons = []
    for field in ("required_ids", "require_reachable"):
        lost = set(before[field]) - set(after[field])
        if lost:
            reasons.append(f"{field} removed: {sorted(lost)}")
    for name, variants in before["domains"].items():
        lost = set(variants) - set(after["domains"].get(name, []))
        if lost:
            reasons.append(f"domain {name} shrank: removed {sorted(lost)}")
    for name, bound in before["bounds"].items():
        current = after["bounds"].get(name)
        if current is None or current["min"] > bound["min"] or current["max"] < bound["max"]:
            reasons.append(f"bound {name} shrank: {bound} -> {current}")
    if after["state_limit"] < before["state_limit"]:
        reasons.append(f"state_limit shrank: {before['state_limit']} -> {after['state_limit']}")
    for field in ("assumptions", "fairness_assumed", "waivers"):
        added = set(after[field]) - set(before[field])
        if added:
            reasons.append(f"{field} added: {sorted(added)}")
    return reasons


def indexed(rows):
    result = {}
    for row in rows:
        name = row["name"]
        if name in result:
            raise ValueError(f"duplicate contract {name}")
        result[name] = row
    return result


def compare(baseline, current):
    errors = []
    before = indexed(baseline)
    after = indexed(current)
    for name, old in before.items():
        if name not in after:
            errors.append(f"{name}: contract removed")
        else:
            errors.extend(f"{name}: {reason}" for reason in weakening(old, after[name]))
    for name in after.keys() - before.keys():
        errors.append(f"{name}: contract missing from baseline")
    return errors


def git(root, *args):
    return subprocess.run(["git", *args], cwd=root, text=True, capture_output=True, check=False)


def amendment_errors(root, baseline_path, current):
    """A weakening baseline amendment and its source change share one commit.

    Check the last baseline amendment, including a pending working-tree amendment.
    A subsequent baseline-only commit cannot retroactively approve weakened code.
    """
    repository = git(root, "rev-parse", "--show-toplevel")
    if repository.returncode or Path(repository.stdout.strip()).resolve() != root.resolve():
        raise ValueError("contract amendment checks require the repository root and Git history")
    relative = baseline_path.resolve().relative_to(root.resolve()).as_posix()
    head_baseline = git(root, "show", f"HEAD:{relative}")
    pending = head_baseline.returncode != 0 or head_baseline.stdout != baseline_path.read_text()
    if pending:
        old = head_baseline
        changed = set(git(root, "diff", "HEAD", "--name-only").stdout.splitlines())
        changed.update(git(root, "ls-files", "--others", "--exclude-standard").stdout.splitlines())
    else:
        commit = git(root, "log", "-1", "--format=%H", "--", relative).stdout.strip()
        if not commit:
            raise ValueError("baseline has no Git provenance")
        old = git(root, "show", f"{commit}^:{relative}")
        changed = set(git(root, "diff-tree", "--root", "--no-commit-id", "--name-only", "-r", commit).stdout.splitlines())
    if old.returncode:
        return []  # First introduction has no prior baseline to weaken.
    previous = indexed(tomllib.loads(old.stdout)["contracts"])
    errors = []
    for contract in current:
        name = contract["name"]
        if name in previous and weakening(previous[name], contract) and contract["source"] not in changed:
            errors.append(f"{name}: weakening baseline amendment must change {contract['source']} in the same commit")
    for removed in previous.keys() - indexed(current).keys():
        if previous[removed]["source"] not in changed:
            errors.append(f"{removed}: removing a baseline contract requires its source change in the same commit")
    return errors


def normalized_golden(result):
    # Expansion paths/line numbers depend on whether a declaration is included
    # by a test or an example. Real evidence retains the original location.
    result = json.loads(json.dumps(result))
    result["provenance"]["declaration"] = {"file": "<declaration>", "line": 0, "column": 0}
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", type=Path, default=Path("verification/protocol-contracts.toml"))
    parser.add_argument("--current", type=Path, required=True, help="fresh output from the compiled verification declaration")
    parser.add_argument("--golden", type=Path)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    args = parser.parse_args()
    try:
        baseline = tomllib.loads(args.baseline.read_text())
        if baseline["schema"] != 1:
            raise ValueError("unsupported baseline schema")
        results = json.loads(args.current.read_text())
        if isinstance(results, dict):
            results = [results]
        current = [snapshot(r) for r in results]
        errors = compare(baseline["contracts"], current)
        # A textual change alone is never a bypass: active scope is still
        # compared with the new baseline, then the amendment commit is checked.
        errors.extend(amendment_errors(args.root, args.baseline, baseline["contracts"]))
        old_text = json.dumps([{k: v for k, v in row.items() if k != "source"} for row in baseline["contracts"]], indent=2, sort_keys=True).splitlines()
        new_text = json.dumps(current, indent=2, sort_keys=True).splitlines()
        for line in difflib.unified_diff(old_text, new_text, fromfile="baseline", tofile="current contract", lineterm=""):
            print(line)
        if args.golden:
            golden = json.loads(args.golden.read_text())
            if len(results) != 1 or normalized_golden(results[0]) != golden:
                errors.append("structured result differs from the committed golden")
        for error in errors:
            print("protocol-contracts: " + error, file=sys.stderr)
        print(f"protocol-contracts: {'FAILED' if errors else 'held'} ({len(current)} contracts)")
        return int(bool(errors))
    except (KeyError, TypeError, ValueError, OSError, tomllib.TOMLDecodeError) as error:
        print(f"protocol-contracts: invalid input: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
