# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Inspect only a private fixture's owned PIDs; retain counts, never raw values.

For a daemon/companion we cannot ask a live process to fork an arbitrary child.
Instead env serializes its actual KERN_PROCARGS2 snapshot, independently of ps.
Darwin appends Apple auxiliary entries to that region. They are retained too:
this conservative superset discards no exec environment entry.
HOME must match the private fixture: a restricted/redacted snapshot is a failure.
The parser independently implements the public Darwin argc/path/argv/env ABI:
https://github.com/apple-oss-distributions/xnu/blob/main/bsd/kern/kern_sysctl.c
No kernel implementation code is copied.
"""
import ctypes
import errno
import os
from pathlib import Path
import struct
import subprocess
import sys


def process_snapshot(pid):
    """Return argv and the exec environment/Apple region; no observer fallback."""
    if sys.platform != 'darwin':
        raise RuntimeError('native capability fixture requires Darwin process snapshots')
    libc = ctypes.CDLL(None, use_errno=True)
    sysctl = libc.sysctl
    sysctl.argtypes = [ctypes.POINTER(ctypes.c_int), ctypes.c_uint, ctypes.c_void_p,
                      ctypes.POINTER(ctypes.c_size_t), ctypes.c_void_p, ctypes.c_size_t]
    sysctl.restype = ctypes.c_int
    mib = (ctypes.c_int * 3)(1, 49, pid)  # CTL_KERN / KERN_PROCARGS2
    size = ctypes.c_size_t(os.sysconf('SC_ARG_MAX') + ctypes.sizeof(ctypes.c_int))
    buffer = ctypes.create_string_buffer(size.value)
    if sysctl(mib, len(mib), buffer, ctypes.byref(size), None, 0):
        raise OSError(ctypes.get_errno(), 'owned process snapshot unavailable')
    raw = buffer.raw[:size.value]
    argc, = struct.unpack_from('=i', raw)
    assert 0 < argc <= 16384, 'invalid owned process argc'
    cursor = raw.index(b'\0', 4) + 1  # bare executable path, then alignment
    while cursor < len(raw) and raw[cursor] == 0:
        cursor += 1
    argv = []
    for _ in range(argc):
        end = raw.index(b'\0', cursor)
        argv.append(raw[cursor:end])
        cursor = end + 1
    environment = {}
    while cursor < len(raw) and raw[cursor] != 0:
        end = raw.index(b'\0', cursor)
        key, separator, value = raw[cursor:end].partition(b'=')
        assert separator and key and key not in environment, 'invalid owned environment entry'
        environment[key] = value
        cursor = end + 1
    return argv, environment


def stamp(pid):
    result = subprocess.run(['/bin/ps', '-p', str(pid), '-o', 'uid=,ppid=,lstart='],
                            capture_output=True, check=True)
    fields = result.stdout.split(None, 2)
    assert len(fields) == 3 and int(fields[0]) == os.geteuid(), 'not an owned uid'
    return int(fields[1]), fields[2].strip()


def counts(output, values):
    return dict(value_matches=sum(output.count(value) for value in values),
                legacy_keys=output.count(b'BBOXY_CAPABILITY='))


def assert_clear(row):
    assert row['value_matches'] == row['legacy_keys'] == 0, \
        'SharedDaemonEnvironmentIsCapabilityFree: ' + str(row)


def scan_process(pid, values, home, roles):
    before = stamp(pid)
    _, environment = process_snapshot(pid)
    assert environment.get(b'HOME') == os.fsencode(home), \
        'owned HOME missing: restricted or redacted environment snapshot'
    sources = []
    ps = subprocess.check_output(['/bin/ps', '-Eww', '-p', str(pid)])
    kernel = b'\0'.join(key + b'=' + value for key, value in environment.items())
    # Refuse before serializing an unsafe snapshot into a new process environment.
    for name, output in [('ps-Eww', ps), ('kernel-environment', kernel)]:
        row = dict(source=name, **counts(output, values))
        assert_clear(row)
        sources.append(row)
    output = subprocess.check_output(['/usr/bin/env'], env=environment, timeout=10)
    row = dict(source='env-of-kernel-snapshot', **counts(output, values))
    assert_clear(row)
    sources.append(row)
    assert stamp(pid) == before, 'owned process replaced during environment scan'
    return dict(pid=pid, roles=roles, sources=sources)


def descendants(pid):
    result, pending = set(), [pid]
    while pending:
        parent = pending.pop()
        children = subprocess.run(['/usr/bin/pgrep', '-P', str(parent)], capture_output=True)
        assert children.returncode in (0, 1), 'owned process tree query failed'
        for child in map(int, children.stdout.split()):
            if child not in result:
                result.add(child)
                pending.append(child)
    return result


def proven_exit(pid, error):
    """Only ESRCH or a successful PID census can excuse missing observations."""
    if isinstance(error, OSError) and error.errno == errno.ESRCH:
        return 'esrch'
    result = subprocess.run(['/usr/bin/pgrep', '-f', '.*'], capture_output=True, timeout=10)
    if result.returncode not in (0, 1):
        raise RuntimeError('owned process exit recheck failed')
    if pid not in set(map(int, result.stdout.split())):
        return 'absent-on-pgrep-recheck'
    return None


class NativeScanner:
    """One private HOME/daemon; remember retired generation values only in RAM."""
    def __init__(self, root, binary):
        self.root, self.binary = Path(root), os.fsencode(Path(binary).resolve())
        self.values = set()
        self.observations = []

    def scan(self, daemon, app, stage):
        for path in (self.root / 'run').glob('seat-*/overlays/launch-*/capability'):
            value = path.read_bytes()
            assert len(value) == 65 and value[-1:] == b'\n', 'invalid owned admission file'
            self.values.add(value[:-1])
        assert self.values, 'no live seat admission values to scan'
        daemon_children = descendants(daemon)
        owned = daemon_children | descendants(app) | {daemon}
        for path in (self.root / 'run').glob('*.owner.json'):
            row = __import__('json').loads(path.read_text())
            pid = row.get('pid') or row.get('process', {}).get('pid')
            if type(pid) is int and pid > 1:
                owned |= descendants(pid)
        rows, exited = [], []
        for pid in sorted(owned):
            roles = ['shared-daemon'] if pid == daemon else []
            try:
                argv, _ = process_snapshot(pid)
                if argv and os.path.realpath(argv[0]) == self.binary:
                    if b'--codex-proxy' in argv:
                        roles.append('proxy-companion')
                    if len(argv) > 1 and argv[1] == b'mcp':
                        roles.append('mcp-bridge')
                        if pid in daemon_children:
                            roles.append('daemon-started-mcp-child')
                if roles:
                    rows.append(scan_process(pid, self.values, self.root / 'home', roles))
            except Exception as error:
                labels = roles or ['unclassified-owned']
                context = f'NativeCapabilityScanFailed pid={pid} roles={",".join(labels)}'
                # Never excuse an observed leak, bad HOME, parser error or a
                # replaced process as an exit. Only sampling failures qualify.
                if isinstance(error, (OSError, subprocess.CalledProcessError, subprocess.TimeoutExpired)):
                    try:
                        reason = proven_exit(pid, error)
                    except Exception:
                        raise RuntimeError(context + ' cause=exit-recheck-failed') from None
                    if reason:
                        exited.append(dict(pid=pid, roles=labels, reason=reason))
                        continue
                # Do not include raw subprocess output or environment bytes.
                raise RuntimeError(context + ' cause=' + type(error).__name__) from None
        roles = {role for row in rows for role in row['roles']}
        required = {'shared-daemon', 'proxy-companion', 'mcp-bridge', 'daemon-started-mcp-child'}
        observation = dict(stage=stage, distinct_admission_values=len(self.values), processes=rows,
                           exited_count=len(exited), exited_processes=exited)
        self.observations.append(observation)
        assert required <= roles, ('missing production process roles: ' + str(sorted(required - roles))
                                   + f'; exited_count={len(exited)} exited_processes={exited}')
        return observation
