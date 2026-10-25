#!/bin/sh
# #323 §6: diagnose only the selected test, invariant or parity cases.
set -eu
dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
export RUSTC_WRAPPER='' RUSTC_WORKSPACE_WRAPPER='' CARGO_BUILD_JOBS=2
exec python3 -I "$dir/landing_recovery.py" culprit "$@"
