#!/bin/sh
# #327 B2: explicit real boundaries, never a blanket run of ignored custody tests.
set -eu
cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
export RUSTC_WRAPPER= RUSTC_WORKSPACE_WRAPPER= CARGO_BUILD_JOBS=2
export BBOXY_TEST_CUSTODY=memory
export CARGO_PROFILE_DEV_DEBUG_ASSERTIONS=true CARGO_PROFILE_TEST_DEBUG_ASSERTIONS=true
. ./scripts/cache-env.sh
cargo test -p bbt-mux --locked --test socket_backlog socket_request_flood_replays_exact_output_and_delivers_exit_to_logic -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test lifecycle lifecycle_real_process_boundary -- --ignored --exact --nocapture
cargo test -p bbt-mux --locked --test identity_local pending_logic_keeps_pty_live_retries_at_30s_and_releases_durable_sends_in_order -- --ignored --exact --nocapture
cargo test -p bbt-mux --locked --test provenance custody_hold_keeps_observations_ephemeral_until_legitimate_projection_recovers -- --ignored --exact --nocapture
cargo test -p bbt-mux --locked --test provenance unchanged_clipped_git_refs_produce_no_activity_for_ten_seconds -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test custody custody_hold_keeps_gui_echo_live_and_releases_held_sends_on_retry -- --ignored --exact --nocapture
cargo test -p bbt-mux --locked --test logic ring_seals_twenty_mib_with_logic_stopped_and_a_stalled_notification_queue_then_replays_without_gap -- --ignored --exact --nocapture
cargo test -p bbt-mux --locked --test messaging_flow custody_hold_gui_boundary -- --ignored --exact --nocapture
