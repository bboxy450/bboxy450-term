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
import select
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import call, patch

spec = importlib.util.spec_from_file_location('capability_scan', Path(__file__).parent / 'e2e/capability_scan.py')
scan = importlib.util.module_from_spec(spec)
spec.loader.exec_module(scan)


class ProcessScanTests(unittest.TestCase):
    def start_child(self, home, extra=None, startup=''):
        command = [sys.executable, '-I', '-B', '-c', startup +
                   'import os; os.write(1, b"ready\\n"); os.read(0, 1)', 'space = arg', '']
        child = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                 env=dict(HOME=home, FIXTURE_MARKER='spaces = and equals', **(extra or {})))

        def cleanup():
            if child.poll() is None:
                child.terminate()
            child.wait(timeout=10)
            child.stdin.close()
            child.stdout.close()

        self.addCleanup(cleanup)
        return child

    def wait_for_marker(self, child, marker):
        self.assertTrue(select.select([child.stdout], [], [], 10)[0], 'owned child readiness timeout')
        self.assertEqual(child.stdout.readline(), marker, 'owned child failed before readiness')

    def child(self, home, extra=None):
        child = self.start_child(home, extra)
        self.wait_for_marker(child, b'ready\n')
        return child

    def test_actual_kernel_environment_is_serialized_without_inheriting_the_runner(self):
        with tempfile.TemporaryDirectory() as home:
            child = self.child(home)
            argv, env = scan.process_snapshot(child.pid)
            self.assertEqual(argv[-2:], [b'space = arg', b''])
            self.assertEqual(env[b'FIXTURE_MARKER'], b'spaces = and equals')
            self.assertEqual(env[b'HOME'], os.fsencode(home))
            # KERN_PROCARGS2 also includes Darwin's auxiliary string vector and
            # the macOS Python launcher's additions after its final exec.
            # Keeping them is conservative; no arbitrary runner variable is used.
            apple = {b'main_stack', b'executable_cdhash', b'security_config', b'pfz',
                     b'stack_guard', b'executable_file', b'malloc_entropy', b'ptr_munge',
                     b'executable_boothash', b'dyld_file', b'th_port',
                     b'__CF_USER_TEXT_ENCODING', b'__PYVENV_LAUNCHER__'}
            self.assertLessEqual(set(env), {b'HOME', b'FIXTURE_MARKER'} | apple)
            row = scan.scan_process(child.pid, {b'ab' * 32}, home, ['owned-control'])
            self.assertEqual([s['source'] for s in row['sources']],
                             ['ps-Eww', 'kernel-environment', 'env-of-kernel-snapshot'])

    def test_real_legacy_value_is_detected_without_printing_it(self):
        with tempfile.TemporaryDirectory() as home:
            child = self.child(home, {'BBOXY_CAPABILITY': 'ab' * 32})
            with self.assertRaisesRegex(AssertionError, 'SharedDaemonEnvironmentIsCapabilityFree') as failure:
                scan.scan_process(child.pid, {b'ab' * 32}, home, ['broken-owned-control'])
            self.assertNotIn('ab' * 32, str(failure.exception))

    def test_omitted_readiness_wait_reproduces_real_kernel_eio(self):
        # Gate one argument page before readiness to deterministically reproduce
        # Darwin's unreadable exec-map EIO, without racing the scheduler. This
        # independently exercises the public ABI; no XNU source is copied.
        startup = '''import ctypes, os
libc = ctypes.CDLL(None, use_errno=True)
libc._NSGetEnviron.restype = ctypes.POINTER(ctypes.POINTER(ctypes.c_void_p))
environment = libc._NSGetEnviron()[0]
index = 0
while not ctypes.string_at(environment[index]).startswith(b'FIXTURE_PADDING='):
    index += 1
page = os.sysconf('SC_PAGESIZE')
address = (environment[index] + 32768) // page * page
libc.mprotect.argtypes = [ctypes.c_void_p, ctypes.c_size_t, ctypes.c_int]
assert libc.mprotect(address, page, 0) == 0
os.write(1, b'starting\\n')
os.read(0, 1)
assert libc.mprotect(address, page, 3) == 0
'''
        with tempfile.TemporaryDirectory() as home:
            child = self.start_child(home, {'FIXTURE_PADDING': 'x' * 65536}, startup)
            self.wait_for_marker(child, b'starting\n')
            # Broken control: scan before waiting for ready. Expire the retry
            # clock immediately so the 200-run regression need not sleep here.
            with patch.object(scan.time, 'monotonic', side_effect=[0, 1]), \
                 self.assertRaises(OSError) as failure:
                scan.scan_process(child.pid, {b'ab' * 32}, home, ['unready-owned-control'])
            self.assertEqual(failure.exception.errno, errno.EIO)
            child.stdin.write(b'x')
            child.stdin.flush()
            self.wait_for_marker(child, b'ready\n')
            row = scan.scan_process(child.pid, {b'ab' * 32}, home, ['ready-owned-control'])
            self.assertTrue(row['sources'])

    def test_transient_snapshot_eio_retries_to_an_observation(self):
        observation = ([b'owned'], {b'HOME': b'private'})
        with patch.object(scan, '_process_snapshot_once', side_effect=[
                OSError(errno.EIO, 'owned exec map transitioning'), observation]) as snapshot, \
             patch.object(scan.time, 'sleep') as sleep:
            self.assertEqual(scan.process_snapshot(123), observation)
        self.assertEqual(snapshot.call_args_list, [call(123)] * 2)
        sleep.assert_called_once_with(0.01)

    def test_persistent_snapshot_eio_and_other_errors_stay_fatal(self):
        for code in (errno.EIO, errno.EACCES, errno.ESRCH):
            with self.subTest(code=code), \
                 patch.object(scan, '_process_snapshot_once', side_effect=OSError(code, 'owned failure')), \
                 patch.object(scan.time, 'monotonic', side_effect=[0, 1]), \
                 patch.object(scan.time, 'sleep') as sleep, \
                 self.assertRaises(OSError) as failure:
                scan.process_snapshot(123)
            self.assertEqual(failure.exception.errno, code)
            sleep.assert_not_called()

    def synthetic_scan_with_failed_second_bridge(self, root, recheck_pids, recheck_status=0,
                                                failure=None, snapshot_failure=None):
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

        observed = set()

        def snapshot(pid):
            if pid == 301 and pid in observed and snapshot_failure:
                raise snapshot_failure
            observed.add(pid)
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

    def test_snapshot_eio_on_a_live_labelled_second_bridge_is_fatal(self):
        with tempfile.TemporaryDirectory() as root:
            with self.assertRaisesRegex(RuntimeError, 'pid=301.*mcp-bridge.*cause=OSError'):
                self.synthetic_scan_with_failed_second_bridge(root, b'100\n200\n300\n301\n',
                        snapshot_failure=OSError(errno.EIO, 'owned snapshot failed'))

    def test_proven_exits_are_counted_and_reported_with_roles(self):
        for reason, failure in [('absent-on-pgrep-recheck', None),
                                ('absent-on-pgrep-recheck', OSError(errno.EIO, 'owned snapshot failed')),
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
