#!/bin/sh
# BUILD is capped at 30 minutes; test/invariant RUN time is bounded at 15 minutes.
set -eu
dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
if [ "${1:-}" = --worker ]; then
    shift
    if [ -f "$dir/cache-env.sh" ]; then
        . "$dir/cache-env.sh"
    else
        export RUSTC_WRAPPER= RUSTC_WORKSPACE_WRAPPER=
    fi
    export CARGO_BUILD_JOBS=2
    exec python3 -I "$dir/landing.py" presubmit-worker "$@"
fi
exec python3 -I "$dir/landing.py" presubmit "$@"
