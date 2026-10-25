# coverage: partial; missing: displayed historical bytes are not exported by ctl (row-coordinate traversal is not a cell proof); bounded lazy history page metrics
# S1: drive the real pane from the top of a 20k-line fixture through the live tail.
. "$(dirname "$0")/lib.sh"
exec python3 -I "$(dirname "$0")/ui.py" S1
