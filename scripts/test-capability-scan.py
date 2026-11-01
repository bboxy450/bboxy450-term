# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Real owned children prove snapshots preserve argv/env and detect a leak."""
import importlib.util
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

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


if __name__ == '__main__':
    unittest.main()
