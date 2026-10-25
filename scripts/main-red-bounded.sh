#!/bin/sh
# Invariant #322, using the same machine-wide state as land.sh/main-watch.sh.
set -eu
dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
exec python3 -I "$dir/landing_recovery.py" main-red-bounded "$@"
