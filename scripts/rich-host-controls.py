#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Causal process controls. No source mutation, credentials or operator input."""
import argparse
import json
import os
from pathlib import Path
import subprocess
p = argparse.ArgumentParser()
p.add_argument('--target', type=Path, required=True)
a = p.parse_args()
output = a.target / 'rich-controls'
output.mkdir(parents=True, exist_ok=True)
base = ['cargo', 'test', '--locked', '--target-dir', str(a.target)]
controls = [
    ('fresh-id', base + ['-p', 'bboxy450-harness-wake-up', '--test', 'rich_socket', 'private_socket_duplicate_busy_closed_reconnect_and_generation', '--', '--exact', '--nocapture'], 'duplicate control must produce two turns'),
    ('pty-fallback', base + ['-p', 'bboxy450-term-mux', '--test', 'it', 'mcp::mcp_harness_seat_completes_and_closes_without_orphans', '--', '--exact', '--ignored', '--nocapture'], 'zero bytes for socket delivery'),
]
results = []
for control, command, assertion in controls:
    env = {**os.environ, 'BBOXY_RICH_TEST_CONTROL': control,
           'BBOXY_HARNESS_TEST_BIN': str(a.target / 'debug/bboxy450-harness')}
    run = subprocess.run(command, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=120)
    (output / (control + '.log')).write_text(run.stdout)
    if run.returncode != 101 or assertion not in run.stdout or 'assertion' not in run.stdout:
        raise SystemExit(f'{control}: expected causal assertion exit 101, got {run.returncode}:\n{run.stdout[-8000:]}')
    results.append({'control': control, 'command': command, 'exit_code': run.returncode, 'assertion': assertion})
    print(f'rich-control: {control} exit=101; causal assertion observed', flush=True)
(output / 'report.json').write_text(json.dumps(results, indent=2) + '\n')
