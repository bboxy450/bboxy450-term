#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Canonical prepared-bundle install + GUI-only reattach. Never invokes shell ctl."""
import argparse
import json
import os
from pathlib import Path
import plistlib
import select
import shutil
import signal
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parent.parent
APP = 'bboxy450-term'


def info(bundle):
    with (bundle / 'Contents/Info.plist').open('rb') as file:
        return plistlib.load(file)


def preservation_supported(bundle):
    # Static signed metadata: do not execute a legacy binary to ask whether it
    # understands a flag which that binary might ignore and launch normally.
    return info(bundle).get('BBOXYPreserveBackend') == 1


def executable(bundle):
    return bundle / 'Contents/MacOS' / APP


def probe(program, arguments):
    reply = subprocess.run([str(program), '--gui-handoff-probe', *arguments],
                           stdin=subprocess.DEVNULL, capture_output=True,
                           text=True, timeout=9)
    if reply.returncode:
        raise RuntimeError(f'GUI preservation probe refused: {reply.stderr[-4096:].strip()}')
    return json.loads(reply.stdout)


def ready(child, timeout=50):
    deadline = time.monotonic() + timeout
    stream = child.stdout
    os.set_blocking(stream.fileno(), False)
    data = bytearray()
    while time.monotonic() < deadline:
        if select.select([stream], [], [], min(0.1, deadline - time.monotonic()))[0]:
            block = os.read(stream.fileno(), 65537)
            if not block:
                raise RuntimeError(f'GUI exited before readiness: {child.poll()}')
            data.extend(block)
            if len(data) > 65536:
                raise RuntimeError('GUI readiness exceeded its byte bound')
            if b'\n' in data:
                value = json.loads(data.split(b'\n', 1)[0])
                if not value.get('backend_ready') or not value.get('gui_ready') or not value.get('frame_seq'):
                    raise RuntimeError('GUI did not produce a ready rendered frame')
                return value
    raise RuntimeError('GUI render readiness timed out')


def launch(program, arguments, log):
    with log.open('ab') as errors:
        return subprocess.Popen([str(program), '--app-ready', *arguments],
                                stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                                stderr=errors, start_new_session=True)


