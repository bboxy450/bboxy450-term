# coverage: full
# harness: installed interactive Claude and Codex, scripted local model; real zsh PTY
# T9: explicit ctl restarts retain seat IDs, native sessions/transcripts and history, with new process generations.
. "$(dirname "$0")/lib.sh"
exec python3 -I -B "$(dirname "$0")/t9_restart.py"
