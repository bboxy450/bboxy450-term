#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Initialize the pinned harness or import its native model exports; never skip it."""
import argparse
import json
from pathlib import Path
import os
import shutil
import subprocess
import tomllib

MEMBER = 'crates/bboxy450-harness'
URL = 'https://github.com/bboxy450/bboxy450-harness'


def initialize(root):
    if MEMBER not in tomllib.loads((root / 'Cargo.toml').read_text()).get('workspace', {}).get('members', []):
        return  # Standalone checker and landing's tiny repository fixtures.
    module = subprocess.check_output(['git', 'config', '-f', '.gitmodules', '--get', f'submodule.{MEMBER}.url'], cwd=root, text=True).strip()
    entry = subprocess.check_output(['git', 'ls-files', '--stage', '--', MEMBER], cwd=root, text=True).split()
    if module != URL or len(entry) != 4 or entry[0] != '160000':
        raise ValueError('harness: missing or unapproved pinned submodule')
    candidate = os.environ.get('BBOXY_HARNESS_CANDIDATE')
    child = root / MEMBER
    if candidate:
        if len(candidate) != 40 or any(c not in '0123456789abcdef' for c in candidate):
            raise ValueError('harness: candidate must be an exact commit SHA')
        if subprocess.check_output(['git', 'status', '--porcelain'], cwd=child).strip():
            raise ValueError('harness: candidate requires a clean child worktree')
        subprocess.run(['git', 'verify-commit', candidate], cwd=child, check=True)
        subprocess.run(['git', 'checkout', '--detach', candidate], cwd=child, check=True)
    else:
        subprocess.run(['git', 'submodule', 'update', '--init', '--recursive', '--', MEMBER], cwd=root,
                       env={**os.environ, 'GIT_TERMINAL_PROMPT': '0', 'GCM_INTERACTIVE': 'never'}, check=True, timeout=180)
    if not (child / '.git').is_file() or not (child / 'Cargo.toml').is_file():
        raise ValueError('harness: pinned submodule checkout is missing')
    head = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=child, text=True).strip()
    if head != (candidate or entry[1]):
        raise ValueError('harness: checkout differs from the requested pin')
    if candidate:
        print(f'harness: PRIVATE TEST CANDIDATE {head}; committed pin remains {entry[1]}', flush=True)
    print(f'harness: pinned {head}', flush=True)
    return True


def import_models(root, source, target):
    expected = {m['name']: m['expected'] for m in tomllib.loads((root / MEMBER / 'verification/corpus.toml').read_text())['model']}
    incoming = (source / 'manifest.tsv').read_text().splitlines()
    rows = [line.split('\t') for line in incoming]
    if len(rows) != len(expected) or {r[0]: r[1] for r in rows} != expected:
        raise ValueError('harness: missing, duplicate or unexpected native model')
    current = (target / 'manifest.tsv').read_text().splitlines()
    if {r.split('\t')[0] for r in current} & set(expected):
        raise ValueError('harness: duplicate imported model')
    for name in expected:
        for suffix in ('.tla', '.cfg', 'Graph.cfg', '.states', '.edges', '.native-trace', '.metadata.json'):
            shutil.copyfile(source / (name + suffix), target / (name + suffix))
    (target / 'manifest.tsv').write_text('\n'.join(current + incoming) + '\n')
    print(f'harness: imported {len(rows)} native models for parent TLC parity', flush=True)


def workspace_metadata(root):
    return json.loads(subprocess.check_output(['cargo', 'metadata', '--format-version', '1', '--locked'], cwd=root))


def validate_json_features(data):
    if set(data) != {'serde_json', 'bboxy450-harness'}:
        raise ValueError('harness: parent JSON feature inventory is missing')
    harness_features = data['bboxy450-harness']
    violations = set()
    # The requested ACP-agent backend enables preserve_order upstream. The
    # legacy ACP frontend remains forbidden in term's embedded dependency.
    if 'preserve_order' in data['serde_json'] and 'acp-agent' not in harness_features:
        violations.add('serde_json/preserve_order')
    if 'acp' in harness_features:
        violations.add('bboxy450-harness/acp')
    if violations:
        raise ValueError('harness: parent feature isolation rejected: ' + ', '.join(sorted(violations)))
    return sorted(data['serde_json'])


