#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Causal GUI-only lifecycle control; never signals a nonfixture process."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

p = argparse.ArgumentParser()
p.add_argument('--target', type=Path, required=True)
a = p.parse_args()
root = Path(__file__).resolve().parents[1]
source = root / 'crates/bboxy450-term/src/cockpit/gui_handoff.rs'
original = source.read_bytes()
guard = b'owner.signal(libc::SIGHUP)?;'
assert original.count(guard) == 1
broken = original.replace(guard, b'owner.signal(libc::SIGTERM)?;')
out = a.target / 'rich-gui-handoff-control'
out.mkdir(parents=True, exist_ok=True)
command = ['make', 'test-rich-gui-handoff', f'TARGET_DIR={a.target}']
try:
    source.write_bytes(broken)
    result = subprocess.run(command, cwd=root, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    (out / 'broken.log').write_text(result.stdout)
finally:
    if source.read_bytes() != broken:
        raise SystemExit('source changed during isolated control; refusing overwrite')
    source.write_bytes(original)
if result.returncode != 2 or 'GUI handoff stopped a backend owner' not in result.stdout:
    raise SystemExit(f'expected causal backend-owner assertion, got {result.returncode}:\n{result.stdout[-6000:]}')
restored = subprocess.run(command, cwd=root, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
(out / 'restored.log').write_text(restored.stdout)
if restored.returncode:
    raise SystemExit(f'restored handoff failed: {restored.returncode}:\n{restored.stdout[-6000:]}')
(out / 'report.json').write_text(json.dumps({'source_sha256': hashlib.sha256(original).hexdigest(),
    'source_restored': source.read_bytes() == original, 'command': command,
    'broken_exit': result.returncode, 'restored_exit': restored.returncode,
    'assertion': 'GUI handoff stopped a backend owner'}, indent=2) + '\n')
print('rich-gui-handoff-control: GUI-only HUP replaced by TERM; backend-owner assertion; exact restored pass')
