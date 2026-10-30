#!/bin/sh
# Exact real boundaries for bbt-mux; append new selectors here.
set -eu
cargo test -p bboxy450-term-mux --locked --test socket_backlog socket_request_flood_replays_exact_output_and_delivers_exit_to_logic -- --ignored --exact --nocapture
cargo test -p bboxy450-term-mux --locked --test identity_local pending_logic_keeps_pty_live_retries_at_30s_and_releases_durable_sends_in_order -- --ignored --exact --nocapture
cargo test -p bboxy450-term-mux --locked --test provenance custody_hold_keeps_observations_ephemeral_until_legitimate_projection_recovers -- --ignored --exact --nocapture
cargo test -p bboxy450-term-mux --locked --test provenance unchanged_clipped_git_refs_produce_no_activity_for_ten_seconds -- --ignored --exact --nocapture
cargo test -p bboxy450-term-mux --locked --test logic ring_seals_twenty_mib_with_logic_stopped_and_a_stalled_notification_queue_then_replays_without_gap -- --ignored --exact --nocapture
cargo test -p bboxy450-term-mux --locked --test messaging_flow custody_hold_gui_boundary -- --ignored --exact --nocapture
cargo test -p bboxy450-term-mux --locked --lib identity::native_keychain::fixtures::private_native_keychain_round_trip_and_cleanup -- --ignored --exact --nocapture
cargo test -p bboxy450-term-mux --locked --test provider_gui gui_spawn_roles_follow_the_catalog_and_default_shells_to_worker -- --ignored --exact --nocapture
cargo test -p bboxy450-term-mux --locked --test provider_gui operator_spawn_replays_across_connections_logic_and_supervisor_restart_without_relaunch -- --ignored --exact --nocapture
cargo test -p bboxy450-term-mux --locked --test provider_gui one_supervisor_routes_host_gui_states_and_keeps_remote_ptys_and_labels_after_logic_restart -- --ignored --exact --nocapture
cargo test -p bboxy450-term-mux --locked --test mcp mcp_seat_input_consumes_readiness_and_enter_is_explicit -- --ignored --exact --nocapture
cargo test -p bboxy450-term-mux --locked --test mcp mcp_seat_input_lost_receipt_is_unknown_and_never_repeated_after_restart -- --ignored --exact --nocapture
cargo test -p bboxy450-term-mux --locked --test mcp mcp_seat_reads_page_real_capture_and_refuse_stale_view_cursors -- --ignored --exact --nocapture
cargo test -p bboxy450-term-mux --locked --test mcp mcp_seat_spawn_runs_once_with_isolated_home_and_existing_wiring -- --ignored --exact --nocapture
cargo test -p bboxy450-term-mux --locked --test mcp mcp_seat_native_interrupt_is_bounded_audited_and_never_replayed -- --ignored --exact --nocapture
cargo test -p bboxy450-term-mux --locked --lib seat_socket::gui_tests::mute_reply_refreshes_authoritative_seat_before_receipt_without_events -- --ignored --exact --nocapture
cargo test -p bboxy450-term-mux --locked --test mcp mcp_oracle_split_runs_without_a_ui_and_replays_after_logic_restart -- --ignored --exact --nocapture
