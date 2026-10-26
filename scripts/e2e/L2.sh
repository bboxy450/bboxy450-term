# coverage: full
# L2: all four CLI picks, sibling geometry and replay after focus changes.
# The test-account dispatch barrier holds one admitted op, never an IO owner.
set -eu
exec python3 -I - "$BBT" "$E2E_WORK" <<'PY'
import json, os, subprocess, sys, time
from pathlib import Path
binary, work = Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve()
sequence = 0

def request(*args, check=True):
    global sequence
    sequence += 1
    result = subprocess.run([str(binary), 'ctl', *args], env=os.environ, stdin=subprocess.DEVNULL,
                            capture_output=True, text=True, timeout=40)
    rows = [json.loads(line) for line in result.stdout.splitlines() if line.strip()]
    assert len(rows) == 1, (args, result.stdout, result.stderr)
    row = rows[0]
    (work/f'l2-ctl-{sequence:03}.json').write_text(json.dumps(
        {'command':list(args),'exit':result.returncode,'reply':row,'stderr':result.stderr},indent=2)+'\n')
    if check: assert result.returncode == 0 and row.get('ok') is True, (args,row)
    return result.returncode,row

def ctl(*args): return request(*args)[1]['data']
def rect(layout,seat): return next(p['rect'] for p in layout['panes'] if p['seat']==seat)
def same(a,b): assert all(abs(x-y)<0.01 for x,y in zip(a,b)), (a,b)
def pick(place,name,*extra):
    return request('seat','spawn','zsh','--place',place,'--name',name,'--cwd',str(cwd),*extra)[1]
def capture(seat,label,frame):
    path=work/f'l2-{label}.png'
    reply=ctl('seat','capture',seat,str(path))
    assert path.read_bytes().startswith(b'\x89PNG\r\n\x1a\n')
    assert reply['frame_seq']>=frame
    return str(path)

