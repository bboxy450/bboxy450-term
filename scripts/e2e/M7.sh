# coverage: full
. "$(dirname "$0")/lib.sh"
exec python3 -I -B "$(dirname "$0")/attention_checks.py"
