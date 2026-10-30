# coverage: full
# K3: Cmd-G selects the real Graph page and Escape restores the terminal focus.
. "$(dirname "$0")/lib.sh"
exec python3 -I -B "$(dirname "$0")/keyboard_checks.py" K3
