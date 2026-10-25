# coverage: partial; missing: a waiting seat posts one desktop notification; clicking it raises the window, switches space
# R6: recording proves policy, replacement and exact-pane frame activation, not OS delivery.
set -eu
exec python3 -I - "$BBT" "$E2E_WORK" <<'PY'
import json, os, re, shlex, subprocess, sys, time
from pathlib import Path

binary, work = Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve()
assert work.is_dir(), 'the night must supply its private E2E_WORK'
# Lifecycle and the recording selector belong to the runner's one app.
env = dict(os.environ)
sequence = 0

def invoke(*args, check=True):
    global sequence
    sequence += 1
    result = subprocess.run([str(binary), 'ctl', *args], env=env, cwd=work,
                            stdin=subprocess.DEVNULL, capture_output=True, text=True, timeout=40)
    rows = [json.loads(line) for line in result.stdout.splitlines() if line.strip()]
    assert len(rows) == 1, f'ctl {args[:2]} did not return one JSON reply: {result.stderr}'
    reply = rows[0]
    (work/f'r6-ctl-{sequence:03}.json').write_text(json.dumps(
        {'command':list(args), 'exit':result.returncode, 'reply':reply, 'stderr':result.stderr}, indent=2)+'\n')
    if check:
        assert result.returncode == 0 and reply.get('ok') is True, (args, reply)
    return result.returncode, reply

def ctl(*args):
    return invoke(*args)[1]['data']

def wait_record(phase, predicate):
    deadline = time.monotonic()+30
    while True:
        state = ctl('debug', 'notifications')
        if state.get('available') and state.get('settled') and predicate(state):
            assert state['backend'] == 'recording' and state['os_presentation'] is False
            assert state['omitted_outstanding'] == 0 and state['omitted_records'] == 0
            (work/f'r6-{phase}.json').write_text(json.dumps(state, indent=2)+'\n')
            return state
        assert time.monotonic() < deadline, (phase, state)
        time.sleep(0.05)

def line(seat, text):
    ctl('seat', 'type', seat, text)
    ctl('seat', 'key', seat, 'enter')

def question(text):
    line(target, text)
    ctl('seat', 'wait', target, '--text', re.escape('R6PRODUCER:'+text), '--timeout', '20000')

ready = ctl('app', 'status')
assert ready['windows'] == 0 and ready['custody_backend'] == 'file', ready
backend = ctl('debug', 'notifications')
assert backend['backend'] == 'recording', 'runner must select --notifications record'
target = ctl('seat', 'spawn', 'zsh', '--place', 'tab', '--cwd', str(work), '--name', 'R6 waiting')['seat']
original = ctl('ui', 'focus', target)
other = ctl('seat', 'spawn', 'zsh', '--place', 'tab', '--cwd', str(work), '--name', 'R6 background')['seat']
background = ctl('ui', 'focus', other)
assert background['tab'] != original['tab'] and background['focused'] == other
producer = work/'r6-waiting-producer.py'
producer.write_text("""import base64,sys
print('R6READY',flush=True)
for raw in sys.stdin:
    question=raw.rstrip('\\n')
    print('R6PRODUCER:'+question,flush=True)
    if question=='IDLE':
        print('\\033]7501;state=idle\\007',end='',flush=True)
    else:
        detail=base64.b64encode((question+'\\nprivate second line').encode()).decode()
        print('\\033]7501;state=blocked:kind=question:msg='+detail+'\\007',end='',flush=True)
""")
compile(producer.read_text(), str(producer), 'exec')
line(target, 'python3 -u '+shlex.quote(str(producer)))
ctl('seat', 'wait', target, '--text', 'R6READY', '--timeout', '20000')
question('First question?')
first = wait_record('first', lambda s: s['outstanding_count'] == 1 and
                    s['outstanding'][0]['body'] == 'First question?')
assert first['posts'] == 1 and first['outstanding'][0]['id'] == target
assert 'private second line' not in json.dumps(first)
question('Replacement question?')
repeated = wait_record('replaced', lambda s: s['posts'] == 2 and
                       s['outstanding'][0]['body'] == 'Replacement question?')
assert repeated['outstanding_count'] == 1 and repeated['replacements'] == 1
assert repeated['outstanding'][0]['id'] == target
assert any(r['operation'] == 'replace' and r['stage'] == 'completion' for r in repeated['records'])
before = ctl('ui', 'layout')
assert before['focused'] == other
clicked = ctl('ui', 'notification-click', target)
assert clicked['focused'] == target and clicked['tab'] == original['tab']
assert clicked['frame_seq'] > before['frame_seq']
assert any(p['seat'] == target and p['focused'] for p in clicked['panes'])
image = work/'r6-clicked.png'
capture = ctl('seat', 'capture', target, str(image))
assert image.read_bytes().startswith(b'\x89PNG\r\n\x1a\n')
assert capture['frame_seq'] >= clicked['frame_seq']
handled = wait_record('activated', lambda s: s['activations'] == 1 and s['outstanding_count'] == 0)
assert handled['focused'] == target
# Mute uses the same chrome action as a user; a fresh waiting episode then
# reaches the processed-plan fence without any notification post.
ctl('ui', 'key', 'cmd-m')
row = next(r for r in ctl('seat', 'ls') if r['record']['id'] == target)
assert row['record']['muted'] is True
ctl('ui', 'focus', other)
question('IDLE')
ctl('seat', 'wait', target, '--state', 'idle', '--timeout', '20000')
question('Muted question?')
muted = wait_record('muted', lambda s: any(r['seat'] == target and r['muted'] for r in s['waiting']))
assert muted['posts'] == 2 and muted['outstanding_count'] == 0, 'muted seat posted'
code, refused = invoke('ui', 'notification-click', other, check=False)
assert code != 0 and refused['ok'] is False
assert 'notification is not outstanding' in refused['error']['message'], refused
assert ctl('ui', 'layout')['focused'] == other
assert ctl('app', 'status')['windows'] == 0
(work/'r6-result.json').write_text(json.dumps({
    'backend':'recording', 'os_presentation':False, 'posts':muted['posts'],
    'replacements':muted['replacements'], 'muted_new_posts':muted['posts']-handled['posts'],
    'activated_seat':target, 'frame_seq':clicked['frame_seq'], 'capture':str(image)}, indent=2)+'\n')
print('R6 recording: first line, one outstanding ID, replacement, muted sends=0, exact tab/pane completed frame; OS presentation unproven')
PY
