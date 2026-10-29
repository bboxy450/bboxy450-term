# coverage: full
. "$(dirname "$0")/lib.sh"
exec python3 -I -B "$(dirname "$0")/rim_more.py" R2
