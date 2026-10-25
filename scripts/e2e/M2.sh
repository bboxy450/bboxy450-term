# coverage: partial; missing: the role chip fans out to the exact seats holding the role, one receipt each, shown as one send; above 32 seats it refuses with a named reason
. "$(dirname "$0")/lib.sh"
exec python3 -I "$(dirname "$0")/messaging.py" M2
