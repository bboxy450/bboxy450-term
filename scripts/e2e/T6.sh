# coverage: full
# harness: installed interactive Claude and Codex, scripted local model; real zsh PTY
# T6: stable seats, new process generations, native session/transcript resume and painted restart chips.
. "$(dirname "$0")/lib.sh"
exec python3 -I -B "$(dirname "$0")/t6_recover.py"
