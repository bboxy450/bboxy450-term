#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Remove only Cancel's remote guard; restore exact source before returning."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
p = argparse.ArgumentParser()
p.add_argument('--target', type=Path, required=True)
a = p.parse_args()
root = Path(__file__).resolve().parents[1]
source = root / 'crates/bboxy450-term/src/rich_pane.rs'
original = source.read_bytes()
guard = b'Input::Enter | Input::Click(_) | Input::Cancel'
if original.count(guard) != 1:
    raise SystemExit('expected exactly one Cancel remote-admission classifier')
broken = original.replace(guard, b'Input::Enter | Input::Click(_)')
out = a.target / 'rich-cancel-control'
out.mkdir(parents=True, exist_ok=True)
command = ['cargo', 'test', '--locked', '-p', 'bboxy450-term', '--test', 'rich_client',
           '--target-dir', str(a.target), 'delayed_cancel_cannot_deny_a_new_approval',
           '--', '--exact', '--nocapture']
try:
    source.write_bytes(broken)
    result = subprocess.run(command, cwd=root, env={**os.environ, 'BBOXY_RICH_CANCEL_CASE': 'session'},
                            text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    (out / 'broken.log').write_text(result.stdout)
finally:
    if source.read_bytes() != broken:
        raise SystemExit('source changed during isolated control; refuse to overwrite another edit')
    source.write_bytes(original)
assert source.read_bytes() == original
if result.returncode != 101 or 'stale cancel crossed identity fence: session' not in result.stdout or 'assertion' not in result.stdout:
    raise SystemExit(f'expected causal Cancel assertion exit 101, got {result.returncode}:\n{result.stdout[-8000:]}')
(out / 'report.json').write_text(json.dumps({'command': command, 'exit_code': result.returncode,
    'assertion': 'stale cancel crossed identity fence: session',
    'source_sha256': hashlib.sha256(original).hexdigest(), 'source_restored': True}, indent=2)+'\n')
print('rich-cancel-control: removed only Cancel guard; causal assertion exit=101; exact source restored')
