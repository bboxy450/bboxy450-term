"""Unattended package-license fixtures; fake Cargo never builds or publishes."""
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Copyright (c) 2026 Boxy (bboxy450). Retain credit on reuse:
# https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
import json
import os
from pathlib import Path
import shutil
import subprocess
from tempfile import TemporaryDirectory
import unittest

SOURCE = Path(__file__).resolve().parent.parent
CHECK = SOURCE / 'scripts/license-packages.py'
COMMON = ('LICENSE', 'NOTICE', 'ATTRIBUTION.md', 'LICENSE-MIT', 'LICENSE-APACHE')
MEMBERS = {
    'bboxy450-ui': 'bbt-ui', 'bboxy450-term-core': 'bbt-core',
    'bboxy450-yarn': 'bbt-yarn', 'bboxy450-yarn-macros': 'bbt-yarn-macros',
    'bboxy450-render': 'bbt-render', 'bboxy450-mermaid': 'bbt-mermaid',
    'bboxy450-term-firehose': 'bboxy-firehose',
}
ASSETS = {
    'bbt-core': ('src/vendor/alacritty/LICENSE-APACHE', 'src/vendor/vte/LICENSE-MIT', 'src/vendor/vte/LICENSE-APACHE'),
    'bbt-render': ('tests/fonts/OFL.txt', 'tests/fonts/JetBrainsMono-Regular.ttf'),
    'bbt-mermaid': ('tests/fixtures/LICENSE-MIT',),
    'bboxy-firehose': ('reference/vendor/LICENSE', 'reference/vendor/unicode11-LICENSE'),
}


