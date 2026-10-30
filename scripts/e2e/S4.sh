# coverage: full
# S4: allocated core-valid over-budget fixture plus independent supported live pane.
. "$(dirname "$0")/lib.sh"
exec python3 -I "$(dirname "$0")/ui.py" S4
