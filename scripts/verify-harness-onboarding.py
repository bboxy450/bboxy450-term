#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
# Adapted from scripts/verify-harness-attachments.py: source-bound focused producer/consumer evidence.
"""One private, unattended onboarding gate. No install, backend restart or inference."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
ROOT = Path(__file__).resolve().parents[1]
PRODUCER = ROOT / 'crates/bboxy450-harness'
TARGET = Path(os.environ.get('TARGET_DIR', ROOT / 'target')).resolve()
assert ROOT in TARGET.parents, 'Use this worktree local target'
OUT = TARGET / 'harness-onboarding'
OUT.mkdir(parents=True, exist_ok=True)
env = dict(os.environ, TARGET_DIR=str(TARGET), RUSTC_WRAPPER='', CARGO_PROFILE_DEV_DEBUG='0', CARGO_BUILD_JOBS='2', RUST_TEST_THREADS='2')


def hashes():
    paths = [ROOT / p for p in ['Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml', 'Makefile', 'ATTRIBUTION.md', 'scripts/verify-harness-onboarding.py']]
    paths += [PRODUCER / p for p in ['Cargo.toml', 'Cargo.lock', 'Makefile', 'ATTRIBUTION.md', 'src/lib.rs', 'src/onboarding.rs', 'src/process.rs', 'tests/harness_onboarding.rs', 'tests/formal_models.rs', 'tests/formal/models.rs', 'tests/formal/export.rs', 'verification/contracts.toml', 'verification/corpus.toml', 'verification/coverage.toml', 'verification/tools.lock.json', 'scripts/formal.py', 'scripts/verify-harness-onboarding.py']]
    paths += list((PRODUCER / 'src/frontends/gui').glob('*.rs'))
    paths += list((ROOT / 'crates').glob('*/ATTRIBUTION.md')) + list((ROOT / 'plugins').glob('*/ATTRIBUTION.md'))
    paths += list((ROOT / 'crates/bboxy450-term/src').glob('rich_*.rs'))
    return {str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(paths)}


initial = hashes()
commands = []
for name, command in [
    ('producer', ['python3', '-I', str(PRODUCER / 'scripts/verify-harness-onboarding.py')]),
    ('strict-consumer', ['cargo', 'clippy', '--offline', '--locked', '--jobs', '2', '--target-dir', str(TARGET), '-p', 'bboxy450-term', '--lib', '--bin', 'bboxy450-term', '--no-deps', '--', '-D', 'warnings']),
    ('parent-license-copies-and-source-notices', ['make', 'check-licenses']),
]:
    log = OUT / (name + '.log')
    with log.open('w') as stream:
        result = subprocess.run(command, cwd=ROOT, env=env, stdout=stream, stderr=subprocess.STDOUT)
    commands.append(dict(name=name, command=command, exit_code=result.returncode, log=str(log), sha256=hashlib.sha256(log.read_bytes()).hexdigest()))
    print(f'harness-onboarding: {name}: exit {result.returncode}', flush=True)
    if result.returncode:
        print(log.read_text()[-7000:], file=sys.stderr)
        raise SystemExit(result.returncode)
assert hashes() == initial, 'Source changed during verification'
receipt = dict(verify=dict(command='make verify-harness-onboarding', exit_code=0), source_hashes=initial, commands=commands, producer=json.loads((OUT / 'producer.json').read_text()), producer_head=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=PRODUCER, text=True).strip(), consumer_head=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(), operator_input=False, skipped=['full CI/parity/runtime/formal corpus', 'live vendor inference/authentication', 'real package/profile changes', 'backend restart', 'production bundles/install', 'desktop input', 'push/merge/gitlink pin'])
(ROOT / '.bboxy/runs/codex-harness-onboarding.json').write_text(json.dumps(receipt, indent=2) + '\n')
