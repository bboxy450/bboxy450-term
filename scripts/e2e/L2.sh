# coverage: partial; missing: never a guessed slot
# L2: all four CLI picks, sibling geometry and replay after focus changes.
# The pending-spawn focus-change race has unit coverage, not a night witness yet.
set -eu
exec python3 -I - "$BBT" "$E2E_WORK" <<'PY'
import json, os, subprocess, sys
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
(work/'l2-result.json').write_text(json.dumps({
    'placements':['tab','right','down','space'], 'sibling_rectangles_preserved':True,
    'retry_same_seat':cid,'captured_revision':c['layout_revision'],
    'applied_revision':c['applied_layout_revision'],'captures':captures,
    'pending_focus_race':'not witnessed by this script'},indent=2)+'\n')
print('L2: tab/right/down/new swarm space, captured anchor/generation, sibling rectangles, produced frames and same-op replay held; pending-focus race remains partial')
PY
