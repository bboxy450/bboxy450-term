# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Real owned children prove snapshots preserve argv/env and detect a leak."""
import importlib.util
import errno
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('capability_scan', Path(__file__).parent / 'e2e/capability_scan.py')
scan = importlib.util.module_from_spec(spec)
spec.loader.exec_module(scan)


class ProcessScanTests(unittest.TestCase):
    def child(self, home, extra=None):
        command = [sys.executable, '-c', 'import signal; signal.pause()', 'space = arg', '']
        child = subprocess.Popen(command, env=dict(HOME=home, FIXTURE_MARKER='spaces = and equals', **(extra or {})))
        self.addCleanup(lambda: (child.terminate(), child.wait()) if child.poll() is None else None)
        return child

    def test_actual_kernel_environment_is_serialized_without_inheriting_the_runner(self):
        with tempfile.TemporaryDirectory() as home:
            child = self.child(home)
            argv, env = scan.process_snapshot(child.pid)
            self.assertEqual(argv[-2:], [b'space = arg', b''])
            self.assertEqual(env[b'FIXTURE_MARKER'], b'spaces = and equals')
            self.assertEqual(env[b'HOME'], os.fsencode(home))
            # KERN_PROCARGS2 also includes Darwin's auxiliary string vector.
            # Keeping it is conservative; no arbitrary runner variable is used.
            apple = {b'main_stack', b'executable_cdhash', b'security_config', b'pfz',
                     b'stack_guard', b'executable_file', b'malloc_entropy', b'ptr_munge',
                     b'executable_boothash', b'dyld_file', b'th_port'}
            self.assertLessEqual(set(env), {b'HOME', b'FIXTURE_MARKER'} | apple)
            row = scan.scan_process(child.pid, {b'ab' * 32}, home, ['owned-control'])
            self.assertEqual([s['source'] for s in row['sources']],
                             ['ps-Eww', 'kernel-environment', 'env-of-kernel-snapshot'])

    def test_real_legacy_value_is_detected_without_printing_it(self):
        with tempfile.TemporaryDirectory() as home:
            child = self.child(home, {'BBOXY_CAPABILITY': 'ab' * 32})
            with self.assertRaisesRegex(AssertionError, 'SharedDaemonEnvironmentIsCapabilityFree'):
                scan.scan_process(child.pid, {b'ab' * 32}, home, ['broken-owned-control'])

    def synthetic_scan_with_failed_second_bridge(self, root, recheck_pids, recheck_status=0, failure=None):
        """A healthy first bridge must not conceal an unreadable second bridge."""
        root = Path(root)
        home = root / 'home'
        home.mkdir()
        file = root / 'run/seat-fixture/overlays/launch-fixture/capability'
        file.parent.mkdir(parents=True)
        file.write_bytes(b'ab' * 32 + b'\n')
        binary = root / 'fixture-app'
        argv = {100: [b'codex', b'app-server'],
                200: [os.fsencode(binary), b'harness-exec', b'--codex-proxy'],
                300: [os.fsencode(binary), b'mcp'],
                301: [os.fsencode(binary), b'mcp']}

        def snapshot(pid):
            return argv[pid], {b'HOME': os.fsencode(home)}

        def output(command, **_):
            if command == ['/bin/ps', '-Eww', '-p', '301']:
                raise failure or subprocess.CalledProcessError(7, command)
            return b'private fixture metadata\n'

        def recheck(command, **_):
            self.assertEqual(command, ['/usr/bin/pgrep', '-f', '.*'])
            return subprocess.CompletedProcess(command, recheck_status, recheck_pids, b'')

        with patch.object(scan, 'descendants', side_effect=lambda pid: {300, 301} if pid == 100 else {200}), \
             patch.object(scan, 'process_snapshot', side_effect=snapshot), \
             patch.object(scan, 'stamp', return_value=(99, b'stable fixture start')), \
             patch.object(scan.subprocess, 'check_output', side_effect=output), \
             patch.object(scan.subprocess, 'run', side_effect=recheck):
            return scan.NativeScanner(root, binary).scan(100, 999, 'failed-second-bridge')

    def test_ps_failure_on_a_live_labelled_second_bridge_is_fatal(self):
        with tempfile.TemporaryDirectory() as root:
            with self.assertRaisesRegex(RuntimeError, 'pid=301.*mcp-bridge'):
                self.synthetic_scan_with_failed_second_bridge(root, b'100\n200\n300\n301\n')

    def test_proven_exits_are_counted_and_reported_with_roles(self):
        for reason, failure in [('absent-on-pgrep-recheck', None),
                                ('esrch', OSError(errno.ESRCH, 'owned fixture exited'))]:
            with self.subTest(reason=reason), tempfile.TemporaryDirectory() as root:
                row = self.synthetic_scan_with_failed_second_bridge(root, b'100\n200\n300\n', failure=failure)
                self.assertEqual(row['exited_count'], 1)
                self.assertEqual(row['exited_processes'], [dict(pid=301,
                    roles=['mcp-bridge', 'daemon-started-mcp-child'], reason=reason)])
                self.assertEqual(len(row['processes']), 3)

    def test_failed_exit_recheck_cannot_turn_a_scan_failure_into_an_exit(self):
        with tempfile.TemporaryDirectory() as root:
            with self.assertRaisesRegex(RuntimeError, 'pid=301.*mcp-bridge'):
                self.synthetic_scan_with_failed_second_bridge(root, b'', recheck_status=2)


if __name__ == '__main__':
    unittest.main()
