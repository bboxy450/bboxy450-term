# coverage: partial; missing: malformed/unapproved report rejection; reported grade drawn in the seat row
# T8: OSC 7501 state shows as the reported grade; a silent seat shows idle, never invented.
. "$(dirname "$0")/lib.sh"
quiet=$(spawn_zsh) || fail "spawn refused"
run_line "$quiet" 'echo quiet'
wait_text "$quiet" '(?m)^quiet$'
ctl seat wait "$quiet" --state idle --timeout 5000 >/dev/null || fail "silent seat never became idle"
[ "$(seat_field "$quiet" state.activity.kind)" = idle ] || fail "a silent seat is not idle: $(seat_field "$quiet" state.activity)"
seat=$(spawn_zsh) || fail "spawn refused"
# Keep the command open: returning to zsh's prompt emits 133 B (main idle).
# A 7501 frame without id names the main seat; id=t8 names a subagent.
run_line "$seat" "printf '\\033]7501;state=working:app=e2e\\007'; echo reported; read -r response"
wait_text "$seat" '(?m)^reported$'
tries=0
until [ "$(seat_field "$seat" state.activity.kind)" = active ] &&
      [ "$(seat_field "$seat" state.grade)" = reported ]; do
    tries=$((tries + 1))
    [ "$tries" -le 25 ] || fail "OSC 7501 state=working did not show: $(seat_field "$seat" state.activity), grade $(seat_field "$seat" state.grade)"
    sleep 0.2
done
[ "$(seat_field "$seat" state.subagents)" = '[]' ] || fail "untagged 7501 invented a subagent"

child=$(spawn_zsh) || fail "spawn refused"
run_line "$child" "printf '\\033]7501;state=working:app=e2e:id=t8\\007'; echo child-reported; read -r response"
wait_text "$child" '(?m)^child-reported$'
tries=0
until seat_field "$child" state.subagents | python3 -I -c '
import json, sys
rows = json.load(sys.stdin)
sys.exit(0 if any(row["id"] == "t8" and row["activity"]["kind"] == "active"
                  and row["grade"] == "reported" for row in rows) else 1)'; do
    tries=$((tries + 1))
    [ "$tries" -le 25 ] || fail "tagged OSC 7501 did not show a reported active subagent: $(seat_field "$child" state.subagents)"
    sleep 0.2
done
