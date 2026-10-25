#!/bin/sh
# #323 §8: scheduled scale measurement, after the e2e night; no quiet window.
set -eu
dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
export RUSTC_WRAPPER='' RUSTC_WORKSPACE_WRAPPER='' CARGO_BUILD_JOBS=2
exec python3 -I "$dir/landing_recovery.py" scale-night "$@"
