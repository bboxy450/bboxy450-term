# coverage: partial; missing: approved blur pixels; harness labels and role presets; search results
# L1: the app's Cmd-T/Esc, produced launcher bounds and zero-spawn end state.
. "$(dirname "$0")/lib.sh"
exec python3 -I "$(dirname "$0")/ui.py" L1
