"""Observational stage accounting (#333). Accounting errors never veto a command.

macOS time(1) supplies waited-for descendant rusage. Ten-second libproc samples
supply physical write lower bounds; block-operation counts are NOT byte counts.
No argv, environment, capability, transcript, or process command lines are stored.
"""
import argparse
import ctypes
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import sys
import time
import uuid

INTERVAL = 10.0
STAGES = ('build', 'unit', 'integration', 'smoke', 'e2e', 'parity', 'real-world')


def now():
    return datetime.now(timezone.utc).isoformat()


def state_dir():
    path = Path(os.environ.get('BBOXY_LANDING_STATE_DIR') or
                str(Path(os.environ.get('XDG_STATE_HOME') or Path.home() / '.local/state') /
                    'bboxy450-term/landing'))
    if not path.is_absolute():
        raise OSError('ledger state directory must be absolute')
    return path


def append(row):
    directory = state_dir() / 'ledger' / row['at'][:10]
    directory.mkdir(parents=row['kind'] != 'machine', exist_ok=True, mode=0o700)
    fd = os.open(directory / 'ledger.jsonl', os.O_WRONLY | os.O_CREAT | os.O_APPEND, 0o600)
    try:
        line = (json.dumps(row, separators=(',', ':')) + '\n').encode()
        if os.write(fd, line) != len(line):
            raise OSError('short ledger write')
    finally:
        os.close(fd)


def output(args, cwd=None):
    return subprocess.check_output(args, cwd=cwd, text=True, stderr=subprocess.DEVNULL,
                                   timeout=2).strip()


def target_size(path):
    try:
        return int(output(['du', '-sk', str(path)]).split()[0]) * 1024
    except (OSError, ValueError, subprocess.SubprocessError):
        return None


def parse_time(text):
    result = {}
    match = re.search(r'([\d.]+) real\s+([\d.]+) user\s+([\d.]+) sys', text)
    if match:
        result.update(cpu_user_seconds=float(match[2]), cpu_system_seconds=float(match[3]))
    for label, name in [('maximum resident set size', 'peak_rss_bytes'),
                        ('block output operations', 'block_output_operations')]:
        match = re.search(r'(\d+)\s+' + label, text)
        if match:
            result[name] = int(match[1])
    return result


class Usage(ctypes.Structure):
    _fields_ = [('uuid', ctypes.c_ubyte * 16)] + [(name, ctypes.c_uint64) for name in (
        'user', 'system', 'idle_wakes', 'interrupt_wakes', 'pageins', 'wired', 'rss',
        'footprint', 'start', 'exit', 'child_user', 'child_system', 'child_idle_wakes',
        'child_interrupt_wakes', 'child_pageins', 'child_elapsed', 'read', 'written')]


def mach_seconds_per_tick():
    # proc_pid_rusage CPU times use the host Mach timebase on Apple Silicon.
    # Calibrate with mach_timebase_info, never assume one tick is one ns.
    class Timebase(ctypes.Structure):
        _fields_ = [('numerator',ctypes.c_uint),('denominator',ctypes.c_uint)]
    value = Timebase()
    lib = ctypes.CDLL('/usr/lib/libSystem.B.dylib')
    if lib.mach_timebase_info(ctypes.byref(value)) != 0 or value.denominator == 0:
        raise OSError('Mach timebase unavailable')
    return value.numerator / value.denominator / 1e9


def processes():
    """Only numeric identity/accounting data, never ps args or other payloads."""
    if sys.platform != 'darwin':
        raise OSError('libproc accounting unavailable on this platform')
    lib = ctypes.CDLL('/usr/lib/libproc.dylib', use_errno=True)
    rows = {}
    seconds_per_tick = mach_seconds_per_tick()
    for line in output(['ps', '-axo', 'pid=,ppid=,pgid=']).splitlines():
        pid, parent, group = map(int, line.split())
        usage = Usage()
        if lib.proc_pid_rusage(pid, 2, ctypes.byref(usage)) == 0:
            rows[pid] = dict(pid=pid, parent=parent, pgid=group, start=usage.start,
                             user=usage.user * seconds_per_tick, system=usage.system * seconds_per_tick,
                             rss=usage.rss, written=usage.written)
    return rows