class PackageLicenses(unittest.TestCase):
    def setUp(self):
        self.temporary = TemporaryDirectory(prefix='bboxy-license-fixture-')
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.data = {'workspace_root': str(self.root), 'workspace_members': [], 'packages': []}
        self.lists = {}
        self.paths = {}
        for name in COMMON:
            shutil.copyfile(SOURCE / name, self.root / name)
        for name, directory in MEMBERS.items():
            path = self.root / 'members' / directory
            path.mkdir(parents=True)
            manifest = path / 'Cargo.toml'
            manifest.write_text(f'[package]\nname="{name}"\nversion="0.0.0"\nlicense-file="LICENSE"\n')
            self.paths[name] = path
            self.data['workspace_members'].append(name)
            self.data['packages'].append({'id': name, 'name': name, 'manifest_path': str(manifest),
                                          'license': None, 'license_file': str(path / 'LICENSE')})
            for filename in COMMON:
                source = SOURCE / 'crates' / directory / filename if filename.startswith('LICENSE-') else SOURCE / filename
                shutil.copyfile(source, path / filename)
            for filename in ASSETS.get(directory, ()):
                (path / filename).parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(SOURCE / 'crates' / directory / filename, path / filename)
            self.lists[name] = list(COMMON) + ([] if directory == 'bboxy-firehose' else list(ASSETS.get(directory, ())))
        self.root.joinpath('Cargo.toml').write_text('[workspace]\nmembers=' + json.dumps(['members/' + d for d in MEMBERS.values()]) + '\n')
        mapfile = self.root / '.bboxy/specs/licensing.md'
        mapfile.parent.mkdir(parents=True)
        mapfile.write_text('| Crate | Publishes as | License | Why |\n|---|---|---|---|\n' + ''.join(
            f'| `{directory}` | `{name}` | Respectful Slop License 1.0-Draft + upstream terms and prior grants retained | fixture |\n'
            for name, directory in MEMBERS.items()))
        binary = self.root / 'bin'
        binary.mkdir()
        cargo = binary / 'cargo'
        cargo.write_text('''#!/usr/bin/env python3
import json,os,sys
from pathlib import Path
root=Path(os.environ['BBOXY_LICENSE_FIXTURE'])
a=sys.argv[1:]
with (root/'calls.jsonl').open('a') as out: out.write(json.dumps(a)+'\\n')
if a==['metadata','--no-deps','--format-version','1','--locked']:
 print((root/'metadata.json').read_text())
else:
 assert a[:-1]==['package','--locked','--allow-dirty','--list','--package'],a
 print('\\n'.join(json.loads((root/'lists.json').read_text())[a[-1]]))
''')
        cargo.chmod(0o700)
        self.environment = dict(os.environ, PATH=str(binary) + os.pathsep + os.environ['PATH'], BBOXY_LICENSE_FIXTURE=str(self.root))

    def run_check(self, expected=0, reason=None, *arguments):
        (self.root / 'metadata.json').write_text(json.dumps(self.data))
        (self.root / 'lists.json').write_text(json.dumps(self.lists))
        result = subprocess.run(['python3', '-I', str(CHECK), str(self.root), *arguments],
                                env=self.environment, capture_output=True, text=True, timeout=20)
        self.assertEqual(result.returncode, expected, result.stdout + result.stderr)
        if reason:
            self.assertIn(reason, result.stdout + result.stderr)
        return result

    def test_valid_mixed_packages_and_targeted_package_lists(self):
        self.run_check()
        self.run_check(0, '7 package file lists checked', '--packages')
        (self.root / 'calls.jsonl').unlink()
        self.run_check(0, '1 package file lists checked', '--packages', '--package', 'bboxy450-ui')
        calls = [json.loads(line) for line in (self.root / 'calls.jsonl').read_text().splitlines()]
        self.assertEqual([call[-1] for call in calls if call[0] == 'package'], ['bboxy450-ui'])
        self.run_check(1, 'unknown workspace package', '--packages', '--package', 'missing')

    def test_missing_empty_symlink_and_stale_policy_copies_refuse(self):
        for filename in ('LICENSE', 'NOTICE', 'ATTRIBUTION.md', 'LICENSE-MIT'):
            path = self.paths['bboxy450-ui'] / filename
            original = path.read_bytes()
            path.unlink()
            self.run_check(1, filename)
            path.write_bytes(b'')
            self.run_check(1, filename)
            target = self.root / 'saved'
            target.write_bytes(original)
            path.unlink()
            path.symlink_to(target)
            self.run_check(1, 'regular copy')
            path.unlink()
            path.write_bytes(original + b'changed')
            self.run_check(1, 'canonical' if filename in COMMON[:3] else 'preserved')
            path.write_bytes(original)
        self.run_check()
        root_notice = self.root / 'NOTICE'
        original = root_notice.read_bytes()
        root_notice.unlink()
        root_notice.symlink_to(self.paths['bboxy450-ui'] / 'NOTICE')
        self.run_check(1, 'regular copy')
        root_notice.unlink()
        root_notice.write_bytes(original)

    def test_metadata_and_map_reject_blanket_or_ambiguous_terms(self):
        package = self.data['packages'][0]
        for key, value, reason in [('license', 'MIT', 'blanket SPDX'), ('license_file', None, 'missing license_file'),
                                   ('license_file', str(self.root / 'LICENSE'), 'local LICENSE')]:
            old = package[key]
            package[key] = value
            self.run_check(1, reason)
            package[key] = old
        manifest = self.paths['bboxy450-ui'] / 'Cargo.toml'
        original = manifest.read_text()
        manifest.write_text(original.replace('license-file="LICENSE"', 'license.workspace=true'))
        self.run_check(1, 'manifest must explicitly')
        manifest.write_text(original)
        mapfile = self.root / '.bboxy/specs/licensing.md'
        original = mapfile.read_text()
        mapfile.write_text(original.replace('Respectful Slop License 1.0-Draft + upstream terms and prior grants retained', 'MIT OR Apache-2.0', 1))
        self.run_check(1, 'retained upstream/prior')
        mapfile.write_text(original.replace('`bbt-ui`', '`unmapped`', 1))
        self.run_check(1, 'missing from the licensing map')
        mapfile.write_text(original + original.splitlines()[2] + '\n')
        self.run_check(1, 'map repeats')
        mapfile.write_text(original.replace('fixture |', r'[[decision\|alias]] and [[another\|more\|aliases]] |'))
        self.run_check()
        aliased = mapfile.read_text()
        mapfile.write_text(aliased.replace('Respectful Slop License 1.0-Draft + upstream terms and prior grants retained', 'MIT OR Apache-2.0', 1))
        self.run_check(1, 'retained upstream/prior')
        mapfile.write_text(original)
        self.data['workspace_members'].append('absent')
        self.run_check(1, 'member has no package')
        self.data['workspace_members'].pop()
        self.data['packages'].append(dict(package))
        self.run_check(1, 'duplicate workspace package')

    def test_upstream_terms_and_assets_are_preserved_and_packaged(self):
        for name, directory in MEMBERS.items():
            for filename in ASSETS.get(directory, ()):
                path = self.paths[name] / filename
                original = path.read_bytes()
                path.unlink()
                self.run_check(1, filename)
                path.write_bytes(original + b'changed')
                self.run_check(1, 'preserved prior/upstream bytes changed')
                path.write_bytes(original)
        self.lists['bboxy450-term-core'].remove('src/vendor/vte/LICENSE-MIT')
        self.run_check(1, 'package omits src/vendor/vte/LICENSE-MIT', '--packages')
        self.lists['bboxy450-term-core'].append('src/vendor/vte/LICENSE-MIT')
        self.lists['bboxy450-render'].remove('tests/fonts/OFL.txt')
        self.run_check(1, 'package omits tests/fonts/OFL.txt', '--packages')
        self.lists['bboxy450-render'].append('tests/fonts/OFL.txt')
        self.lists['bboxy450-term-firehose'].append('reference/vendor/LICENSE')
        self.run_check(1, 'excluded reference assets', '--packages')

    def test_missing_package_policy_and_notices_are_detected(self):
        for filename in ('LICENSE', 'NOTICE', 'ATTRIBUTION.md', 'LICENSE-APACHE'):
            self.lists['bboxy450-ui'].remove(filename)
            self.run_check(1, 'package omits ' + filename, '--packages')
            self.lists['bboxy450-ui'].append(filename)

    def test_sync_repairs_copies_but_never_upstream_bytes(self):
        path = self.paths['bboxy450-ui'] / 'LICENSE'
        path.write_text('stale')
        self.run_check(0, 'synchronized', '--sync')
        self.assertEqual(path.read_bytes(), (self.root / 'LICENSE').read_bytes())
        self.run_check()
        old = self.paths['bboxy450-yarn'] / 'LICENSE-MIT'
        original = old.read_bytes()
        old.write_text('changed upstream')
        path.write_text('still stale')
        self.run_check(1, 'preserved', '--sync')
        self.assertEqual(path.read_text(), 'still stale', 'failed sync never starts rewriting files')
        self.assertEqual(old.read_text(), 'changed upstream', 'sync never repairs or rewrites upstream terms')
        old.write_bytes(original)


if __name__ == '__main__':
    unittest.main()
