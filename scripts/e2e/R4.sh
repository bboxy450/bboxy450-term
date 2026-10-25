# coverage: partial; missing: cards carry the rim and the swell out of the box
. "$(dirname "$0")/lib.sh"
exec python3 -I "$(dirname "$0")/rim_more.py" R4