def process_start(pid):
    if sys.platform != 'darwin':
        return None
    usage = Usage()
    lib = ctypes.CDLL('/usr/lib/libproc.dylib')
    return usage.start if lib.proc_pid_rusage(pid, 2, ctypes.byref(usage)) == 0 else None


def bind_cache(worktree, port):
    """Called only after cache-env has selected and checked this listener."""
    import hashlib
    if not str(port).isdecimal() or not 0 < int(port) < 65536:
        return
    pids = set(output(['lsof', '-nP', '-iTCP:' + str(port), '-sTCP:LISTEN', '-t']).splitlines())
    if len(pids) != 1:
        return
    pid = int(pids.pop())
    start = process_start(pid)
    if start is None:
        return
    worktree = str(Path(worktree).resolve())
    directory = state_dir() / 'ledger-cache-bindings' / str(port)
    directory.mkdir(parents=True, exist_ok=True, mode=0o700)
    name = hashlib.sha256(os.fsencode(worktree)).hexdigest()
    temporary = directory / (name + '.' + uuid.uuid4().hex + '.tmp')
    data = dict(pid=pid, start=start, worktree=worktree, port=int(port), at=now(),
                basis='cache-env successful caller-selected worktree/port association')
    fd = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    try:
        os.write(fd, json.dumps(data).encode())
    finally:
        os.close(fd)
    os.replace(temporary, directory / (name + '.json'))


def cache_root(cwd, port=None, rows=None):
    """Missing, reused or conflicting bindings stay unattributed (#333)."""
    port = port or os.environ.get('SCCACHE_SERVER_PORT')
    if not port or not str(port).isdecimal():
        return None
    valid = []
    for path in (state_dir() / 'ledger-cache-bindings' / str(port)).glob('*.json'):
        try:
            value = json.loads(path.read_text())
            pid = value['pid']
            start = rows.get(pid, {}).get('start') if rows is not None else process_start(pid)
            if start is not None and start == value['start']:
                valid.append(value)
        except (OSError, ValueError, KeyError):
            continue
    if len(valid) == 1 and valid[0]['worktree'] == str(Path(cwd).resolve()):
        return dict(pid=valid[0]['pid'], start=valid[0]['start'], basis=valid[0]['basis'])
    return None


def cache_totals(cwd):
    if os.environ.get('RUSTC_WRAPPER') != 'sccache':
        return None
    root = cache_root(cwd)
    if root is None:
        return None
    usage = Usage()
    lib = ctypes.CDLL('/usr/lib/libproc.dylib')
    if lib.proc_pid_rusage(root['pid'], 2, ctypes.byref(usage)) != 0 or usage.start != root['start']:
        return None
    factor = mach_seconds_per_tick()
    return dict(pid=root['pid'], start=root['start'],
                user=(usage.user + usage.child_user) * factor,
                system=(usage.system + usage.child_system) * factor)


def machine():
    result = dict(load=list(os.getloadavg()), disk_free_bytes=shutil.disk_usage(state_dir()).free)
    if sys.platform == 'darwin':
        lib = ctypes.CDLL('/usr/lib/libSystem.B.dylib')
        ticks = (ctypes.c_uint * 4)()
        count = ctypes.c_uint(4)
        lib.mach_host_self.restype = ctypes.c_uint
        if lib.host_statistics(lib.mach_host_self(), 3, ticks, ctypes.byref(count)) == 0:
            result['cpu_busy_seconds'] = (ticks[0] + ticks[1] + ticks[3]) / os.sysconf('SC_CLK_TCK')
        vm = output(['vm_stat'])
        page = int(re.search(r'page size of (\d+) bytes', vm)[1])
        result['memory_free_bytes'] = int(re.search(r'Pages free:\s+(\d+)', vm)[1]) * page
        swap = output(['sysctl', '-n', 'vm.swapusage'])
        value, unit = re.search(r'used = ([\d.]+)([KMG])', swap).groups()
        result['swap_used_bytes'] = int(float(value) * 1024 ** ('KMG'.index(unit) + 1))
    return result


def descendants(root, rows):
    owned = {root} if root in rows else set()
    while True:
        more = {pid for pid, row in rows.items() if row['parent'] in owned}
        if more <= owned:
            return owned
        owned |= more


