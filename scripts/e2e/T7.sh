# coverage: partial; missing: solved pane rectangle and focus after undo; visible grace strip
# T7: close and undo use the window's actual layout; never a second lifecycle door.
# The runner owns the throwaway account/app. This script needs no desktop input.
. "$(dirname "$0")/lib.sh"
seat=$(spawn_zsh 'T7 retained fixture') || fail 'spawn refused'
run_line "$seat" 't7_cookie=73189; echo T7_READY-$$'
wait_text "$seat" '(?m)^T7_READY-[0-9]+$'
shell=$(seat_field "$seat" state.process.pid)
generation=$(seat_field "$seat" record.generation)
case "$generation" in ''|*[!0-9]*) fail 'fixture generation is missing';; esac
[ "$generation" -ge 1 ] || fail 'fixture generation is not live'
[ -n "$shell" ] && kill -0 "$shell" 2>/dev/null || fail 'fixture PTY is not running'
name=$(seat_field "$seat" record.name)
# Read the retained UI used by both hosts, not another registry projection.
check_ui() {
    phase=$1
    ctl app status > "$E2E_WORK/t7-ui-$phase.json" || fail 'app status refused'
    python3 -I - "$E2E_WORK" "$phase" "$seat" "$name" <<'PYUI'
import json,sys
from pathlib import Path
root,phase,seat,name=Path(sys.argv[1]),*sys.argv[2:]
c=json.loads((root/f't7-ui-{phase}.json').read_text())['data']['close']
assert c['complete'], 'the close readback is incomplete'
slots=[s for s in c['slots'] if s['seat']==seat]
grace=[g for g in c['grace'] if g['seat']==seat]
if phase=='before':
    assert len(slots)==1 and not grace
else:
    before=json.loads((root/'t7-ui-before.json').read_text())['data']['close']
    owner=next(s for s in before['slots'] if s['seat']==seat)
    if phase=='grace':
        assert not slots and len(grace)==1
        assert grace[0]['undo'] and 0 < grace[0]['remaining_ms'] <= 60000
        assert grace[0]['swarm']==owner['swarm']
    elif phase=='restored':
        assert c['slots']==before['slots'], 'sidebar order or owner changed'
        assert not grace
    elif phase=='gone':
        assert not slots and not grace
        notices=[n for n in c['notices'] if n['text']==f'{name} closed; its history is kept']
        assert len(notices)==1 and notices[0]['swarm']==owner['swarm']
        assert notices[0]['time'] and not notices[0]['truncated']
PYUI
}
wait_ui() {
    phase=$1; tries=0
    while :; do
        if check_ui "$phase"; then return; fi
        tries=$((tries+1)); [ "$tries" -le 100 ] || fail "UI never reached $phase"
        sleep 0.2
    done
}
# Focus already has a first-produced-frame acknowledgement; fixture setup does
# not rely on a spawn reply or a backend screen proving a pane was drawn.
ctl seat focus "$seat" > "$E2E_WORK/t7-focus.json" || fail 'fixture focus refused'
wait_ui before
# A presented pane, not just a live backend record, is required before close.
ctl seat capture "$seat" "$E2E_WORK/t7-before.png" > "$E2E_WORK/t7-before.json" || fail 'the running pane has no capture'
wait_lifecycle() {
    tries=0
    until [ "$(seat_field "$seat" record.lifecycle)" = "$1" ]; do
        tries=$((tries+1)); [ "$tries" -le "$2" ] || fail "seat never became $1"
        sleep 0.2
    done
}
assert_not_on_screen() {
    python3 -I - "$1" "$seat" <<'PYHIDDEN'
import json,sys
from pathlib import Path
rows=[json.loads(line) for line in Path(sys.argv[1]).read_text().splitlines() if line.strip()]
assert len(rows)==1, 'capture did not return one structured error'
row=rows[0]
assert row.get('ok') is False, 'capture did not refuse'
error=row.get('error',{})
assert error.get('kind')=='not_on_screen', f'unrelated capture error: {error}'
assert error.get('message')==f'not_on_screen: seat {sys.argv[2]} has no pane on screen', 'capture refused for a different seat'
PYHIDDEN
}
receipt_frame() {
    python3 -I - "$1" "$seat" "$2" <<'PYRECEIPT'
import json,sys
row=json.load(open(sys.argv[1]))
assert row.get('ok') is True, 'window action refused'
data=row['data']
assert data['seat']==sys.argv[2] and data['action']==sys.argv[3]
frame=data.get('frame_seq')
assert isinstance(frame,int) and not isinstance(frame,bool) and frame>0, 'no produced window frame receipt'
print(frame)
PYRECEIPT
}
close() {
    ctl seat close "$seat" > "$E2E_WORK/t7-close.json" || fail 'window close refused'
    close_frame=$(receipt_frame "$E2E_WORK/t7-close.json" close) || fail 'close replied before a produced frame'
    [ "$(seat_field "$seat" record.lifecycle)" = closed_grace ] || fail 'close receipt precedes its lifecycle'
    seat_field "$seat" record.close.position > "$E2E_WORK/t7-position-$1.json"
    [ "$(seat_field "$seat" record.generation)" = "$generation" ] || fail 'close replaced the generation'
    kill -0 "$shell" 2>/dev/null || fail 'close terminated the PTY inside grace'
    # One capture after the receipt; a retry loop would hide an early reply.
    ctl seat capture "$seat" "$E2E_WORK/t7-hidden.png" > "$E2E_WORK/t7-hidden.json" 2>&1 || :
    assert_not_on_screen "$E2E_WORK/t7-hidden.json" || fail 'closed pane capture failed for an unrelated reason'
    [ ! -e "$E2E_WORK/t7-hidden.png" ] || fail 'a closed pane wrote an image'
    check_ui grace || fail 'close receipt precedes the retained UI state'
}
close first
ctl seat undo-close "$seat" > "$E2E_WORK/t7-undo.json" || fail 'window undo refused'
undo_frame=$(receipt_frame "$E2E_WORK/t7-undo.json" undo-close) || fail 'undo replied before a produced frame'
[ "$undo_frame" -gt "$close_frame" ] || fail 'undo did not produce a newer window frame'
[ "$(seat_field "$seat" record.lifecycle)" = open ] || fail 'undo receipt precedes its lifecycle'
check_ui restored || fail 'undo receipt precedes the retained UI state'
[ "$(seat_field "$seat" state.process.pid)" = "$shell" ] || fail 'undo spawned another PTY'
[ "$(seat_field "$seat" record.generation)" = "$generation" ] || fail 'undo replaced the generation'
run_line "$seat" 'echo T7_PERSIST-$t7_cookie-$$'
wait_text "$seat" "(?m)^T7_PERSIST-73189-$shell\$"
ctl seat capture "$seat" "$E2E_WORK/t7-restored.png" > "$E2E_WORK/t7-restored.json" || fail 'restored pane is not visible after the receipt'
restored_frame=$(jget data.frame_seq < "$E2E_WORK/t7-restored.json") || fail 'restored capture lacks a window frame'
[ "$restored_frame" -ge "$undo_frame" ] || fail 'restored image predates undo'
python3 -I - "$E2E_WORK/t7-before.png" "$E2E_WORK/t7-restored.png" <<'PY'
import struct,sys
from pathlib import Path
def size(path):
    b=Path(path).read_bytes()
    assert b[:8]==b'\x89PNG\r\n\x1a\n', 'capture is not PNG'
    return struct.unpack('>II',b[16:24])
