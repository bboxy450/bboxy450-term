#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Measure the real release CLI writing a known byte count to /dev/null."""
from pathlib import Path
import json
import subprocess
import time

ROOT = Path(__file__).resolve().parents[3]
BINARY = ROOT / "target/release/bboxy450-term-firehose"
LOOPS = 200_000_000
cycle = subprocess.check_output([str(BINARY), "--seed", "42", "--rate", "max"])
with open("/dev/null", "wb", buffering=0) as sink:
    start = time.perf_counter()
    result = subprocess.run(
        [str(BINARY), "--seed", "42", "--rate", "max", "--loop", str(LOOPS)],
        stdout=sink, check=False, timeout=30,
    )
    elapsed = time.perf_counter() - start
total = len(cycle) * LOOPS
mib_per_second = total / (1024 * 1024) / elapsed
print(json.dumps({
    "command": [str(BINARY), "--seed", "42", "--rate", "max", "--loop", str(LOOPS)],
    "stdout": "/dev/null", "exit_status": result.returncode,
    "cycle_bytes": len(cycle), "cycles": LOOPS, "bytes": total,
    "seconds": elapsed, "MiB_per_second": mib_per_second,
    "minimum_MiB_per_second": 200,
}, sort_keys=True))
assert result.returncode == 0, result.returncode
assert mib_per_second >= 200, mib_per_second
