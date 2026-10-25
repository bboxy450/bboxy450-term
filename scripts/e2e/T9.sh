# coverage: partial; missing: a harness resumed by session id (only a zsh seat is restarted)
# T9: `ctl seat restart` gives a seat its next process generation: the same seat, generation
# + 1 live, the old process gone, its screen and history kept (or a gap marked), and a
# fresh shell that answers.
. "$(dirname "$0")/lib.sh"
seat=$(spawn_zsh) || fail "spawn refused"
run_line "$seat" 'echo before-$$'
wait_text "$seat" '(?m)^before-[0-9]+$'
shell=$(ctl seat screen "$seat" --human | sed -n 's/^before-\([0-9]*\)$/\1/p' | tail -1)
[ -n "$shell" ] || fail "no shell pid on the screen"
generation=$(ctl seat screen "$seat" | jget data.generation)
out=$(ctl seat restart "$seat") || fail "restart refused: $out"
from=$(printf '%s\n' "$out" | jget data.from_generation)
[ "$from" = "$generation" ] || fail "restart reported generation $from, the seat was at $generation"
tries=0
until now=$(ctl seat screen "$seat" 2>/dev/null | jget data.generation) && [ "$now" = $((generation + 1)) ]; do
    tries=$((tries + 1)); [ "$tries" -le 150 ] || fail "seat $seat never reached generation $((generation + 1)) (now ${now:-gone})"; sleep 0.2
done
[ "$(seat_field "$seat" record.id)" = "$seat" ] || fail "seat $seat left the roster"
tries=0
while kill -0 "$shell" 2>/dev/null; do
    tries=$((tries + 1)); [ "$tries" -le 50 ] || fail "the old shell $shell is still running"; sleep 0.1
done
ctl seat screen "$seat" --human | grep -Eq "^before-$shell\$|^\[gap [0-9]+\.\.[0-9]+: " ||
    fail "the restarted seat's screen lost what it showed before, with no gap marker"
run_line "$seat" 'echo after-$$'
wait_text "$seat" '(?m)^after-[0-9]+$' 30000
fresh=$(ctl seat screen "$seat" --human | sed -n 's/^after-\([0-9]*\)$/\1/p' | tail -1)
[ "$fresh" != "$shell" ] || fail "after-$fresh: the old shell answered"
note "seat $seat restarted: generation $generation -> $now, shell $shell -> $fresh"
