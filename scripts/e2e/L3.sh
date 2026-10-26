# coverage: full
# harness: installed clients, scripted local model
# L3: private home, real seat launch overlays, real MCP handshake and real hook publication.
. "$(dirname "$0")/lib.sh"
python3 -I -B - "$(dirname "$0")" <<'PY'
import hashlib
import json
import os
from pathlib import Path
import sys
import time
sys.path.insert(0, sys.argv[1])
from task_checks import ctl, seats

work = Path(os.environ['E2E_WORK']).resolve()
home = work.parent / 'home'
assert (home / '.l3-private-home').read_text() == 'L3 throwaway harness home\n'

def snapshot():
    paths = [home / '.claude.json'] + [p for name in ('.claude', '.codex')
            for p in sorted((home / name).rglob('*')) if p.is_file()]
    return {str(p.relative_to(home)): hashlib.sha256(p.read_bytes()).hexdigest() for p in paths}

before = snapshot()
assert before, 'private harness configuration seed is absent'
seed = Path(sys.argv[1]) / 'L3.fixture-home'
expected = {str(p.relative_to(seed)): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in seed.rglob('*') if p.is_file() and (
                p.name == '.claude.json' or p.relative_to(seed).parts[0] in ('.claude', '.codex'))}
assert before == expected, 'app startup changed the private user harness configuration seed'
proofs = []
for kind in ('claude', 'codex'):
    launched = ctl('seat', 'spawn', kind, '--role', 'executor', '--place', 'tab',
                   '--cwd', str(work), '--name', 'L3 ' + kind + ' wiring stand-in')
    seat = launched['data']['seat']
    # PTY text and the retained GUI state publish independently. Wait for the
    # actual transition before asserting the state; READY alone is not a fence.
    ctl('seat', 'wait', seat, '--text', 'L3_LAUNCH_READY ' + seat, '--timeout', '30000')
    deadline = time.monotonic() + 30
    while True:
        row = seats()[seat]
        if row['state']['activity']['kind'] == 'active' and row['state']['grade'] == 'shim':
            break
        assert time.monotonic() < deadline, ('GUI shim state not published', row)
        time.sleep(.05)
    proof = json.loads((work / (seat + '.launch.json')).read_text())
    assert proof['harness'] == 'stand-in' and proof['kind'] == kind, proof
    assert proof['seat'] == seat and proof['generation'] == row['record']['generation'], proof
    assert proof['home'] == str(home), proof
    assert Path(proof['overlay']).resolve().is_relative_to(work.parent / 'run'), proof
    assert proof['before'] == before == proof['after'], proof
    assert proof['skill_sha256'] == proof['pinned_skill_sha256'], proof
    assert row['state']['activity']['kind'] == 'active' and row['state']['grade'] == 'shim', row
    assert row['record']['role'] == {'name': 'executor', 'class': 'worker'}, row
    proofs.append(dict(launch=launched, proof=proof, published=row['state']))
    print(json.dumps(dict(kind='L3-case', harness='stand-in', seat=seat,
        op=launched['op'], frame_seq=launched['data']['frame_seq'],
        actor=proof['responses']['2']['result']['structuredContent']['actor'],
        overlay=proof['overlay'], hook_events=proof['hook_events'],
        hook_observation_attempts=proof['hook_observation_attempts'],
        published_grade=row['state']['grade'], skill_sha256=proof['skill_sha256'],
        config_sha256=proof['after'])), flush=True)

assert len({p['proof']['overlay'] for p in proofs}) == len(proofs), 'seats shared a launch overlay'
assert snapshot() == before, 'launches changed private .claude/.codex bytes or paths'
(work / 'l3-launch-evidence.json').write_text(json.dumps(
    dict(harness='stand-in', before=before, after=snapshot(), seats=proofs), indent=2) + '\n')
print('L3 overlay PASS (harness: stand-in): pinned skill, MCP initialize/tools/orient, published shim activity and unchanged private configs')
from native_checks import run
run()
PY
[ "$?" -eq 0 ] || fail 'L3 private launch wiring assertion failed'