def sample(registrations, previous):
    rows = processes()
    charges = {}
    # The deepest explicitly recorded root wins. A nested invariant is never
    # charged a second time to an enclosing wrapper.
    owners = {}
    expanded = []
    for registration in registrations:
        expanded.append(registration)
        if registration['pid'] in rows and rows[registration['pid']]['start'] == registration['start']:
            expanded.extend(dict(registration, **root) for root in registration.get('external_roots', []))
            if registration.get('cache_port'):
                root = cache_root(registration['worktree'], registration['cache_port'], rows)
                if root:
                    expanded.append(dict(registration, **root))
    for registration in expanded:
        pid = registration['pid']
        if pid not in rows or rows[pid]['start'] != registration['start']:
            continue
        owned = descendants(pid, rows)
        for child in owned:
            old = owners.get(child)
            if old is None or pid in descendants(old['pid'], rows):
                owners[child] = registration
    for pid, row in rows.items():
        before = previous.get((pid, row['start']))
        owner = owners.get(pid)
        key = owner['id'] if owner else 'unattributed'
        charge = charges.setdefault(key, dict(id=key, agent=owner['agent'] if owner else 'unattributed',
            cut=owner['cut'] if owner else None, stage=owner['stage'] if owner else 'unattributed',
            cpu_user_seconds=0., cpu_system_seconds=0., rss_bytes=0,
            disk_bytes_written=0, pids=[]))
        charge['rss_bytes'] += row['rss']
        charge['pids'].append(pid)
        if before:
            for source, target in [('user', 'cpu_user_seconds'), ('system', 'cpu_system_seconds'),
                                   ('written', 'disk_bytes_written')]:
                charge[target] += max(0, row[source] - before[source])
    return list(charges.values()), {(p, v['start']): v for p, v in rows.items()}


def observe(owner, registration_dir, session):
    previous = {}
    previous_busy = None
    totals = {}
    owner_start = process_start(owner)
    while True:
        try:
            os.kill(owner, 0)
        except ProcessLookupError:
            return
        if sys.platform == 'darwin' and process_start(owner) != owner_start:
            return
        started = time.monotonic()
        row = dict(kind='machine', at=now(), monotonic=time.monotonic(), sampler=session, interval_seconds=INTERVAL,
                   sampling='live_process_deltas; exited_between_samples_unattributed',
                   cpu_time_basis='host Mach ticks times mach_timebase_info; machine host CPU load ticks')
        try:
            row.update(machine())
            registrations = []
            for path in registration_dir.glob('*.json'):
                try:
                    registrations.append(json.loads(path.read_text()))
                except (OSError, ValueError):
                    pass
            row['groups'], previous = sample(registrations, previous)
            busy = row.get('cpu_busy_seconds')
            assigned = sum(g['cpu_user_seconds'] + g['cpu_system_seconds']
                           for g in row['groups'] if g['id'] != 'unattributed')
            row['unattributed_cpu_seconds'] = (max(0, busy - previous_busy - assigned)
                                               if busy is not None and previous_busy is not None else None)
            previous_busy = busy
            for group in row['groups']:
                if group['id'] == 'unattributed':
                    continue
                total = totals.setdefault(group['id'], dict(disk_bytes_written=0, peak_group_rss_bytes=0, samples=0))
                total['disk_bytes_written'] += group['disk_bytes_written']
                total['peak_group_rss_bytes'] = max(total['peak_group_rss_bytes'], group['rss_bytes'])
                total['samples'] += 1
                path = registration_dir / (group['id'] + '.sample')
                path.write_text(json.dumps(total))
                path.chmod(0o600)
        except Exception as error:
            row['unavailable'] = type(error).__name__
        try:
            append(row)
        except Exception:
            pass
        time.sleep(max(0, INTERVAL - (time.monotonic() - started)))


def start_observer(owner):
    if owner <= 1:
        raise ValueError('observer requires an owned process')
    (state_dir() / 'ledger' / now()[:10]).mkdir(parents=True, exist_ok=True, mode=0o700)
    directory = state_dir() / 'ledger-processes'
    directory.mkdir(parents=True, exist_ok=True, mode=0o700)
    child = subprocess.Popen([sys.executable, '-I', __file__, 'observe',
         str(owner), str(directory), uuid.uuid4().hex], stdin=subprocess.DEVNULL,
         stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, start_new_session=True)
    return child.pid


