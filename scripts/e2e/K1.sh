# coverage: full
# K1: named Kitty press bytes and literal input observed by a real raw-mode PTY program.
. "$(dirname "$0")/lib.sh"
exec python3 -I -B "$(dirname "$0")/native_keys.py"
