#!/usr/bin/env python3
"""CLI-only live captures. Never injects OS input or captures the desktop."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shlex
import shutil
import signal
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[3]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def process_rows():
    result = subprocess.run(['ps', '-axo', 'pid=,ppid=,command='], capture_output=True, text=True, check=True)
    return [line.strip().split(None, 2) for line in result.stdout.splitlines()]


def record_descendants(app, owned):
    rows = process_rows()
    changed = True
    while changed:
        changed = False
        for row in rows:
            if len(row) != 3:
                continue
            pid, parent, command = int(row[0]), int(row[1]), row[2]
            if pid == app or parent in owned:
                if pid not in owned:
                    changed = True
                owned[pid] = {'parent': parent, 'command': command}


def stop_owned(app, owned):
    """Signal only recorded descendants, rechecking command identity per PID."""
    actions = []
    for sig, delay in [(signal.SIGTERM, 2), (signal.SIGKILL, 1)]:
        # Children first; app last. Never signal a process group or a name.
        for pid in [p for p in reversed(owned) if p != app.pid] + [app.pid]:
            check = subprocess.run(['ps', '-p', str(pid), '-o', 'command='], capture_output=True, text=True)
            if check.returncode or check.stdout.strip() != owned[pid]['command']:
                continue
            try:
                os.kill(pid, sig)
                actions.append({'pid': pid, 'signal': sig.name})
            except ProcessLookupError:
                pass
        end = time.monotonic() + delay
        while time.monotonic() < end:
            if app.poll() is not None:
                break
            time.sleep(.05)
    app.wait(timeout=2)
    remaining = []
    for pid, old in owned.items():
        check = subprocess.run(['ps', '-p', str(pid), '-o', 'stat=,command='], capture_output=True, text=True)
        if check.returncode == 0:
            remaining.append({'pid': pid, 'state': check.stdout.strip(), 'recorded_command': old['command']})
    return actions, remaining


def capture(args, scene, theme):
    out = args.out / f'{scene}-{theme}-1x'
    out.mkdir()  # Refuse to overwrite previous evidence.
    runtime = Path(tempfile.mkdtemp(prefix='t2-aster-'))  # macOS SUN_LEN needs a short path.
    zd = out / 'zsh'
    zd.mkdir()
    (zd / '.zshrc').write_text("PROMPT=''\nRPROMPT=''\nunset HISTFILE\n")
    png = out / 'capture.png'
    argv = [str(args.app), '--run-dir', str(runtime), '--spawn-zsh', '1',
            '--capture', str(png), '--capture-after', str(args.after),
            '--theme', theme, '--grid-size', '80x24', '--grid-hash']
    if args.pacing:
        argv += ['--pacing', args.pacing]
    # Query replies are PTY input; keep line-discipline echo from turning them
    # into extra visible fixture bytes.
    command = f'stty -echo; {shlex.quote(str(args.firehose))} --scene {shlex.quote(scene)} --seed 42 --rate max; exec /bin/sleep 600\\n'
    expected = subprocess.run([str(args.firehose), '--scene', scene, '--seed', '42', '--check'],
                              capture_output=True, text=True, check=True).stdout.strip().split('fnv1a64=')[1]
    record = {'scene': scene, 'theme': theme, 'path': 'live1x', 'argv': argv,
              'fixture_command': command, 'runtime': str(runtime), 'expected': expected,
              'app_sha256': digest(args.app), 'firehose_sha256': digest(args.firehose)}
    owned = {}
    with (out / 'app.log').open('w') as log:
        app = subprocess.Popen(argv, cwd=ROOT, env={**os.environ, 'BBOXY_TEST_CUSTODY': 'memory',
                               'ZDOTDIR': str(zd), 'BBOXY_VERIFY_INPUT': command},
                               stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
        record['app_pid'] = app.pid
        owned[app.pid] = {'parent': os.getpid(), 'command': shlex.join(argv)}
        start = time.monotonic()
        try:
            while time.monotonic() - start < args.timeout:
                record_descendants(app.pid, owned)
                record['owned'] = owned
                (out / 'process.json').write_text(json.dumps(record, indent=2) + '\n')
                if app.poll() is not None:
                    break
                if png.exists() and png.with_suffix('.grid.json').exists():
                    captures = (out / 'app.log').read_text().count(f'capture {png}: Ok(())')
                    if captures >= 2 and time.monotonic() - start >= args.after + 1:
                        break
                time.sleep(.2)
            record_descendants(app.pid, owned)
            if png.exists() and png.with_suffix('.grid.json').exists():
                grid = json.loads(png.with_suffix('.grid.json').read_text())
                record['grid'] = grid
                record['png_sha256'] = digest(png)
                record['match_expected'] = (len(grid['panes']) == 1 and
                    grid['panes'][0]['cols'] == 80 and grid['panes'][0]['rows'] == 24 and
                    grid['panes'][0]['fnv1a64'] == expected)
            else:
                record['match_expected'] = False
                record['error'] = 'capture-or-grid-metadata-missing'
        finally:
            actions, remaining = stop_owned(app, owned)
            record.update(owned=owned, cleanup_signals=actions, remaining_pids=remaining,
                          app_exit=app.returncode, elapsed_seconds=time.monotonic()-start)
            if not remaining:
                shutil.rmtree(runtime)  # Only the temporary directory created above.
                record['runtime_removed'] = True
            (out / 'process.json').write_text(json.dumps(record, indent=2) + '\n')
    print(json.dumps({'case': out.name, 'match_expected': record['match_expected'],
                      'app_exit': record['app_exit'], 'remaining_pids': record['remaining_pids']}), flush=True)
    return record


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--app', type=Path, default=ROOT / 'target/debug/bboxy450-term')
    parser.add_argument('--firehose', type=Path, default=ROOT / 'target/release/bboxy450-term-firehose')
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--scene', action='append')
    parser.add_argument('--theme', choices=['dark', 'light', 'both'], default='both')
    parser.add_argument('--after', type=float, default=4)
    parser.add_argument('--timeout', type=float, default=16)
    parser.add_argument('--pacing', choices=['fifo', 'metal'])
    args = parser.parse_args()
    args.app, args.firehose, args.out = args.app.resolve(), args.firehose.resolve(), args.out.resolve()
    # The memory selector is compiled out of plain release builds. Never launch
    # one merely because the environment variable was set.
    binary = args.app.read_bytes()
    if b'BBOXY_TEST_CUSTODY' not in binary or b'memory custody (debug)' not in binary:
        parser.error('app-without-memory-custody: use debug or release with debug assertions')
    del binary
    args.out.mkdir(exist_ok=True)
    names = [line.split('\t')[0] for line in subprocess.run([str(args.firehose), '--list'],
             capture_output=True, text=True, check=True).stdout.splitlines()]
    scenes = args.scene or names
    if not all(scene in names for scene in scenes):
        parser.error('unknown scene')
    themes = ['dark', 'light'] if args.theme == 'both' else [args.theme]
    results = [capture(args, scene, theme) for theme in themes for scene in scenes]
    (args.out / 'manifest.json').write_text(json.dumps({'source': subprocess.run(
        ['git', 'rev-parse', 'HEAD'], cwd=ROOT, capture_output=True, text=True, check=True).stdout.strip(),
        'driver_sha256': digest(Path(__file__)), 'cases': results}, indent=2) + '\n')
    return int(any(not row['match_expected'] or row['remaining_pids'] for row in results))


if __name__ == '__main__':
    raise SystemExit(main())
