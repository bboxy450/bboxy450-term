#!/bin/sh
# #327 B2: explicit real boundaries, never a blanket run of ignored custody tests.
set -eu
cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
export RUSTC_WRAPPER= RUSTC_WORKSPACE_WRAPPER= CARGO_BUILD_JOBS=2
export BBOXY_TEST_CUSTODY=memory
export CARGO_PROFILE_DEV_DEBUG_ASSERTIONS=true CARGO_PROFILE_TEST_DEBUG_ASSERTIONS=true
cargo test -p bbt-mux --locked --test socket_backlog socket_request_flood_replays_exact_output_and_delivers_exit_to_logic -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test lifecycle lifecycle_real_process_boundary -- --ignored --exact --nocapture
