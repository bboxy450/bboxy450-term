#!/bin/sh
# The Oracle starts one watcher; postsubmit does not occupy a presubmit slot.
set -eu
dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
. "$dir/cache-env.sh"
export CARGO_BUILD_JOBS=2
exec nice -n 10 python3 -I "$dir/landing.py" main-watch "$@"
