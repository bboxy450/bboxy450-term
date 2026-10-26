# coverage: full
# T3: real SIGWINCH, actual stty sizes, and complete-grid GPU captures.
. "$(dirname "$0")/lib.sh"
exec python3 -I -B "$(dirname "$0")/t3_resize.py"
