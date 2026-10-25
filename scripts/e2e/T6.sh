# coverage: partial; missing: the app says so; resumes by session id (only the zsh respawn is proven)
# T6: the supervisor dies: the seat survives as a seat, the app restarts the supervisor and
# re-attaches it. A plain zsh seat respawns (d11, c1-live-seats §6 acceptance 2): same seat
# id, next process generation, the screen recovered (or a gap marked), a fresh shell that
# answers. The old shell dies with the PTY master by design; resume by session id is for
# harness seats and is not checked here. Headless: "the app says so" is not checked.
. "$(dirname "$0")/lib.sh"
seat=$(spawn_zsh) || fail "spawn refused"
run_line "$seat" 'echo before-$$'
wait_text "$seat" '(?m)^before-[0-9]+$'
shell=$(ctl seat screen "$seat" --human | sed -n 's/^before-\([0-9]*\)$/\1/p' | tail -1)
generation=$(ctl seat screen "$seat" | jget data.generation)
supervisor=$(pid supervisor)
[ -n "$supervisor" ] || fail "no supervisor pid recorded"
kill -9 "$supervisor" || fail "could not stop the supervisor $supervisor"
tries=0
until [ "$(pid supervisor)" != "$supervisor" ] && ctl seat ls >/dev/null 2>&1; do
    tries=$((tries + 1)); [ "$tries" -le 300 ] || fail "the supervisor did not come back"; sleep 0.2
done
tries=0
until now=$(ctl seat screen "$seat" 2>/dev/null | jget data.generation) && [ -n "$now" ] && [ "$now" -gt "$generation" ]; do
    tries=$((tries + 1)); [ "$tries" -le 150 ] || fail "seat $seat did not respawn past generation $generation (now ${now:-gone})"; sleep 0.2
done
ctl seat screen "$seat" --human | grep -Eq "^before-$shell\$|^\[gap [0-9]+\.\.[0-9]+: " ||
    fail "the respawned seat's screen lost what it showed before, with no gap marker"
run_line "$seat" 'echo after-$$'
wait_text "$seat" '(?m)^after-[0-9]+$' 30000
fresh=$(ctl seat screen "$seat" --human | sed -n 's/^after-\([0-9]*\)$/\1/p' | tail -1)
[ "$fresh" != "$shell" ] || fail "after-$fresh: the old shell answered; expected a respawned one"
note "seat $seat respawned: generation $generation -> $now, shell $shell -> $fresh"
