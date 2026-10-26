# coverage: full
# harness: three real PTYs running zsh-exec Python programs, no native model claim
# T5: app-only SIGKILL/relaunch keeps programs and journal-restores geometry, focus and scroll.
. "$(dirname "$0")/lib.sh"
exec python3 -I -B "$(dirname "$0")/t5_relaunch.py"
