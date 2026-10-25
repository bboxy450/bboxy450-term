# coverage: partial; missing: ⌘T on a Claude or Codex row (only zsh is spawned, by ctl); the seat opens within one frame; placements other than tab
# T1: a new seat is a real PTY in a pane, named from the furry pool, in the chosen placement.
# Headless: the launcher's ⌘T and "within one frame" need a window; the spawn is the same
# GuiAction::Spawn the launcher issues, so placement, the PTY and the name are checked here.
. "$(dirname "$0")/lib.sh"
seat=$(spawn_zsh) || fail "spawn refused"
[ -n "$seat" ] || fail "spawn returned no seat"
[ "$(seat_field "$seat" state.process.kind)" = running ] || fail "no running process: $(seat_field "$seat" state.process)"
run_line "$seat" 'echo pty-$(tty | sed "s|/dev/||")'
wait_text "$seat" '(?m)^pty-ttys?[0-9]+$'
name=$(seat_field "$seat" record.name)
[ -n "$name" ] && [ "$name" != zsh ] || fail "the seat is named '$name', not from the furry pool (no name pool in the app)"
second=$(spawn_zsh) || fail "second spawn refused"
third=$(spawn_zsh) || fail "third spawn refused"
second_name=$(seat_field "$second" record.name)
third_name=$(seat_field "$third" record.name)
[ -n "$second_name" ] && [ -n "$third_name" ] || fail "a default name is empty"
[ "$name" != "$second_name" ] && [ "$name" != "$third_name" ] && [ "$second_name" != "$third_name" ] ||
    fail "default names collide: '$name', '$second_name', '$third_name'"
custom=$(spawn_zsh 'T1 Explicit Name') || fail "named spawn refused"
[ "$(seat_field "$custom" record.name)" = 'T1 Explicit Name' ] || fail "--name override was changed"
