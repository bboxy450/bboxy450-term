#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Remove only native workflow forwarding and require the real child/MCP witness to fail."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

p = argparse.ArgumentParser()
p.add_argument('--target', type=Path, required=True)
a = p.parse_args()
root = Path(__file__).resolve().parents[1]
source = root / 'crates/bboxy450-harness-wake-up/src/launch.rs'
original = source.read_bytes()
guard = b'format!("{}\\n\\n{}", startup_prompt(context)?, context.skill)'
assert original.count(guard) == 2
# The first is bboxy450-harness; leave the independent Codex path intact.
broken = original.replace(guard, b'startup_prompt(context)?', 1)
out = a.target / 'rich-launch-context-control'
out.mkdir(parents=True, exist_ok=True)
command = ['make', 'test-rich-launch-context', f'TARGET_DIR={a.target}']
try:
    source.write_bytes(broken)
    result = subprocess.run(command, cwd=root, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    (out / 'broken.log').write_text(result.stdout)
finally:
    if source.read_bytes() != broken:
        raise SystemExit('source changed during isolated control; refusing overwrite')
    source.write_bytes(original)
assertion = 'native system context lost # bboxy450 seat workflow'
if result.returncode != 2 or assertion not in result.stdout:
    raise SystemExit(f'expected causal workflow assertion, got {result.returncode}:\n{result.stdout[-6000:]}')
restored = subprocess.run(command, cwd=root, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
(out / 'restored.log').write_text(restored.stdout)
if restored.returncode:
    raise SystemExit(f'restored context failed: {restored.returncode}:\n{restored.stdout[-6000:]}')
(out / 'report.json').write_text(json.dumps({'source_sha256': hashlib.sha256(original).hexdigest(),
    'source_restored': source.read_bytes() == original, 'command': command,
    'broken_exit': result.returncode, 'restored_exit': restored.returncode,
    'assertion': assertion}, indent=2) + '\n')
print('rich-launch-context-control: native workflow forwarding removed; actual child assertion; exact restored pass')
