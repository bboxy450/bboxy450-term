#!/bin/sh
# #323 §5. The implementation never forces a push or signs somebody else's cut.
set -eu
export GIT_TERMINAL_PROMPT=0
exec python3 -I "$(dirname "$0")/landing.py" land "$@"
