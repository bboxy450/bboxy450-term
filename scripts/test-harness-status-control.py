#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Restore the actual cadence bug, require the native witness to reject it, restore."""
import os
from pathlib import Path
import subprocess
import sys

root = Path(__file__).resolve().parent.parent
target = Path(sys.argv[1]).resolve()
source = root / 'crates/bboxy450-harness-wake-up/src/observe.rs'
original = source.read_bytes()
mutant = original
for old, new in [
    (b'if self.reported_ms.is_some() && obs.grade < Grade::Reported {',
     b'if false && self.reported_ms.is_some() && obs.grade < Grade::Reported {'),
    (b'if let Some(reported_ms) = self.reported_ms {',
     b'if false && let Some(reported_ms) = self.reported_ms {'),
]:
    assert mutant.count(old) == 1, 'cadence control source changed; update its exact guard'
    mutant = mutant.replace(old, new)
command = ['cargo', 'test', '--locked', '-p', 'bboxy450-term-mux', '--test', 'it',
           '--target-dir', str(target), 'mcp::mcp_harness_seat_status_survives_quiet_tool',
           '--', '--exact', '--ignored', '--nocapture']
env = {**os.environ, 'BBOXY_HARNESS_TEST_BIN': str(target / 'debug/bboxy450-harness')}
target.mkdir(parents=True, exist_ok=True)

def run(label):
    path = target / f'harness-status-control-{label}.log'
    with path.open('w') as log:
        result = subprocess.run(command, cwd=root, env=env, stdout=log,
                                stderr=subprocess.STDOUT, timeout=300)
    print(f'harness-status-control: {label} exit={result.returncode} log={path}', flush=True)
    return result.returncode, path.read_text()

try:
    source.write_bytes(mutant)
    code, output = run('mutant')
    # Cadence can first erase the reported grade/detail while still "active",
    # before its quiet-output timer switches to idle. Either is the exact bug.
    assert code == 101 and 'panicked at' in output and \
        '"source":"cadence"' in output and '"grade":"inferred"' in output, output[-12000:]
finally:
    source.write_bytes(original)
assert source.read_bytes() == original
code, output = run('restored')
assert code == 0, output[-12000:]
print('harness-status-control: cadence override rejected; restored witness passed', flush=True)
