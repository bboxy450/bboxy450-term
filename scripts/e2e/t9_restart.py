"""Explicit seat restart: real interactive clients retain sessions and transcript.

Model SSE is loopback-scripted; only the native clients handle sessions. No
provider handler calls app tools or fabricates a session id or ctl verdict.
"""
import hashlib
import http.server
import io
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import threading
import time

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'scripts/e2e'))
import native_checks as native
from native_turn import wait_codex_turn
from ui import App, require


def objects(value):
    if isinstance(value, dict):
        yield value
        for child in value.values():
            yield from objects(child)
    elif isinstance(value, list):
        for child in value:
            yield from objects(child)


class Provider(native.Provider):
    observations = []
    phase = 'before'

    def do_POST(self):
        raw = self.rfile.read(int(self.headers['Content-Length']))
        body = json.loads(raw)
        self.observations.append(dict(
            kind='codex' if self.path.endswith('/responses') else 'claude',
            phase=self.phase, path=self.path,
            assistant_before=any(x.get('role') == 'assistant' and 'T9_BEFORE' in json.dumps(x)
                                 for x in objects(body)),
            request_sha256=hashlib.sha256(raw).hexdigest()))
        self.rfile = io.BytesIO(raw)
        super().do_POST()


def until(read, ready, description, seconds=45):
    deadline = time.monotonic() + seconds
    while True:
        try:
            value = read()
            if ready(value):
                return value
        except AssertionError as error:
            value = str(error)
        require(time.monotonic() < deadline, f'{description}: {value}')
        time.sleep(.05)


def run():
    app = App()
    work = app.work
    config = work / 'l3-native-provider.json'
    server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Provider)
    worker = threading.Thread(target=server.serve_forever, daemon=True)
    worker.start()
    config.write_text(json.dumps(dict(port=server.server_port, interactive=True)))
    report = dict(harness='installed interactive Claude and Codex; real zsh',
                  model='scripted loopback; no remote model claim', before={}, after={})
    def roster():
        return {r['record']['id']: r for r in app.ctl('seat', 'ls')}
    try:
        # Native onboarding is fixture setup, not an operator dependency. Trust
        # only this run's own empty work directory in its isolated harness home.
        home = work.parent / 'home'
        require(Path(os.environ['HOME']).resolve() == home.resolve() and
                (home / '.l3-private-home').is_file(), 'private harness home required')
        preferences = home / '.claude/.claude.json'
        private = json.loads(preferences.read_text())
        private.setdefault('projects', {})[str(work)] = {'hasTrustDialogAccepted': True}
        preferences.write_text(json.dumps(private, indent=2) + '\n')
        preferences.chmod(0o600)
        native.MARKER = 'T9_BEFORE'
        claude = app.ctl('seat', 'spawn', 'claude', '--role', 'executor', '--place', 'tab',
                         '--cwd', work, '--name', 'T9 native Claude')['seat']
        app.wait(claude, 'T9_BEFORE')
        codex = app.ctl('seat', 'spawn', 'codex', '--role', 'executor', '--place', 'tab',
                        '--cwd', work, '--name', 'T9 native Codex')['seat']
        wait_codex_turn(app, codex, 'T9_BEFORE', 't9')
        shell = app.spawn('T9 zsh')
        app.line(shell, "printf 'T9_ZSH_BEFORE %s\\n' $$")
        app.wait(shell, '(?m)^T9_ZSH_BEFORE [0-9]+$')
        ids = [claude, codex, shell]
        before = until(roster, lambda xs: all(xs[s]['state']['process']['kind'] == 'running' for s in ids)
                       and all(xs[s]['record']['harness_session_id'] for s in [claude, codex]),
                       'live native sessions bound')
        report['before'] = {s: before[s] for s in ids}
        report['capture_through'] = {}
        for seat in ids:
            layout = app.ctl('ui', 'focus', seat)
            app.capture(seat, 't9-before-' + seat, layout['frame_seq'])
            report['capture_through'][seat] = app.ctl('seat', 'screen', seat)['sequence']
        native.MARKER = 'T9_AFTER'
        Provider.phase = 'after'
        report['restarts'] = {}
        for seat in ids:
            reply = app.ctl('seat', 'restart', seat)
            require(reply['from_generation'] == before[seat]['record']['generation'],
                    'restart acknowledged the wrong generation')
            report['restarts'][seat] = reply
        after = until(roster, lambda xs: all(xs.get(s, {}).get('record', {}).get('generation') ==
                                            before[s]['record']['generation'] + 1 for s in ids),
                      'all seats reattached at generation+1', 60)
        for seat in ids:
            require(after[seat]['record']['id'] == seat, 'seat identity changed')
            require(after[seat]['record']['harness_session_id'] == before[seat]['record']['harness_session_id'],
                    'native session id changed')
        wait_codex_turn(app, codex, 'T9_AFTER', 't9-after')
        for seat in [claude, codex]:
            app.wait(seat, 'T9_AFTER')
        app.line(shell, "printf 'T9_ZSH_AFTER %s\\n' $$")
        app.wait(shell, '(?m)^T9_ZSH_AFTER [0-9]+$')
        retained = '\n'.join(app.ctl('seat', 'screen', shell)['rows'])
        old_shell = before[shell]['state']['process']['pid']
        require(re.search(rf'(?m)^T9_ZSH_BEFORE {old_shell}$|^\[gap [0-9]+\.\.[0-9]+: ', retained),
                'shell recovery lost the retained screen without a gap marker')
        report['retained_history'] = {}
        for seat in ids:
            history = app.ctl('seat', 'screen', seat, '--scrollback',
                              '1..' + str(report['capture_through'][seat]))
            text = '\n'.join(history['rows'])
            marker = 'T9_BEFORE' if seat in [claude, codex] else 'T9_ZSH_BEFORE'
            require(marker in text or re.search(r'(?m)^\[gap [0-9]+\.\.[0-9]+: ', text),
                    'restart lost prior output without a gap marker: ' + seat)
            report['retained_history'][seat] = history
        report['after'] = {s: roster()[s] for s in ids}
        for seat in ids:
            old = before[seat]['state']['process']['pid']
            new = report['after'][seat]['state']['process']
            require(new['kind'] == 'running' and new['pid'] != old, 'replacement process not live')
            until(lambda: subprocess.run(['ps', '-p', str(old), '-o', 'pid='], capture_output=True).returncode,
                  lambda code: code != 0, f'old process {old} remains after explicit restart')
            layout = app.ctl('ui', 'focus', seat, label='t9-after-' + seat)
            app.capture('window', 't9-after-' + seat, layout['frame_seq'])
        for kind in ['claude', 'codex']:

            require(any(r['phase'] == 'after' and r['kind'] == kind and r['assistant_before']
                        for r in Provider.observations), f'{kind}: resumed model request lost its prior assistant turn')
        report['verdict'] = 'full: explicit ctl restarts preserve 3 seat IDs and native sessions/transcripts, advance generations once, reap old PIDs and retain the shell screen or gap'
        print(json.dumps(dict(kind='T9', verdict=report['verdict'], seats=ids)), flush=True)
    finally:
        report['requests'] = Provider.observations
        (work / 't9-recovery.json').write_text(json.dumps(report, indent=2) + '\n')
        config.unlink(missing_ok=True)
        server.shutdown()
        server.server_close()
        worker.join(timeout=3)


if __name__ == '__main__':
    run()
