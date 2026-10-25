# coverage: partial; missing: malformed/unapproved report rejection; reported grade drawn in the seat row
# T8: OSC 7501 state shows as the reported grade; a silent seat shows idle, never invented.
. "$(dirname "$0")/lib.sh"
quiet=$(spawn_zsh) || fail "spawn refused"
run_line "$quiet" 'echo quiet'
wait_text "$quiet" '(?m)^quiet$'
ctl seat wait "$quiet" --state idle --timeout 30000 >/dev/null || fail "silent seat never became idle"
[ "$(seat_field "$quiet" state.activity.kind)" = idle ] || fail "a silent seat is not idle: $(seat_field "$quiet" state.activity)"
seat=$(spawn_zsh) || fail "spawn refused"
# Keep the command open: returning to zsh's prompt emits 133 B (main idle).
# A 7501 frame without id names the main seat; id=t8 names a subagent.
run_line "$seat" "printf '\\033]7501;state=working:app=e2e\\007'; echo reported; read -r response"
wait_text "$seat" '(?m)^reported$'
ctl seat wait "$seat" --state active --grade reported --timeout 30000 >/dev/null ||
    fail "OSC 7501 state=working did not show: $(seat_field "$seat" state.activity), grade $(seat_field "$seat" state.grade)"
[ "$(seat_field "$seat" state.subagents)" = '[]' ] || fail "untagged 7501 invented a subagent"

child=$(spawn_zsh) || fail "spawn refused"
run_line "$child" "printf '\\033]7501;state=working:app=e2e:id=t8\\007'; echo child-reported; read -r response"
wait_text "$child" '(?m)^child-reported$'
ctl seat wait "$child" --state active --subagent t8 --grade reported --timeout 30000 >/dev/null ||
    fail "tagged OSC 7501 did not show a reported active subagent: $(seat_field "$child" state.subagents)"