assert size(sys.argv[1])==size(sys.argv[2]), 'undo moved the pane extent'
PY
[ "$?" -eq 0 ] || fail 'restored capture changed extent'
# Closing it again reads the restored solved layout, proving tab/path/ratios,
# including a split when the runner has placed this seat beside a neighbour.
close second
python3 -I - "$E2E_WORK/t7-position-first.json" "$E2E_WORK/t7-position-second.json" <<'PY'
import json,sys
positions=[json.load(open(p)) for p in sys.argv[1:]]
for p in positions:
    assert p['swarm'] and p['tab'], 'position lacks its owner'
    assert len(p['split_path'])==len(p['ratios']), 'split path/ratio mismatch'
    assert all(n in (0,1) for n in p['split_path'])
    assert all(0 < r < 1 for r in p['ratios'])
    p.pop('layout_revision') # successful close/undo intentionally advances it
assert positions[0]==positions[1], 'undo changed the solved split position'
PY
[ "$?" -eq 0 ] || fail 'restored position differs'
deadline=$(seat_field "$seat" record.close.deadline_ms)
# Real sixty-second grace, not a shorter test-only lifecycle or a synthetic clock.
wait_lifecycle closed_retained 375
python3 -I - "$deadline" <<'PY'
import sys,time
assert int(time.time()*1000) >= int(sys.argv[1]), 'grace ended early'
PY
[ "$?" -eq 0 ] || fail 'grace deadline was not held'
[ "$(seat_field "$seat" record.generation)" = "$generation" ] || fail 'expiry invented a new generation'
wait_ui gone
tries=0
while kill -0 "$shell" 2>/dev/null; do
    tries=$((tries+1)); [ "$tries" -le 100 ] || fail 'the retained PTY is still running after grace'
    sleep 0.1
done
# The record and ring remain readable; no purge is part of close.
ctl seat screen "$seat" --scrollback 0..1 > "$E2E_WORK/t7-retained-history.json" || fail 'retained history is inaccessible'
ctl seat undo-close "$seat" > "$E2E_WORK/t7-expired-undo.json" 2>&1 || :
sleep 0.3
[ "$(seat_field "$seat" record.lifecycle)" = closed_retained ] || fail 'expired undo reopened the seat'
ctl seat capture "$seat" "$E2E_WORK/t7-gone.png" > "$E2E_WORK/t7-gone.json" 2>&1 || :
assert_not_on_screen "$E2E_WORK/t7-gone.json" || fail 'retained pane capture failed for an unrelated reason'
[ ! -e "$E2E_WORK/t7-gone.png" ] || fail 'a retained pane wrote an image'
note 'T7 same PID/generation, retained history, position, presented extent, sidebar order/grace/notice, and real grace/expiry held'
