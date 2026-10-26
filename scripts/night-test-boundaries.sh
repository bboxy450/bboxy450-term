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
cargo test -p bbt-mux --locked --lib identity::native_keychain::fixtures::private_native_keychain_round_trip_and_cleanup -- --ignored --exact --nocapture

# #327 supervisor: preserved native boundaries; exact parents, never helper entrypoints.
cargo test -p bbt-supervisor --locked --lib client::tests::a_peer_closed_after_its_final_reply_does_not_hide_buffered_bytes_behind_timeout_setup -- --ignored --exact --nocapture
cargo test -p bbt-supervisor --locked --lib client::tests::ring_notifications_survive_ten_thousand_requests_without_an_event_receive -- --ignored --exact --nocapture
cargo test -p bbt-supervisor --locked --lib supervisor::child_events::tests::a_live_child_cannot_make_the_shutdown_wait_infinite -- --ignored --exact --nocapture
cargo test -p bbt-supervisor --locked --lib supervisor::capture_read_tests::a_full_retained_capture_buffer_is_not_pty_eof -- --ignored --exact --nocapture
cargo test -p bbt-supervisor --locked --lib supervisor::capture_read_tests::exited_between_spawn_and_registration_emits_exactly_one_exit -- --ignored --exact --nocapture
cargo test -p bbt-supervisor --locked --lib supervisor::capture_read_tests::an_early_exiting_writer_is_drained_before_reaping_without_blocking_the_actor -- --ignored --exact --nocapture
cargo test -p bbt-supervisor --locked --lib supervisor::capture_read_tests::shutdown_closes_a_blocked_staged_pty_and_preserves_capture_and_one_exit -- --ignored --exact --nocapture
cargo test -p bbt-supervisor --locked --lib transport::tests::refused_socket_rebinds_without_replacing_a_live_socket_or_file -- --ignored --exact --nocapture
cargo test -p bbt-supervisor --locked --lib transport::tests::ownership_lock_persists_and_a_lockless_socket_is_probed -- --ignored --exact --nocapture
cargo test -p bbt-supervisor --locked --lib transport::tests::invalid_lock_types_permissions_and_owner_are_rejected_without_changes -- --ignored --exact --nocapture
cargo test -p bbt-supervisor --locked --lib transport::tests::full_listener_backlog_is_live_and_never_unlinked -- --ignored --exact --nocapture
cargo test -p bbt-supervisor --locked --lib transport::tests::fork_inherited_listener_is_live_until_the_child_releases_it -- --ignored --exact --nocapture
cargo test -p bbt-supervisor --locked --lib transport::tests::dropping_owner_releases_flock_while_a_fork_child_keeps_the_descriptor -- --ignored --exact --nocapture
cargo test -p bbt-supervisor --locked --test companions companion_survives_client_restart_and_is_reaped_before_seat_exit -- --ignored --exact --nocapture
cargo test -p bbt-supervisor --locked --test companions companion_failure_ends_the_seat_and_reaps_the_service -- --ignored --exact --nocapture
cargo test -p bbt-supervisor --locked --test companions unready_companion_never_holds_the_serial_client_or_recovery_inventory -- --ignored --exact --nocapture
cargo test -p bbt-supervisor --locked --test companions companion_dies_when_its_supervisor_process_is_killed -- --ignored --exact --nocapture
cargo test -p bbt-supervisor --locked --test journal_config capture_requires_an_explicit_supervisor_journal_directory -- --ignored --exact --nocapture
cargo test -p bbt-supervisor --locked --test journal_config supervisor_shutdown_preserves_every_already_admitted_byte_for_offline_reopen -- --ignored --exact --nocapture
cargo test -p bbt-supervisor --locked --test probes unconfigured_inputs_always_fence_the_context -- --ignored --exact --nocapture
cargo test -p bbt-supervisor --locked --test probes configured_probes_preserve_context_and_still_deliver_once -- --ignored --exact --nocapture
cargo test -p bbt-supervisor --locked --test probes guarded_inputs_use_the_same_probe_allowlist -- --ignored --exact --nocapture
cargo test -p bbt-supervisor --locked --test reaping final_output_and_exit_reach_the_owner -- --ignored --exact --nocapture
cargo test -p bbt-supervisor --locked --test reaping stdio_eof_does_not_wait_for_a_still_running_child -- --ignored --exact --nocapture
cargo test -p bbt-supervisor --locked --test supervisor zsh_echo_cwd_environment_and_resize -- --ignored --exact --nocapture
cargo test -p bbt-supervisor --locked --test supervisor two_seats_interleave_without_cross_talk -- --ignored --exact --nocapture
cargo test -p bbt-supervisor --locked --test supervisor spawn_errors_name_the_failed_paths_without_arguments_or_environment -- --ignored --exact --nocapture
cargo test -p bbt-supervisor --locked --test supervisor fifty_seats_share_one_supervisor_poller -- --ignored --exact --nocapture
cargo test -p bbt-supervisor --locked --test supervisor killed_child_reports_generation_and_fences_replacement -- --ignored --exact --nocapture
cargo test -p bbt-supervisor --locked --test supervisor logic_and_writer_replacement_preserve_child_and_deduplicate_input -- --ignored --exact --nocapture
cargo test -p bbt-supervisor --locked --test supervisor validation_rejects_stale_and_oversized_input -- --ignored --exact --nocapture
cargo test -p bbt-supervisor --locked --test supervisor a_dead_socket_without_a_lock_does_not_stop_the_supervisor -- --ignored --exact --nocapture
cargo test -p bbt-supervisor --locked --test supervisor unix_socket_auth_framing_and_live_pty -- --ignored --exact --nocapture
cargo test -p bbt-supervisor --locked --test supervisor blocked_write_survives_fencing_and_becomes_unknown_without_replay -- --ignored --exact --nocapture
cargo test -p bbt-supervisor --locked --test supervisor natural_exit_follows_all_final_output -- --ignored --exact --nocapture
cargo test -p bbt-supervisor --locked --test supervisor slow_reader_backpressures_and_recovers_the_complete_output -- --ignored --exact --nocapture
