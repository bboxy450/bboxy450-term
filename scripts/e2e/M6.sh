# coverage: partial; missing: priority delays, never deletes
. "$(dirname "$0")/lib.sh"
exec python3 -I "$(dirname "$0")/messaging.py" M6
