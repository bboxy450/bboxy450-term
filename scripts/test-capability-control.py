# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Reintroduce the actual exec-time value, demand the process scan, restore it."""
import fcntl
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

root = Path(__file__).resolve().parents[1]
target = Path(sys.argv[1]).resolve()
out = target / 'capability-file'
out.mkdir(parents=True, exist_ok=True)
command = ['cargo', 'test', '--locked', '-p', 'bboxy450-term', '--test', 'capability_file',
           '--target-dir', str(target), 'every_seat_kind_keeps_capability_out_of_seat_and_ancestor_environments',
           '--', '--exact', '--nocapture']
source = root / 'crates/bbt-supervisor/src/supervisor.rs'
anchor = '            command.env("BBOXY_PROCESS_GENERATION", spec.generation.to_string());'
injection = anchor + '''
            if let Some(path) = spec.env.get("BBOXY_CAPABILITY_FILE") {
                command.env("BBOXY_CAPABILITY", std::fs::read_to_string(path).unwrap().trim());
            }'''
with (out / 'control.claim').open('a+') as claim:
    fcntl.flock(claim, fcntl.LOCK_EX | fcntl.LOCK_NB)
    claim.seek(0)
    claim.truncate()
    claim.write(str(os.getpid()) + '\n')
    claim.flush()
    original = source.read_text()
    digest = hashlib.sha256(original.encode()).hexdigest()
    if original.count(anchor) != 1:
        raise SystemExit('exec-environment mutation anchor drift')
    try:
        source.write_text(original.replace(anchor, injection))
        red = subprocess.run(command, cwd=root, capture_output=True, text=True, timeout=900)
        output = red.stdout + red.stderr
        (out / 'environment-red.log').write_text(output)
        if red.returncode != 101 or 'SeatAndAncestorEnvironmentIsCapabilityFree' not in output:
            raise RuntimeError('wanted 101 at the actual environment scan; inspect environment-red.log')
    finally:
        source.write_text(original)
    if hashlib.sha256(source.read_bytes()).hexdigest() != digest:
        raise RuntimeError('production source restoration hash mismatch')
    green = subprocess.run(command, cwd=root, capture_output=True, text=True, timeout=900)
    (out / 'environment-green.log').write_text(green.stdout + green.stderr)
    record = dict(command=command, assertion='SeatAndAncestorEnvironmentIsCapabilityFree',
                  red_exit=red.returncode, green_exit=green.returncode,
                  restored_source_sha256=digest, operator_input=False)
    (out / 'environment-control.json').write_text(json.dumps(record, indent=2) + '\n')
    print(json.dumps(record), flush=True)
    if green.returncode:
        raise SystemExit(green.returncode)
