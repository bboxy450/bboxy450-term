# T6: the supervisor dies: seats survive, the app restarts it and re-attaches. Headless: "the
# app says so" in the window is not checked; resume by session id is for harness seats.
. "$(dirname "$0")/lib.sh"
seat=$(spawn_zsh) || fail "spawn refused"
run_line "$seat" 'echo before-$$'
wait_text "$seat" '(?m)^before-[0-9]+$'
shell=$(ctl seat screen "$seat" --human | sed -n 's/^before-\([0-9]*\)$/\1/p' | tail -1)
supervisor=$(pid supervisor)
[ -n "$supervisor" ] || fail "no supervisor pid recorded"
kill -9 "$supervisor" || fail "could not stop the supervisor $supervisor"
tries=0
until [ "$(pid supervisor)" != "$supervisor" ] && ctl seat ls >/dev/null 2>&1; do
    tries=$((tries + 1)); [ "$tries" -le 300 ] || fail "the supervisor did not come back"; sleep 0.2
done
kill -0 "$shell" 2>/dev/null || fail "the seat's shell $shell died with the supervisor"
run_line "$seat" 'echo after-$$'
wait_text "$seat" "(?m)^after-$shell\$" 30000
