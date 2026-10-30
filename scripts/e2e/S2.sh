# coverage: full
# Torn-ring production is paired with this GUI witness by make test-history-gap.
. "$(dirname "$0")/lib.sh"
python3 -I "$(dirname "$0")/ui.py" S2
