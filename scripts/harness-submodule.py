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
    subprocess.run(['git', 'submodule', 'update', '--init', '--recursive', '--', MEMBER], cwd=root,
                   env={**os.environ, 'GIT_TERMINAL_PROMPT': '0', 'GCM_INTERACTIVE': 'never'}, check=True, timeout=180)
    child = root / MEMBER
    if not (child / '.git').is_file() or not (child / 'Cargo.toml').is_file():
        raise ValueError('harness: pinned submodule checkout is missing')
    head = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=child, text=True).strip()
    if head != entry[1]:
        raise ValueError('harness: checkout differs from the pinned gitlink')
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


def workspace_metadata(root, features=()):
    command = ['cargo', 'metadata', '--format-version', '1', '--locked']
    if features:
        command += ['--features', ','.join(features)]
    return json.loads(subprocess.check_output(command, cwd=root))


def validate_json_features(data):
    packages = {p['id']: p['name'] for p in data['packages']
                if p['name'] in {'serde_json', 'bboxy450-harness'}}
    nodes = {n['id']: set(n['features']) for n in data['resolve']['nodes']}
    if set(packages.values()) != {'serde_json', 'bboxy450-harness'} or not packages.keys() <= nodes.keys():
        raise ValueError('harness: parent JSON feature inventory is missing')
    violations = set()
    for ident, name in packages.items():
        if name == 'serde_json' and 'preserve_order' in nodes[ident]:
            violations.add('serde_json/preserve_order')
        if name == 'bboxy450-harness' and 'acp' in nodes[ident]:
            violations.add('bboxy450-harness/acp')
    if violations:
        raise ValueError('harness: parent feature isolation rejected: ' + ', '.join(sorted(violations)))
    return sorted(set().union(*(nodes[ident] for ident, name in packages.items() if name == 'serde_json')))


def check_json_features(root, features=()):
    resolved = validate_json_features(workspace_metadata(root, features))
    print('harness: parent JSON features isolated: ' + ', '.join(resolved), flush=True)


def check_resolution(root):
    data = workspace_metadata(root)
    validate_json_features(data)
    toolkit = {"bboxy450-rt": "bbt-rt", "bboxy450-yarn": "bbt-yarn", "bboxy450-tlc": "bboxy450-tlc",
               "bboxy450-ui": "bbt-ui", "bboxy450-ui-core": "bbt-ui-model", "bboxy450-render": "bbt-render", "bboxy450-doc": "bbt-doc"}
    for name, directory in toolkit.items():
        matches = [p for p in data['packages'] if p['name'] == name]
        if len(matches) != 1 or matches[0]['source'] is not None or Path(matches[0]['manifest_path']).resolve() != root / 'crates' / directory / 'Cargo.toml':
            raise ValueError(f'harness: {name} must resolve once to the in-tree path')
    print('harness: all seven toolkit dependencies resolve to in-tree paths', flush=True)


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
    # These are test-owned outputs already tracked by the upstream repository.
    # Preserve their actual results, then restore only these known artifacts so
    # the submodule pin continues to describe source. Never reset source files.
    if subprocess.check_output(['git', 'status', '--porcelain'], cwd=child).strip():
        raise ValueError('harness standalone gate requires a clean pinned checkout')
    names = ("test-current.json", "formal-native.json", "gui-gate.json", "gui-smoke.png")
    originals = {name: (child / ".bboxy/runs" / name).read_bytes() for name in names}
    artifacts.mkdir(parents=True, exist_ok=True)
    try:
        return subprocess.run(['make', 'test'], cwd=child).returncode
    finally:
        for name, content in originals.items():
            path = child / ".bboxy/runs" / name
            if path.is_file():
                shutil.copyfile(path, artifacts / name)
            path.write_bytes(content)
        archive_generated_evidence(child, artifacts)
        # The child's parity receipt also stays next to its models in its target.
        for path in (child / ".bboxy/runs").glob('*'):
            if path.is_file() and path.name not in names:
                tracked = subprocess.check_output(['git', 'ls-files', '--', str(path.relative_to(child))], cwd=child)
                if not tracked:
                    shutil.copyfile(path, artifacts / path.name)
                    path.unlink()  # Only artifacts this invocation created in the initially clean checkout.


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parent.parent)
    parser.add_argument('--models', type=Path)
    parser.add_argument('--standalone', action='store_true')
    parser.add_argument('--check-resolution', action='store_true')
    parser.add_argument('--check-json-features', action='store_true')
    parser.add_argument('--acp-control', action='store_true', help='reject the real ACP-enabled metadata graph (negative control)')
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
