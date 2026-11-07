#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Focused shared-composer/private-native delivery gate; no corpus, install or backend action."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
ROOT = Path(__file__).resolve().parents[1]
PRODUCER = ROOT / 'crates/bboxy450-harness'
TARGET = Path(os.environ.get('TARGET_DIR', ROOT / 'target')).resolve()
OUT = TARGET / 'harness-attachments'
OUT.mkdir(parents=True, exist_ok=True)
env = dict(os.environ, RUSTC_WRAPPER='', CARGO_BUILD_JOBS='2', RUST_TEST_THREADS='2', TARGET_DIR=str(TARGET))
commands = []


def source_hashes():
    paths = [ROOT / p for p in ['Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml', 'Makefile', 'ATTRIBUTION.md', 'scripts/verify-harness-attachments.py']]
    paths += [ROOT / ('crates/bboxy450-term/' + p) for p in ['src/rich_pane.rs', 'src/rich_faces.rs', 'src/terminal_clipboard.rs', 'src/main.rs', 'tests/harness_attachments.rs']]
    paths += [PRODUCER / p for p in ['Cargo.toml', 'Cargo.lock', 'verification/tools.lock.json', 'scripts/formal.py', 'tests/formal/export.rs', 'Makefile', 'ATTRIBUTION.md', 'src/protocol/mod.rs', 'src/store.rs', 'src/store/intake.rs', 'src/engine/mod.rs', 'src/backend/acp_agent.rs', 'tests/harness_attachments.rs', 'tests/gui_bridge.rs', 'tests/fixtures/acp-agent/attachments.py', 'tests/formal/models.rs', 'tests/formal_models.rs', 'verification/contracts.toml', 'verification/corpus.toml', 'scripts/verify-harness-attachments.py']]
    paths += list((PRODUCER / 'src/frontends/gui').glob('*.rs'))
    return {str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(paths)}


def run(name, command):
    log = OUT / (name + '.log')
    with log.open('w') as stream:
        result = subprocess.run(command, cwd=ROOT, env=env, stdout=stream, stderr=subprocess.STDOUT)
    commands.append(dict(command=command, exit_code=result.returncode, log_sha256=hashlib.sha256(log.read_bytes()).hexdigest(), log=str(log)))
    print(f'harness-attachments: {name}: exit {result.returncode}', flush=True)
    if result.returncode:
        print(log.read_text()[-7000:], file=sys.stderr)
        raise SystemExit(result.returncode)


initial = source_hashes()
run('producer', ['python3', '-I', str(PRODUCER / 'scripts/verify-harness-attachments.py')])
cargo = ['--offline', '--locked', '--jobs', '2', '--target-dir', str(TARGET)]
run('native-worker', ['cargo', 'nextest', 'run', *cargo, '-p', 'bboxy450-term', '--test', 'it', '-E', 'binary(it)&test(/^harness_attachments::/)'])
run('private-native-clipboard', ['cargo', 'nextest', 'run', *cargo, '-p', 'bboxy450-term-desktop-macos', '--lib', '-E', 'test(clipboard_private_pasteboard_snapshots_png_tiff_text_without_general_clipboard)'])
run('strict-consumer', ['cargo', 'clippy', *cargo, '-p', 'bboxy450-term', '--lib', '--bin', 'bboxy450-term', '--test', 'it', '--no-deps', '--', '-D', 'warnings'])
assert source_hashes() == initial, 'source changed while focused verification ran'
producer = json.loads((OUT / 'producer.json').read_text())
report = dict(verify=dict(command='make verify-harness-attachments', exit_code=0), producer_head=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=PRODUCER, text=True).strip(), consumer_head=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(), source_hashes=initial, commands=commands, producer=producer, operator_input=False, skipped=['full test/parity/formal/runtime corpus', 'paid provider', 'desktop input', 'backend restart', 'pin update', 'push', 'install'])
(ROOT / '.bboxy/runs/codex-harness-attachments.json').write_text(json.dumps(report, indent=2) + '\n')
