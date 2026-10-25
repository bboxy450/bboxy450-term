#!/bin/sh
# The cache setup and every check execute inside the same 15-minute watchdog.
set -eu
dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
if [ "${1:-}" = --worker ]; then
    shift
    [ -f "$dir/cache-env.sh" ] || { echo 'presubmit: cache_env_missing' >&2; exit 2; }
    . "$dir/cache-env.sh"
    export CARGO_BUILD_JOBS=2
    exec python3 -I "$dir/landing.py" presubmit-worker "$@"
fi
exec python3 -I "$dir/landing.py" presubmit "$@"
