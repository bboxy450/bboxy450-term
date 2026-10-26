# coverage: full
# T4: actual styles, colours, wide-grid placement and cursor pixels.
. "$(dirname "$0")/lib.sh"
exec python3 -I -B "$(dirname "$0")/t4_cells.py"