class Stage:
    def __init__(self, stage, cwd):
        self.at, self.started, self.id = now(), time.monotonic(), uuid.uuid4().hex
        self.stage, self.cwd = stage, Path(cwd)
        self.directory = state_dir() / 'ledger-processes'
        self.directory.mkdir(parents=True, exist_ok=True, mode=0o700)
        self.file = self.directory / (self.id + '.json')
        self.reader = self.writer = None
        if sys.platform == 'darwin' and Path('/usr/bin/time').exists():
            self.reader, self.writer = os.pipe()
            os.set_blocking(self.reader, False)
        self.agent = os.environ.get('BBOXY_LEDGER_AGENT') or 'worktree:' + str(self.cwd.resolve())
        try:
            self.cut = dict(branch=os.environ.get('BBOXY_LEDGER_BRANCH') or output(['git', 'branch', '--show-current'], self.cwd),
                            sha=os.environ.get('BBOXY_LEDGER_SHA') or output(['git', 'rev-parse', 'HEAD'], self.cwd))
        except Exception:
            self.cut = dict(branch=None, sha=None)
        self.target = Path(os.environ.get('CARGO_TARGET_DIR') or
                           os.environ.get('BBOXY_CARGO_TARGET_DIR') or self.cwd / 'target')
        self.child = None
        self.cache_before = cache_totals(self.cwd) if sys.platform == 'darwin' else None

        sampler = os.environ.get('BBOXY_LEDGER_SAMPLER')
        if sampler:
            try:
                os.kill(int(sampler), 0)
                return
            except (OSError, ValueError):
                pass
        # Observers are never awaited by the workload or used as readiness gates.
        try:
            os.environ['BBOXY_LEDGER_SAMPLER'] = str(start_observer(os.getpid()))
        except OSError as error:
            print('ledger: sampler unavailable (' + type(error).__name__ + '); continuing', file=sys.stderr)

    def wrap(self, argv):
        if sys.platform == 'darwin' and Path('/usr/bin/time').exists():
            return ['/usr/bin/time', '-l', '-o', '/dev/fd/' + str(self.writer), *map(str, argv)]
        return list(map(str, argv))

    def popen_options(self):
        return {'pass_fds': (self.writer,)} if self.writer is not None else {}

    def attach(self, pid):
        self.child = pid
        if self.writer is not None:
            os.close(self.writer)
            self.writer = None
        usage = Usage()
        start = None
        if sys.platform == 'darwin':
            lib = ctypes.CDLL('/usr/lib/libproc.dylib')
            if lib.proc_pid_rusage(pid, 2, ctypes.byref(usage)) == 0:
                start = usage.start
        data = dict(id=self.id, agent=self.agent, cut=self.cut, stage=self.stage,
                    pid=pid, pgid=os.getpgid(pid), start=start, owner=os.getpid(),
                    worktree=str(self.cwd.resolve()),
                    cache_port=os.environ.get('SCCACHE_SERVER_PORT') if
                    os.environ.get('RUSTC_WRAPPER') == 'sccache' else None)
        self.file.write_text(json.dumps(data))
        self.file.chmod(0o600)

    def finish(self, code):
        row = dict(kind='stage', id=self.id, at=self.at, ended=now(), agent=self.agent,
                   cut=self.cut, stage=self.stage, wall_seconds=time.monotonic() - self.started,
                   cpu_user_seconds=None, cpu_system_seconds=None, peak_rss_bytes=None,
                   disk_bytes_written=None, disk_accounting='unavailable; sampled deltas in machine rows',
                   target_bytes_after=target_size(self.target), target_accounting='du allocated bytes',
                   exit=code, pid=self.child,
                   rusage='time-l waited descendant usage; max individual RSS; compile included in cargo test')
        try:
            stats = os.read(self.reader, 65536).decode() if self.reader is not None else ''
            row.update(parse_time(stats))
        except OSError:
            row['rusage'] = 'unavailable'
        after = cache_totals(self.cwd) if self.cache_before else None
        row['cache_cpu_accounting'] = 'unavailable; no stable selected cache binding'
        if (after and (after['pid'], after['start']) ==
                (self.cache_before['pid'], self.cache_before['start'])):
            row['cache_cpu_accounting'] = 'selected worktree cache self+reaped-child delta; sequential stage interval'
            for kind in ('user', 'system'):
                delta = after[kind] - self.cache_before[kind]
                row['cache_cpu_' + kind + '_seconds'] = max(0., delta)
                key = 'cpu_' + kind + '_seconds'
                row['waited_' + key] = row[key]
                if row[key] is not None:
                    row[key] += max(0., delta)
        sample_path = self.directory / (self.id + '.sample')
        try:
            sampled = json.loads(sample_path.read_text())
            row.update(sampled)
            row['disk_accounting'] = 'sampled physical write lower bound; short-lived children can be missed'
        except (OSError, ValueError):
            pass
        try:
            append(row)
        finally:
            self.file.unlink(missing_ok=True)
            if self.reader is not None:
                os.close(self.reader)
            sample_path.unlink(missing_ok=True)


