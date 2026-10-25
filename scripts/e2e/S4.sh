# coverage: partial; missing: over-budget GUI viewport fixture (live Replica is capped at 32768 cells); history heap allocation accounting is not exported by ctl
# S4: a real core-valid large pane refuses history by name and stays live.
. "$(dirname "$0")/lib.sh"
exec python3 -I "$(dirname "$0")/ui.py" S4
