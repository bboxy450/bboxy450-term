#!/bin/sh
# Exact real boundaries for bboxy450-term; append new selectors here.
# Origin/credit: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
set -eu
cargo test -p bboxy450-term --locked --test swarm_workdir sidebar_vertical_splits_resize_save_and_restore_without_restarting_harnesses -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test swarm_workdir settings_uses_full_window_height_after_resizes -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test lifecycle lifecycle_real_process_boundary -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test custody custody_hold_keeps_gui_echo_live_and_releases_held_sends_on_retry -- --ignored --exact --nocapture
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
cargo test -p bboxy450-term --locked --test headless finder_root_cwd_starts_logic_in_an_owned_writable_workspace -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test swarm_workdir oracle_mcp_splits_focuses_and_reattaches_without_restarting_harnesses -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --lib cockpit::gui_io_tests::refresh_replaces_a_confirmed_failed_process_without_requiring_clean_shutdown -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test swarm_workdir large_terminal_snapshot_survives_resize_and_ui_reattach -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test swarm_workdir new_swarm_cwd_controls_stay_visible_and_launch_in_the_saved_folder -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test swarm_workdir sidebar_drag_reflows_and_persists_without_restarting_harnesses -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test swarm_workdir swarm_switches_wait_for_membership_and_dim_real_inactive_seats -- --ignored --exact --nocapture
cargo test -p bboxy450-term --locked --test swarm_workdir sidebar_accordions_toggle_without_focus_changes_and_survive_ui_restart -- --ignored --exact --nocapture
