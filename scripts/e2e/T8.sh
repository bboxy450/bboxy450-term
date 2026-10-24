# T8: OSC 7501 state shows as the reported grade; a silent seat shows idle, never invented.
. "$(dirname "$0")/lib.sh"
quiet=$(spawn_zsh) || fail "spawn refused"
run_line "$quiet" 'echo quiet'
wait_text "$quiet" '(?m)^quiet$'
sleep 2
[ "$(seat_field "$quiet" state.activity.kind)" = idle ] || fail "a silent seat is not idle: $(seat_field "$quiet" state.activity)"
seat=$(spawn_zsh) || fail "spawn refused"
run_line "$seat" "printf '\\033]7501;state=working:app=e2e:id=t8\\007'; echo reported"
wait_text "$seat" '(?m)^reported$'
tries=0
until [ "$(seat_field "$seat" state.activity.kind)" = active ]; do
    tries=$((tries + 1))
    [ "$tries" -le 25 ] || fail "OSC 7501 state=working did not show: $(seat_field "$seat" state.activity), grade $(seat_field "$seat" state.grade)"
    sleep 0.2
done