def landed(value):
    append(dict(kind='landing',at=now(),agent=os.environ.get('BBOXY_LEDGER_AGENT','unattributed'),
                branch=value['branch'],sha=value['sha'],tested_sha=value['tested_sha'],
                checked_shas=[r['sha'] for r in value.get('check_timings',[])],
                basis='successful author landing; earlier unmatched cuts remain unlanded'))


def begin(stage, cwd):
    try:
        return Stage(stage, cwd)
    except Exception as error:
        print('ledger: unavailable (' + type(error).__name__ + '); continuing', file=sys.stderr)
        return None


def attempt(stage, method, *args):
    if stage is not None:
        try:
            return getattr(stage, method)(*args)
        except Exception as error:
            print('ledger: unavailable (' + type(error).__name__ + '); continuing', file=sys.stderr)
    return None


def run(stage, argv):
    accounting = begin(stage, Path.cwd())
    wrapped = attempt(accounting, 'wrap', argv) or argv
    child = subprocess.Popen(wrapped, **(attempt(accounting, 'popen_options') or {}))
    attempt(accounting, 'attach', child.pid)
    def forward(sig, frame):
        # time(1) adds one parent. Signal only descendants of this recorded
        # command, never its inherited group (which can contain the caller).
        try:
            rows = {int(p): dict(parent=int(parent)) for p,parent in
                    (line.split() for line in output(['ps','-axo','pid=,ppid=']).splitlines())}
            owned = descendants(child.pid, rows)
        except Exception:
            owned = {child.pid}
        for pid in sorted(owned, reverse=True):
            try:
                os.kill(pid, sig)
            except ProcessLookupError:
                pass
    for sig in (signal.SIGINT, signal.SIGTERM):
        signal.signal(sig, forward)
    code = child.wait()
    attempt(accounting, 'finish', code)
    return code if code >= 0 else 128 - code


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='verb', required=True)
    p = sub.add_parser('run')
    p.add_argument('stage', choices=STAGES)
    p.add_argument('command', nargs=argparse.REMAINDER)
    p = sub.add_parser('start')
    p.add_argument('owner', type=int)
    p = sub.add_parser('bind-cache')
    p.add_argument('worktree', type=Path)
    p.add_argument('port')
    p = sub.add_parser('observe')
    p.add_argument('owner', type=int)
    p.add_argument('directory', type=Path)
    p.add_argument('session')
    args = parser.parse_args()
    if args.verb == 'bind-cache':
        try:
            bind_cache(args.worktree, args.port)
        except Exception:
            pass  # The caller's selected cache is never gated by accounting.
        return 0
    if args.verb == 'start':
        try:
            print(start_observer(args.owner))
        except Exception as error:
            print('ledger: sampler unavailable (' + type(error).__name__ + '); continuing', file=sys.stderr)
        return 0
    if args.verb == 'observe':
        observe(args.owner, args.directory, args.session)
        return 0
    argv = args.command[1:] if args.command[:1] == ['--'] else args.command
    if not argv:
        parser.error('command required')
    return run(args.stage, argv)


if __name__ == '__main__':
    sys.exit(main())
