#!/usr/bin/env python3
"""Night-selector migration and the public shell dispatch contract (#327)."""
from collections import Counter
import json
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import tempfile
import unittest

SCRIPTS = Path(__file__).resolve().parent


def selectors(path):
    return [line for line in path.read_bytes().splitlines(keepends=True)
            if line.startswith(b"cargo test ")]


class NightBoundaries(unittest.TestCase):
    # keeps: spec test-policy §3 (named real boundaries remain in the night)
    def test_migration_retains_each_selector_and_each_crate_owns_its_commands(self):
        baseline = selectors(SCRIPTS / 'fixtures/night-boundaries-before.selectors')
        parts = sorted((SCRIPTS / 'night-boundaries.d').glob('*.sh'))
        self.assertTrue(parts)
        current = []
        for part in parts:
            for line in selectors(part):
                words = shlex.split(line.decode())
                self.assertEqual(words[words.index('-p') + 1], part.stem)
                self.assertEqual(words[-4:], ['--', '--ignored', '--exact', '--nocapture'])
                current.append(line)
        # Future named additions need no edit to this shared historical fixture.
        self.assertFalse(Counter(baseline) - Counter(current))
        self.assertEqual(len(current), len(set(current)))

    def fixture(self):
        temp = tempfile.TemporaryDirectory(prefix='bbt night boundaries ')
        self.addCleanup(temp.cleanup)
        root = Path(temp.name)
        scripts = root / 'scripts'
        scripts.mkdir()
        shutil.copy2(SCRIPTS / 'night-test-boundaries.sh', scripts)
        (scripts / 'cache-env.sh').write_text(':\n')
        parts = scripts / 'night-boundaries.d'
        parts.mkdir()
        binary = root / 'bin'
        binary.mkdir()
        cargo = binary / 'cargo'
        cargo.write_text('''#!/usr/bin/env python3
import json,os,sys
with open(os.environ['BBT_NIGHT_RECORD'], 'a') as out:
    out.write(json.dumps({'argv':sys.argv[1:], 'cwd':os.getcwd(),
        'custody':os.environ.get('BBOXY_TEST_CUSTODY'),
        'assertions':[os.environ.get('CARGO_PROFILE_DEV_DEBUG_ASSERTIONS'),
                      os.environ.get('CARGO_PROFILE_TEST_DEBUG_ASSERTIONS')]})+'\\n')
sys.exit(19 if os.environ.get('BBT_NIGHT_FAIL') in sys.argv[1:] else 0)
''')
        cargo.chmod(0o700)
        env = os.environ.copy()
        env.update(PATH=str(binary) + os.pathsep + env['PATH'],
                   BBT_NIGHT_RECORD=str(root / 'calls.jsonl'))
        env.pop('BBT_NIGHT_FAIL', None)
        return root, parts, env

    def run_fixture(self, root, env):
        # A hang guard only; this fixture runs no compiler, agent or PTY.
        result = subprocess.run(['sh', str(root / 'scripts/night-test-boundaries.sh')],
                                cwd=root.parent, env=env, capture_output=True,
                                text=True, timeout=120)
        records = root / 'calls.jsonl'
        calls = [json.loads(line) for line in records.read_text().splitlines()] if records.exists() else []
        return result, calls

    # keeps: spec test-policy §3 (the entry point executes every exact selector once)
    def test_entry_point_runs_real_selectors_in_sorted_crate_order(self):
        root, parts, env = self.fixture()
        expected = []
        for source in sorted((SCRIPTS / 'night-boundaries.d').glob('*.sh')):
            shutil.copy2(source, parts)
            expected.extend(shlex.split(line.decode())[1:] for line in selectors(source))
        result, calls = self.run_fixture(root, env)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual([row['argv'] for row in calls], expected)
        self.assertTrue(calls)
        for row in calls:
            self.assertEqual(Path(row['cwd']).resolve(), root.resolve())
            self.assertEqual(row['custody'], 'memory')
            self.assertEqual(row['assertions'], ['true', 'true'])

    # keeps: spec test-policy §3 (a missing manifest cannot silently skip the night)
    def test_missing_or_empty_directory_fails_before_running_any_command(self):
        for missing in [False, True]:
            with self.subTest(missing=missing):
                root, parts, env = self.fixture()
                if missing:
                    parts.rmdir()
                result, calls = self.run_fixture(root, env)
                self.assertEqual(result.returncode, 2)
                self.assertIn('night-boundaries.d', result.stderr)
                self.assertEqual(calls, [])

    # keeps: spec test-policy §3 (the script propagates the first real verdict)
    def test_failure_stops_the_current_crate_and_later_crates(self):
        root, parts, env = self.fixture()
        (parts / 'z-last.sh').write_text('set -eu\ncargo test -p z last -- --ignored --exact --nocapture\n')
        (parts / 'a-first.sh').write_text('set -eu\ncargo test -p a broken -- --ignored --exact --nocapture\ncargo test -p a never -- --ignored --exact --nocapture\n')
        env['BBT_NIGHT_FAIL'] = 'broken'
        result, calls = self.run_fixture(root, env)
        self.assertEqual(result.returncode, 19)
        self.assertEqual([row['argv'][3] for row in calls], ['broken'])


if __name__ == '__main__':
    unittest.main()
