#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Remove bootstrap isolation in this worktree, require failure, restore and verify."""
import os
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parent.parent
source = root / "crates/bboxy450-term/src/main.rs"
original = source.read_bytes()
guard = b'''    if !hook_args.first().is_some_and(|arg| matches!(arg.as_str(), "supervisor" | "logic" | "desktop"))
        && let Some(root) = &diagnostics_root'''
if original.count(guard) != 1:
    raise SystemExit("bootstrap guard changed; update the control before running it")
command = ["make", "test-error-journal-bootstrap", "TARGET_DIR=" + os.environ["JOURNAL_CONTROL_TARGET"]]
try:
    source.write_bytes(original.replace(guard, b"    if let Some(root) = &diagnostics_root", 1))
    result = subprocess.run(command, cwd=root, capture_output=True, text=True, timeout=600)
finally:
    source.write_bytes(original)
if result.returncode == 0 or "journal did not converge" not in result.stdout + result.stderr:
    print(result.stdout, end="")
    print(result.stderr, end="")
    raise SystemExit("broken bootstrap did not fail the private-journal assertion")
print(f"broken bootstrap: make exit {result.returncode}; private-journal assertion failed as required", flush=True)
subprocess.run(command, cwd=root, check=True, timeout=600)
print("restored bootstrap: exit 0", flush=True)
