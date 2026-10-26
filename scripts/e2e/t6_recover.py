"""Supervisor death: real interactive clients resume, and the GUI says restarted.

Model SSE is loopback-scripted; only the native clients handle sessions. No
provider handler calls app tools or fabricates a session id or ctl verdict.
"""
import hashlib
import http.server
import io
import json
import math
import os
from pathlib import Path
import re
import signal
import subprocess
import sys
import threading
import time

sys.path.insert(0, str(Path(__file__).resolve().parent))
import native_checks as native
from ui import App, nodes, require


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
            assistant_before=any(x.get('role') == 'assistant' and 'T6_BEFORE' in json.dumps(x)
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


def restart_chips(layout, names):
    tree = list(nodes(layout))
    for name in names:
        # Find the actual positioned sidebar label, then its sibling chip.
        labels = [n for n in tree if n['kind'].startswith('named:swarm') and
                  n['kind'].endswith('/name') and
                  any(c.get('text') == name for c in n.get('children', []))]
        require(len(labels) == 1, f'no unique drawn sidebar row: {name}')
        key = labels[0]['kind'][:-len('/name')] + '/right/restarted'
        chip = next((n for n in tree if n['kind'] == key), None)
        require(chip is not None, f'{name}: restarted chip absent from produced frame')
        require(chip['rect'][2] > 0 and chip['rect'][3] > 0, 'restart notice has no visible rectangle')
        require(any(c.get('text') == 'restarted' for c in objects(chip)), 'restart notice has no painted text')


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
        native.MARKER = 'T6_BEFORE'
        claude = app.ctl('seat', 'spawn', 'claude', '--role', 'executor', '--place', 'tab',
                         '--cwd', work, '--name', 'T6 native Claude')['seat']
        app.wait(claude, 'T6_BEFORE')
        codex = app.ctl('seat', 'spawn', 'codex', '--role', 'executor', '--place', 'tab',
                        '--cwd', work, '--name', 'T6 native Codex')['seat']
        native.approve_codex_hooks(codex, marker='T6_BEFORE', prefix='t6')
        app.wait(codex, 'T6_BEFORE')
        shell = app.spawn('T6 zsh')
        app.line(shell, "printf 'T6_ZSH_BEFORE %s\\n' $$")
        app.wait(shell, '(?m)^T6_ZSH_BEFORE [0-9]+$')
        ids = [claude, codex, shell]
        before = until(roster, lambda xs: all(xs[s]['state']['process']['kind'] == 'running' for s in ids)
                       and all(xs[s]['record']['harness_session_id'] for s in [claude, codex]),
                       'live native sessions bound')
        report['before'] = {s: before[s] for s in ids}
        for seat in ids:
            layout = app.ctl('ui', 'focus', seat)
            app.capture(seat, 't6-before-' + seat, layout['frame_seq'])
        frozen = app.ctl('ui', 'clock', 'freeze', label='t6-clock-origin')
        parent = int(os.environ['E2E_APP'])
        children = []
        for line in subprocess.check_output(['ps', '-axo', 'pid=,ppid=,command='], text=True).splitlines():
            parts = line.strip().split(None, 2)
            if len(parts) == 3 and parts[1] == str(parent) and parts[2].endswith(' supervisor'):
                children.append(int(parts[0]))
        require(len(children) == 1, 'no unique supervisor child of the runner-owned app')
        supervisor = children[0]
        birth = subprocess.check_output(['ps', '-p', str(supervisor), '-o', 'lstart=,comm='], text=True).strip()
        report['fault'] = dict(app=parent, supervisor=supervisor, birth_and_command=birth)
        (work / 't6-owned-supervisor.json').write_text(json.dumps(report['fault']) + '\n')
        require(subprocess.check_output(['ps', '-p', str(supervisor), '-o', 'lstart=,comm='], text=True).strip() == birth,
                'supervisor identity changed before the owned fault')
        native.MARKER = 'T6_AFTER'
        Provider.phase = 'after'
        os.kill(supervisor, signal.SIGKILL)
        after = until(roster, lambda xs: all(xs.get(s, {}).get('record', {}).get('generation') ==
                                            before[s]['record']['generation'] + 1 for s in ids),
                      'all seats reattached at generation+1', 60)
        for seat in ids:
            require(after[seat]['record']['id'] == seat, 'seat identity changed')
            require(after[seat]['record']['harness_session_id'] == before[seat]['record']['harness_session_id'],
                    'native session id changed')
        native.approve_codex_hooks(codex, marker='T6_AFTER', prefix='t6-after')
        for seat in [claude, codex]:
            app.wait(seat, 'T6_AFTER')
        app.line(shell, "printf 'T6_ZSH_AFTER %s\\n' $$")
        app.wait(shell, '(?m)^T6_ZSH_AFTER [0-9]+$')
        retained = '\n'.join(app.ctl('seat', 'screen', shell)['rows'])
        old_shell = before[shell]['state']['process']['pid']
        require(re.search(rf'(?m)^T6_ZSH_BEFORE {old_shell}$|^\[gap [0-9]+\.\.[0-9]+: ', retained),
                'shell recovery lost the retained screen without a gap marker')
        report['after'] = {s: roster()[s] for s in ids}
        for seat in ids:
            old = before[seat]['state']['process']['pid']
            new = report['after'][seat]['state']['process']
            require(new['kind'] == 'running' and new['pid'] != old, 'replacement process not live')
            require(subprocess.run(['ps', '-p', str(old), '-o', 'pid='], capture_output=True).returncode != 0,
                    f'old seat process {old} survived supervisor death')
            layout = app.ctl('ui', 'focus', seat, label='t6-after-' + seat)
            restart_chips(layout, ['T6 native Claude', 'T6 native Codex', 'T6 zsh'])
            app.capture('window', 't6-after-' + seat, layout['frame_seq'])
        # The lab notice holds to 85%, fades, then becomes a persistent stream line.
        held = app.ctl('ui', 'clock', 'advance', 3400, label='t6-notice-hold-layout')
        restart_chips(held, ['T6 native Claude', 'T6 native Codex', 'T6 zsh'])
        hold_image = app.capture('window', 't6-notice-hold', held['frame_seq'])
        fading = app.ctl('ui', 'clock', 'advance', 300, label='t6-notice-fade-layout')
        restart_chips(fading, ['T6 native Claude', 'T6 native Codex', 'T6 zsh'])
        fade_image = app.capture('window', 't6-notice-fade', fading['frame_seq'])
        from rim_clock import png, region
        full, faded = png(hold_image), png(fade_image)
        pixels = []
        for chip in (n for n in nodes(held) if n['kind'].endswith('/right/restarted')):
            x, y = [math.floor(v + .5) for v in chip['rect'][:2]]
            def pixel(image, dx):
                return region(image, [x + dx, y + 2, x + dx + 1, y + 3])[:3]
            background = pixel(full, -2)
            distance = lambda color: sum(abs(c - b) for c, b in zip(color, background))
            opaque, dimmed = distance(pixel(full, 2)), distance(pixel(faded, 2))
            require(0 < dimmed < opaque, 'restart chip did not fade in actual GPU pixels')
            pixels.append(dict(node=chip['kind'], opaque_distance=opaque, faded_distance=dimmed))
        require(len(pixels) == 3, 'fade proof omitted a restart chip')
        report['fade_pixels'] = pixels
        done = app.ctl('ui', 'clock', 'advance', 300, label='t6-notice-done-layout')
        require(done['clock']['time_ms'] == frozen['clock']['time_ms'] + 4000, 'clock did not advance exactly four seconds')
        require(not any(n['kind'].endswith('/right/restarted') for n in nodes(done)), 'chip survived four seconds')
        text = ' '.join(n.get('text', '') for n in nodes(done))
        for name in ['T6 native Claude', 'T6 native Codex', 'T6 zsh']:
            require(text.count(name + ' restarted · process generation 2') == 1, 'missing or duplicate generation stream notice: ' + name)
        later = app.ctl('ui', 'clock', 'advance', 4000, label='t6-notice-later-layout')
        require(not any(n['kind'].endswith('/right/restarted') for n in nodes(later)), 'reprojection rearmed a restart chip')
        report['notice_clock'] = dict(origin=frozen['clock'], done=done['clock'], later=later['clock'])
        app.capture('window', 't6-notice-done', later['frame_seq'])
        for kind in ['claude', 'codex']:

            require(any(r['phase'] == 'after' and r['kind'] == kind and r['assistant_before']
                        for r in Provider.observations), f'{kind}: resumed model request lost its prior assistant turn')
        report['verdict'] = 'full: 3 stable seats, generation+1, old PIDs gone, both native sessions/transcripts resumed, 3 drawn restart notices fade at 4s into single generation stream lines'
        print(json.dumps(dict(kind='T6', verdict=report['verdict'], seats=ids)), flush=True)
    finally:
        report['requests'] = Provider.observations
        (work / 't6-recovery.json').write_text(json.dumps(report, indent=2) + '\n')
        config.unlink(missing_ok=True)
        server.shutdown()
        server.server_close()
        worker.join(timeout=3)


if __name__ == '__main__':
    run()
