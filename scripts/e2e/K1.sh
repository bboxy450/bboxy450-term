# coverage: partial; missing: every platform key combination and modifier; native press/repeat/release and IME/composition; Cmd-Opt navigation through the app
# K1: named Kitty press bytes and literal input observed by a real raw-mode PTY program.
. "$(dirname "$0")/lib.sh"
exec python3 -I -B "$(dirname "$0")/keyboard_checks.py" K1
