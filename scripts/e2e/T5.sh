# T5: the app is killed and relaunched: every seat is on the same PTY and the program never
# noticed. Headless: layout, focus and scroll positions are the window's and are not checked.
. "$(dirname "$0")/lib.sh"
seat=$(spawn_zsh) || fail "spawn refused"
run_line "$seat" 'echo before-$$'
wait_text "$seat" '(?m)^before-[0-9]+$'
shell=$(ctl seat screen "$seat" --human | sed -n 's/^before-\([0-9]*\)$/\1/p' | tail -1)
logic=$(pid logic)
[ -n "$logic" ] || fail "no logic pid recorded"
kill -9 "$logic" || fail "could not stop logic $logic"
tries=0
until [ "$(pid logic)" != "$logic" ] && ctl seat ls >/dev/null 2>&1; do
    tries=$((tries + 1)); [ "$tries" -le 300 ] || fail "logic did not come back"; sleep 0.2
done
run_line "$seat" 'echo after-$$'
wait_text "$seat" "(?m)^after-$shell\$" 30000
ctl seat screen "$seat" --human | grep -q "^before-$shell\$" || fail "the screen lost what it showed before the restart"
note "layout, focus and scroll positions need a window; this run is headless"
