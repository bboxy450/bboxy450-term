#!/bin/sh
# #327 B2: explicit real boundaries, never a blanket run of ignored custody tests.
set -eu
cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
parts=scripts/night-boundaries.d
[ -d "$parts" ] || { printf 'night boundaries: missing directory %s\n' "$parts" >&2; exit 2; }
set -- "$parts"/*.sh
[ -f "$1" ] || { printf 'night boundaries: no crate scripts in %s\n' "$parts" >&2; exit 2; }
export RUSTC_WRAPPER= RUSTC_WORKSPACE_WRAPPER= CARGO_BUILD_JOBS=2
export BBOXY_TEST_CUSTODY=memory
export CARGO_PROFILE_DEV_DEBUG_ASSERTIONS=true CARGO_PROFILE_TEST_DEBUG_ASSERTIONS=true
. ./scripts/cache-env.sh
# Shell pathname expansion visits the crate files in sorted order.
for part do
    sh "$part"
done