def check_json_features(root, features=()):
    # Workspace metadata unions the standalone harness defaults with the
    # embedded consumer. Cargo tree selects the consumer's actual closure.
    command = ['cargo', 'tree', '-p', 'bboxy450-term', '--locked', '-e', 'normal,build',
               '--prefix', 'none', '-f', '{p}|{f}']
    if features:
        command += ['--features', ','.join(features)]
    data = {}
    for row in subprocess.check_output(command, cwd=root, text=True).splitlines():
        package, separator, selected = row.rpartition('|')
        if not separator or not package.split():
            raise ValueError('harness: malformed consumer feature row')
        name = package.split()[0]
        if name in {'serde_json', 'bboxy450-harness'}:
            data.setdefault(name, set()).update(filter(None, selected.removesuffix(' (*)').split(',')))
    resolved = validate_json_features(data)
    print('harness: consumer JSON features (ACP-agent includes preserve_order): ' + ', '.join(resolved), flush=True)
    return resolved


def check_resolution(root):
    data = workspace_metadata(root)
    check_json_features(root)
    toolkit = {"bboxy450-rt": "bbt-rt", "bboxy450-yarn": "bbt-yarn", "bboxy450-tlc": "bboxy450-tlc",
               "bboxy450-ui": "bbt-ui", "bboxy450-ui-core": "bbt-ui-model", "bboxy450-render": "bbt-render", "bboxy450-doc": "bbt-doc", "bboxy450-term-desktop-core": "bboxy450-term-desktop-core", "bboxy450-term-desktop-macos": "bboxy450-term-desktop-macos", "bboxy450-term-desktop": "bboxy450-term-desktop"}
    for name, directory in toolkit.items():
        matches = [p for p in data['packages'] if p['name'] == name]
        if len(matches) != 1 or matches[0]['source'] is not None or Path(matches[0]['manifest_path']).resolve() != root / 'crates' / directory / 'Cargo.toml':
            raise ValueError(f'harness: {name} must resolve once to the in-tree path')
    print('harness: all ten toolkit dependencies resolve to in-tree paths', flush=True)


def archive_generated_evidence(child, artifacts):
    evidence = child / 'verification/evidence'
    if not evidence.exists():
        return
    # The gate started with a clean checkout. These untracked exports belong to
    # this test invocation, not to the pinned source; retain them outside it.
    if evidence.is_symlink() or subprocess.check_output(['git', 'ls-files', '--', 'verification/evidence'], cwd=child).strip():
        raise ValueError('harness: refuse to move tracked or linked formal evidence')
    shutil.copytree(evidence, artifacts / 'formal-evidence', dirs_exist_ok=True)
    shutil.rmtree(evidence)


def standalone(root, artifacts):
    child = root / MEMBER
    # Harness run evidence is machine-local (.bboxy/.gitignore), so a gate run
    # never changes what the pin describes. Keep this run's outputs beside it.
    if subprocess.check_output(['git', 'status', '--porcelain'], cwd=child).strip():
        raise ValueError('harness standalone gate requires a clean pinned checkout')
    artifacts.mkdir(parents=True, exist_ok=True)
    try:
        return subprocess.run(['make', 'test'], cwd=child).returncode
    finally:
        archive_generated_evidence(child, artifacts)
        for path in (child / ".bboxy/runs").glob('*'):
            if path.is_file():
                shutil.copyfile(path, artifacts / path.name)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parent.parent)
    parser.add_argument('--models', type=Path)
    parser.add_argument('--standalone', action='store_true')
    parser.add_argument('--check-resolution', action='store_true')
    parser.add_argument('--check-json-features', action='store_true')
    parser.add_argument('--acp-control', action='store_true', help='reject the real ACP-enabled consumer closure (negative control)')
    parser.add_argument('--target', type=Path)
    args = parser.parse_args()
    if args.acp_control and not args.check_json_features:
        parser.error('--acp-control requires --check-json-features')
    if args.check_json_features:
        check_json_features(args.root, ['bboxy450-harness/acp'] if args.acp_control else ())
        return
    if args.check_resolution:
        check_resolution(args.root)
        return
    if args.standalone:
        raise SystemExit(standalone(args.root, args.target))
    if args.models:
        import_models(args.root, args.models, args.target)
    else:
        if initialize(args.root):
            check_json_features(args.root)


if __name__ == '__main__':
    try:
        main()
    except (ValueError, OSError, subprocess.SubprocessError) as error:
        raise SystemExit(f'harness: FAILED: {error}')