def pending_focus(place, anchor):
    before = ctl('ui','focus',anchor)
    known = {s['record']['id']:s for s in ctl('seat','ls')}
    name = f'L2 pending {place}'
    ctl('debug','fault','spawn-hold')
    args = ['seat','spawn','zsh','--place',place,'--name',name,'--cwd',str(cwd)]
    # Hold exactly one admitted command before backend dispatch. All UI/control
    # traffic keeps its ordinary path, without a stopped process or a sleep.
    out = (work/f'l2-pending-{place}.stdout').open('w')
    err = (work/f'l2-pending-{place}.stderr').open('w')
    child = None
    row = None
    try:
        child = subprocess.Popen([str(binary),'ctl',*args], env=os.environ,
                                 stdin=subprocess.DEVNULL, stdout=out, stderr=err)
        deadline = time.monotonic() + 10
        while True:
            pending = ctl('ui','layout')
            rows = [r for r in pending.get('starting',[]) if r['name'] == name]
            if len(rows) == 1 and rows[0]['first_frame_seq'] is not None:
                break
            assert child.poll() is None and time.monotonic() < deadline, (
                'no produced pending placement before backend completion', place, pending)
        row = rows[0]
        assert row['state'] == {'kind':'starting'} and child.poll() is None, row
        assert row['harness_kind'] == 'zsh' and row['op'] not in known, row
        assert row['first_frame_seq'] > row['admitted_frame_seq'], row
        code, wrong = request('debug','fault','spawn-release','00000000000000000000000001',check=False)
        assert code != 0 and 'spawn_gate_not_held' in wrong['error']['message'], wrong
        assert child.poll() is None, 'wrong release completed the held launch'
        selected = row['placement']
        if place in ('right','down'):
            assert selected == {'kind':'split','tab':before['tab'],'target':anchor,
                'target_generation':known[anchor]['record']['generation'],
                'side':'right' if place == 'right' else 'bottom'}, selected
        else:
            assert selected['kind'] == 'new_tab' and selected['tab'] != before['tab'], selected
        assert pending['tab'] == selected['tab'], (pending, row)
        assert (pending['swarm'] != before['swarm']) == (place == 'space'), (pending, before)
        starting_capture = capture('window',f'pending-{place}-starting',pending['frame_seq'])
        # The explicit focus verb fences earlier placements. Ordinary pane navigation
        # is intentionally used instead. A new UI action supersedes the spawn's
        # frame waiter, not its already admitted operation or captured placement.
        moved = ctl('ui','key','cmd-alt-tab')
        assert moved['frame_seq'] > pending['frame_seq'], (pending, moved)
        assert moved['focused'] in known and moved['focused'] != anchor, moved
        focus_capture = capture(moved['focused'],f'pending-{place}-later-focus',moved['frame_seq'])
        assert child.wait(timeout=10) != 0, 'superseded frame waiter unexpectedly succeeded'
        out.flush(); err.flush()
        reply = json.loads((work/f'l2-pending-{place}.stdout').read_text())
        assert reply['ok'] is False and 'superseded' in reply['error']['message'], reply
        still = ctl('ui','layout')
        retained = [r for r in still.get('starting',[]) if r['op'] == row['op']]
        # The pending pane can be off-screen after navigation. Its admission is
        # retained by the operation, even when it is absent from the visible tree.
        if retained:
            assert retained[0]['state'] == {'kind':'starting'}, retained
    finally:
        if row is not None:
            ctl('debug','fault','spawn-release',row['op'])
        out.close(); err.close()
        if child is not None and child.poll() is None:
            child.wait(timeout=40)
    deadline = time.monotonic() + 30
    while True:
        after = {s['record']['id']:s for s in ctl('seat','ls')}
        added = set(after) - set(known)
        if added:
            break
        assert time.monotonic() < deadline, ('accepted spawn never completed', place)
    assert len(added) == 1 and set(known) <= set(after), (known, after)
    seat = added.pop()
    record = after[seat]['record']
    assert record['name'] == name and record['harness_kind'] == 'zsh', record
    final = ctl('ui','focus',seat)
    assert final['swarm'] == pending['swarm'] and final['tab'] == selected['tab'], final
    same(rect(final,seat), row['rect'])
    if place in ('right','down'):
        for sibling in before['panes']:
            if sibling['seat'] != anchor:
                same(sibling['rect'], rect(final,sibling['seat']))
        for sibling in pending['panes']:
            same(sibling['rect'], rect(final,sibling['seat']))
        x,y,w,h = rect(final,seat)
        ax,ay,aw,ah = rect(final,anchor)
        if place == 'right':
            assert x >= ax+aw and abs(y-ay)<0.01 and abs(h-ah)<0.01, final
        else:
            assert y >= ay+ah and abs(x-ax)<0.01 and abs(w-aw)<0.01, final
    else:
        assert len(final['panes']) == 1, final
    image = capture(seat,f'pending-{place}',final['frame_seq'])
    code, duplicate = request('debug','fault','spawn-release',row['op'],check=False)
    assert code != 0 and 'spawn_gate_not_held' in duplicate['error']['message'], duplicate
    assert len(ctl('seat','ls')) == len(known)+1, 'release duplicated the launch'
    result = {'place':place,'anchor':anchor,
              'captured':row,'pending_frame':pending['frame_seq'],
              'later_focus':moved,'superseded_frame_reply':reply,
              'seat':seat,'final':final,'capture':image,
              'starting_capture':starting_capture,'focus_capture':focus_capture}
    (work/f'l2-pending-{place}.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps({'kind':'L2-pending','place':place,'op':row['op'],
        'placement':selected,'anchor':anchor,'later_focus':moved['focused'],
        'admitted_frame':row['admitted_frame_seq'],'starting_frame':row['first_frame_seq'],
        'pending_frame':pending['frame_seq'],'focus_frame':moved['frame_seq'],
        'final_frame':final['frame_seq'],'starting_rect':row['rect'],
        'final_rect':rect(final,seat),'seat':seat,'one_release_one_seat':True}),flush=True)
    return result

