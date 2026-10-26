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
cargo test -p bboxy450-term --locked --test r5_stall synthetic_runtime_contract_keeps_fifteen_encoded_panes_live_under_mixed_io -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test e2e_night the_runner_turns_script_exit_codes_into_verdicts_and_a_summary -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test headless explicit_placements_capture_the_anchor_preserve_siblings_and_acknowledge_the_pane_frame -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test headless recording_notifications_replace_mute_and_activate_the_exact_headless_pane -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test headless headless_cli_starts_real_gui_types_captures_and_stops_without_a_window -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test headless t7_headless_close_receipts_restore_the_same_tab_and_extent -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test headless headless_smoke_uses_the_idle_probe_with_no_window_or_operator -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test headless headless_consumes_launch_results_so_ten_seats_are_placed_and_capturable -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test headless idle_probe_rejects_an_external_lifecycle_wake -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test headless unattended_headless_control_uses_its_own_file_account -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test headless unattended_idle_smoke_keeps_file_custody_without_a_window -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test headless ctl_ui_relays_real_chrome_layout_focus_and_a_foreground_launcher -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test headless ctl_ui_keys_acknowledge_spawn_mute_and_undo_only_after_their_end_state_frame -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test headless seat_wait_reads_produced_frames_and_not_on_screen_names_its_frame -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test headless ui_clock_refuses_a_non_test_account_even_with_a_forged_issuer -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test headless ctl_ui_scroll_reads_the_produced_historical_view_without_changing_focus -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test headless app_resize_replies_with_the_first_frame_at_the_new_size -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test headless test_account_ui_clock_freezes_pixels_steps_focus_and_resumes_live_time -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test headless retained_gap_fixture_requires_actual_test_account_and_preserves_live_bytes -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test plugins real_binary_runs_headless_against_a_fixture_without_opening_the_production_root -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --bin bboxy450-term live::tests::live_e2e_and_ctl_share_one_corpus_and_refresh_from_document_events -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --bin bboxy450-term live::tests::real_io_echo_resize_reflow_and_logic_loss_retains_last_frame -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --bin bboxy450-term live::tests::real_io_scrollback_reaches_first_line_joins_live_and_releases_on_input -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --bin bboxy450-term stall::load::tests::gui_errors_keep_the_operation_and_identify_the_consumer_path -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --bin bboxy450-term stall::load::tests::gui_keeps_applying_and_acknowledging_while_side_finish_is_held -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test cockpit app_io_owner_observes_both_child_process_exits -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test cockpit cancelled_exit_watch_preserves_orderly_child_shutdown -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test cockpit gui_endpoint_keeps_private_modes_and_rejects_bad_admission_and_frames -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test cockpit real_processes_list_spawn_and_push_unwatched_exit_state_on_one_gui_connection -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test cockpit same_run_directory_relaunches_and_spawns_after_orderly_shutdown -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test ctl_records ctl_events_discover_filter_and_resume_without_a_second_consumer_or_ack -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test ctl_records ctl_record_read_keeps_the_endpoint_exit_when_the_socket_closes_mid_request -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test ctl_records ctl_records_fetch_search_pages_and_changes_use_the_private_provider -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test messaging_flow composed_decision_and_task_delegation_cross_the_real_gui_boundary -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test recovery logic_death_keeps_pty_generation_rewatches_and_preserves_ack_backpressure -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test recovery partial_startup_survives_cancelled_poll_and_shutdown_reaps_its_child -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test recovery recovered_generation_admission_rejects_unknown_and_conflicting_launches_but_exact_retry_is_idempotent -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test recovery shutdown_during_pending_recovery_reaps_children_without_restarting -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test recovery supervisor_death_replays_ring_with_gap_same_ids_new_generation_and_native_resume_argv -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test test_account logic_restart_keeps_the_test_root_device_pty_and_operator_spawn_authority -- --ignored --exact --nocapture
cargo test -p bboxybook --locked --test documents every_tracked_bboxy_markdown_lays_out_in_both_themes_without_unclipped_overflow -- --ignored --exact --nocapture
cargo test -p bbt-mux --locked --test provider_gui gui_spawn_roles_refuse_codex_oracle_authority_and_default_shells_to_worker -- --ignored --exact --nocapture
cargo test -p bbt-mux --locked --test provider_gui operator_spawn_replays_across_connections_logic_and_supervisor_restart_without_relaunch -- --ignored --exact --nocapture
cargo test -p bbt-mux --locked --test provider_gui one_supervisor_routes_host_gui_states_and_keeps_remote_ptys_and_labels_after_logic_restart -- --ignored --exact --nocapture
cargo test -p bbt-mux --locked --test mcp mcp_seat_input_consumes_readiness_and_enter_is_explicit -- --ignored --exact --nocapture
cargo test -p bbt-mux --locked --test mcp mcp_seat_input_lost_receipt_is_unknown_and_never_repeated_after_restart -- --ignored --exact --nocapture
cargo test -p bbt-mux --locked --test mcp mcp_seat_reads_page_real_capture_and_refuse_stale_view_cursors -- --ignored --exact --nocapture
cargo test -p bbt-mux --locked --test mcp mcp_seat_spawn_runs_once_with_isolated_home_and_existing_wiring -- --ignored --exact --nocapture
cargo test -p bbt-mux --locked --test mcp mcp_seat_native_interrupt_is_bounded_audited_and_never_replayed -- --ignored --exact --nocapture
