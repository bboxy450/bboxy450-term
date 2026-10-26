# coverage: full
# T2: typing reaches the program and echoes; a 100 kB paste arrives intact.
. "$(dirname "$0")/lib.sh"
exec python3 -I -B "$(dirname "$0")/t2_input.py"