def close_owned(child):
    if child is None or child.poll() is not None:
        return
    # This exact unreaped child was started and recorded here. No process group.
    os.kill(child.pid, signal.SIGHUP)
    try:
        child.wait(timeout=35)
    except subprocess.TimeoutExpired as error:
        raise RuntimeError('candidate GUI did not detach; no force termination') from error


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--prepared', type=Path, required=True)
    parser.add_argument('--destination', type=Path, required=True)
    parser.add_argument('--run-dir', type=Path)
    parser.add_argument('--private', action='store_true')
    parser.add_argument('--headless', action='store_true')
    parser.add_argument('--refresh-logic', action='store_true', help='Allow the normal logic upgrade while retaining the live supervisor')
    parser.add_argument('--test-account', type=Path)
    parser.add_argument('--repository', type=Path)
    parser.add_argument('--verify-failure-after-detach', action='store_true')
    options = parser.parse_args()
    os.umask(0o077)
    prepared = options.prepared.resolve()
    destination_dir = options.destination.resolve()
    destination = destination_dir / f'{APP}.app'
    expected = info(prepared)['BBOXYSourceRevision']
    if not preservation_supported(prepared):
        raise RuntimeError('prepared bundle has no hard preserve-backend capability')
    signature_target = 'check-prepared-unsigned' if options.private else 'check-prepared'
    subprocess.run(['make', signature_target, f'BUNDLE={prepared}'], cwd=ROOT,
                   stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=True)
    if (not options.private and (len(expected) != 40 or any(c not in '0123456789abcdef' for c in expected))):
        raise RuntimeError('trusted GUI activation requires a clean source revision')
    if options.private and not (options.headless and options.test_account):
        raise RuntimeError('ad-hoc activation requires an explicit private headless test account')
    if options.verify_failure_after_detach and not options.private:
        raise RuntimeError('startup failure fixture requires private mode')
    run_args = ['--run-dir', str(options.run_dir.resolve())] if options.run_dir else []
    arguments = [*run_args]
    if options.headless:
        arguments.extend(['--headless', '--ctl', '--unattended'])
    if options.test_account:
        arguments.extend(['--test-account', str(options.test_account.resolve())])
    before = probe(executable(prepared), run_args)
    repository = str(options.repository.resolve()) if options.repository else before.get('repository')
    if before['running'] and not repository:
        raise RuntimeError('legacy GUI replacement requires explicit GUI_REPOSITORY')
    if repository:
        arguments.extend(['--repository', repository])
    preflight = probe(executable(prepared), arguments if options.refresh_logic else [*arguments, '--preserve-backend-check'])
    if preflight.get('preserve_backend_supported') != 1 or not preflight.get('backend'):
        raise RuntimeError('candidate cannot attest preserved backend compatibility')
    before['backend'] = preflight['backend']
    if not options.refresh_logic:
        arguments.extend(['--preserve-backend', '--preserve-backend-owners', json.dumps(preflight['backend'])])
    previous_revision = info(destination).get('BBOXYSourceRevision') if destination.exists() else None
    evidence = Path(tempfile.mkdtemp(prefix='gui-reopen-', dir=prepared.parent))
    install_target = 'install-prepared-unsigned' if options.private else 'install-prepared'
    variable = 'UNSIGNED_INSTALL_DIR' if options.private else 'INSTALL_DIR'
    command = ['make', install_target, f'BUNDLE={prepared}', f'{variable}={destination_dir}']
    installation = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
    (evidence / 'install.log').write_text(installation.stdout + installation.stderr)
    previous = None
    for line in installation.stdout.splitlines():
        if line.startswith('Previous contents, if any: '):
            previous = Path(line.removeprefix('Previous contents, if any: '))
    child = None
    try:
        if installation.returncode:
            raise RuntimeError(f'prepared install exited {installation.returncode}')
        if info(destination)['BBOXYSourceRevision'] != expected:
            raise RuntimeError('installed source revision does not match the prepared candidate')
        launch_arguments = [*arguments, '--replace-gui', '--replace-gui-owner', json.dumps(before['process'])]
        if options.verify_failure_after_detach:
            launch_arguments.append('--verify-ui-only-after-detach')
        child = launch(executable(destination), launch_arguments, evidence / 'gui.log')
        actual = ready(child, timeout=210 if options.refresh_logic else 50)
        gui = actual.get('gui') or {}
        if gui.get('pid') != child.pid or gui.get('source_revision') != expected:
            raise RuntimeError('rendered GUI did not attest the installed source and launched PID')
        after = probe(executable(prepared), arguments if options.refresh_logic else [*arguments, '--preserve-backend-check'])
        if options.refresh_logic:
            if (after.get('backend') or {}).get('supervisor') != preflight['backend']['supervisor']:
                raise RuntimeError('supervisor owner changed during logic/GUI replacement')
        elif after.get('backend') != preflight['backend']:
            raise RuntimeError('backend owners changed during GUI replacement')
        if not after['running'] or after['process']['pid'] != child.pid or after['gui'] != gui:
            raise RuntimeError('ready GUI does not hold the one current GUI lease')
        if (before.get('gui') or {}).get('instance') == gui.get('instance'):
            raise RuntimeError('GUI generation did not change')
        print(json.dumps({'ok': True, 'installed_revision': expected, 'before': before,
                          'after': after, 'ready': actual, 'previous_contents': str(previous),
                          'evidence': str(evidence)}))
        return 0
    except Exception as failure:
        rollback = None
        rollback_skipped = None
        restored = False
        close_error = None
        try:
            close_owned(child)
        except Exception as error:
            close_error = str(error)
        # Restore only the contents installed by this transaction. The canonical
        # installer already retains the old mapped image in its unique stage.
        current_revision = info(destination).get('BBOXYSourceRevision') if (destination / 'Contents').exists() else None
        observed = probe(executable(prepared), run_args)
        same_old_owner = observed.get('process') == before.get('process')
        may_restore = not observed['running'] or same_old_owner
        if previous and current_revision == expected and may_restore and not close_error:
            stage = previous.parent
            if stage.parent != destination_dir or not stage.name.startswith(f'.{APP}-install.'):
                raise RuntimeError('invalid canonical installer rollback receipt') from failure
            shutil.move(str(destination / 'Contents'), str(stage / 'failed-Contents'))
            if previous.is_dir():
                shutil.move(str(previous), str(destination / 'Contents'))
                subprocess.run(['codesign', '--verify', '--strict', str(destination)], check=True)
                if info(destination).get('BBOXYSourceRevision') != previous_revision:
                    raise RuntimeError('rollback did not restore the prior installed revision') from failure
            restored = True
            if previous_revision and before['running'] and not observed['running']:
                if preservation_supported(destination):
                    subprocess.run(['make', signature_target, f'BUNDLE={destination}'], cwd=ROOT,
                                   stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=True)
                    old = launch(executable(destination), arguments, evidence / 'rollback-gui.log')
                    rollback = {'pid': old.pid, 'ready': ready(old)}
                else:
                    rollback_skipped = 'restored legacy bundle lacks hard preservation; GUI left closed, backend retained'
        print(json.dumps({'ok': False, 'error': str(failure), 'close_error': close_error,
                          'restored': restored,
                          'installed_revision': info(destination).get('BBOXYSourceRevision') if (destination / 'Contents').exists() else None,
                          'rollback': rollback, 'rollback_skipped': rollback_skipped,
                          'concurrent_gui': observed if observed['running'] and not same_old_owner else None,
                          'evidence': str(evidence)}))
        return 2


if __name__ == '__main__':
    try:
        raise SystemExit(main())
    except Exception as error:
        print(json.dumps({'ok': False, 'error': str(error)}))
        raise SystemExit(2)
