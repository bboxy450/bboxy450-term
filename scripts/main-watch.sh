#!/bin/sh
# bboxy450-term — https://github.com/bboxy450/bboxy450-term
# LicenseRef-Respectful-Slop-1.0-Draft; keep accurate source credit on reuse.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
# The Oracle starts one watcher; postsubmit does not occupy a presubmit slot.
set -eu
dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
if [ -f "$dir/cache-env.sh" ]; then
    . "$dir/cache-env.sh"
else
    export RUSTC_WRAPPER= RUSTC_WORKSPACE_WRAPPER=
fi
export CARGO_BUILD_JOBS=2
case "$(uname -s)" in
    Darwin) exec sh "$dir/qos.sh" default python3 -I "$dir/landing.py" main-watch "$@" ;;
    *) exec nice -n 10 python3 -I "$dir/landing.py" main-watch "$@" ;;
esac
