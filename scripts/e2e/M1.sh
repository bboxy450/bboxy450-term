# coverage: partial; missing: from the composer
. "$(dirname "$0")/lib.sh"
exec python3 -I "$(dirname "$0")/messaging.py" M1
