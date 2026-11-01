#!/bin/sh
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# LicenseRef-Respectful-Slop-1.0-Draft; credit copied/adapted/translated reuse.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
# #323 §5. The implementation never forces a push or signs somebody else's cut.
# Usage: sh scripts/land.sh [--full | --gate-log PATH]
# A supplied log must be a completed full normal ci-local run on the exact
# candidate SHA, including ci-local's source/result lines; filenames are not proof.
set -eu
export GIT_TERMINAL_PROMPT=0
exec python3 -I "$(dirname "$0")/landing.py" land "$@"