status=ctl('app','status')
assert status['windows']==0 and status['custody_backend']=='file', status
assert ctl('seat','ls')==[], 'the night must supply a fresh per-promise app'
cwd=work/'L2 chosen cwd';cwd.mkdir(mode=0o700,exist_ok=True)
code,missing=request('seat','spawn','zsh','--place','right',check=False)
assert code!=0 and 'placement_anchor_missing' in missing['error']['message'], missing
assert ctl('seat','ls')==[], 'an invalid split launched a seat'
a=pick('tab','L2 anchor')['data'];aid=a['seat']
assert a['layout']['focused']==aid and len(a['layout']['panes'])==1
swarm,tab=a['swarm'],a['layout']['tab']
captures=[capture(aid,'tab',a['frame_seq'])]
b=pick('right','L2 outer right')['data'];bid=b['seat']
assert b['placement']['target']==aid and b['placement']['side']=='right'
before=ctl('ui','focus',aid)
target_generation=next(s['record']['generation'] for s in ctl('seat','ls') if s['record']['id']==aid)
c_row=pick('right','L2 inner right');c=c_row['data'];cid=c['seat']
assert c['placement']['target']==aid and c['placement']['target_generation']==target_generation
assert c['placement']['tab']==tab and c['layout']['tab']==tab
assert c['layout_revision']==before['layout_revision']
assert c['applied_layout_revision']==c['layout']['layout_revision']>=c['layout_revision']
assert c['frame_seq']>before['frame_seq'] and c['layout']['focused']==cid
assert rect(c['layout'],cid)[0]>rect(c['layout'],aid)[0]
same(rect(before,bid),rect(c['layout'],bid))
captures.append(capture(cid,'right',c['frame_seq']))
# Retry from a different focus must return the saved capture and one seat.
ctl('ui','focus',bid)
retry=pick('right','L2 inner right','--op',c_row['op'])['data']
assert retry==c
assert ctl('ui','layout')['focused']==bid and len(ctl('seat','ls'))==3
before=ctl('ui','focus',cid)
d=pick('down','L2 below')['data'];did=d['seat'];down=d['layout']
assert d['placement']['target']==cid and d['placement']['side']=='bottom'
assert down['focused']==did and down['swarm']==swarm and down['tab']==tab
same(rect(before,aid),rect(down,aid));same(rect(before,bid),rect(down,bid))
assert rect(down,did)[1]>rect(down,cid)[1]
assert abs(rect(down,did)[0]-rect(down,cid)[0])<0.01
captures.append(capture(did,'down',d['frame_seq']))
e=pick('tab','L2 separate tab')['data']
assert e['swarm']==swarm and e['layout']['tab']!=tab and len(e['layout']['panes'])==1
restored=ctl('ui','focus',aid)
for seat in (aid,bid,cid,did): same(rect(down,seat),rect(restored,seat))
space=pick('space','L2 new space')['data'];sid=space['seat']
assert space['swarm']!=swarm and space['layout']['tab']!=tab
assert space['layout']['focused']==sid and len(space['layout']['panes'])==1
assert next(s['record']['swarm'] for s in ctl('seat','ls') if s['record']['id']==sid)==space['swarm']
captures.append(capture(sid,'space',space['frame_seq']))
restored=ctl('ui','focus',aid)
assert restored['swarm']==swarm and restored['tab']==tab
for seat in (aid,bid,cid,did): same(rect(down,seat),rect(restored,seat))
assert len(ctl('seat','ls'))==6
races = [pending_focus(place,aid) for place in ('right','down','tab','space')]
assert len(ctl('seat','ls'))==10
(work/'l2-result.json').write_text(json.dumps({
    'placements':['tab','right','down','space'], 'sibling_rectangles_preserved':True,
    'retry_same_seat':cid,'captured_revision':c['layout_revision'],
    'applied_revision':c['applied_layout_revision'],'captures':captures,
    'pending_focus_races':races},indent=2)+'\n')
print('L2: tab/right/down/new swarm space, captured anchor/generation, sibling rectangles, produced frames and same-op replay held; captured pending slots survive later keyboard focus')
PY
