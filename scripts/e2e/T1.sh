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
