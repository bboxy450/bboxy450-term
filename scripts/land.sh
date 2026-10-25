#!/bin/sh
# #323 §5. The implementation never forces a push or signs somebody else's cut.
# Usage: sh scripts/land.sh [--gate-log PATH]
# A supplied log must be a completed full normal ci-local run on the exact wide
# or previously measured narrow-cold (#325, temporary) candidate.
# candidate SHA, including ci-local's source/result lines; filenames are not proof.
set -eu
export GIT_TERMINAL_PROMPT=0
exec python3 -I "$(dirname "$0")/landing.py" land "$@"
