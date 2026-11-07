#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Explicit measurement inventory, shared by Cargo and per-test nextest policy."""
import ctypes
import json
import os
from pathlib import Path
import sys

TIMED_TARGETS = {"gates_seats", "input_latency", "dream_c1", "dream_replay",
                 "gui_budget", "formal_contracts", "capture_fairness"}
TIMED_FILTERS = {
    "dream::c1::", "gpu::streaming_tests::",
    "lazy_projection_tests::input_frame_cpu_profile", "repeated_text_frame_cpu_profile",
    "rich_faces::budget_verification::rich_four_view_aggregate_budget",
    "logic::idle_command_tests", "seat_socket::input_check_tests",
    "seat_socket::gui_tests::busy_gui_poll_and_locked_cursor_keep_the_session_and_queued_events",
    "logic::ring_batch_tests",
}
# Load-family ignores recorded by the C1 source landing. These ten process/API
# witnesses retain their original CPU and wall assertions when re-enabled.
C1_TIMED_TESTS = {
    "actual_private_process_clears_authority_environment_and_uses_fixed_argv",
    "actual_private_prepared_start_is_one_write_and_duplicate_closes_without_replay",
    "actual_private_control_invalid_ulids_and_unsafe_revision_emit_no_new_frame",
    "actual_private_process_rejects_blank_invalid_utf8_unknown_duplicate_and_oversized_frames",
    "eof_and_drop_retain_exact_child_custody_through_actual_reap",
    "custody_copies_exact_ir_and_only_typed_blob_carriers_into_private_new_inodes",
    "dream_c1_source_failure_is_bounded_immutable_and_fenced_before_any_replay",
    "dream_c1_source_error_capacity_is_atomic_across_real_sqlite_owners",
    "dream_c1_admitted_mcp_is_exact_own_seat_and_reports_honest_missing_runs",
    "dream_c1_held_private_delete_catalogue_keeps_mcp_gui_control_owner_independent",
}


def timed_test(binary, name):
    target = binary.split("::")[-1]
    return (target in TIMED_TARGETS or name.split("::")[-1] in C1_TIMED_TESTS
            or any(part in name for part in TIMED_FILTERS)
            or (binary.startswith("bboxy450-harness-host")
                and name.startswith(("client::tests::", "transport::tests::"))))


def run_test(arguments):
    # nextest sets these for execution; --list has no per-test name. Its
    # controller stays unclamped so each actual test can choose independently.
    triple = "aarch64-apple-darwin" if os.uname().machine == "arm64" else "x86_64-apple-darwin"
    if arguments[:1] == ["--target-triple"]:
        triple, arguments = arguments[1], arguments[2:]
    name = os.environ.get("NEXTEST_TEST_NAME", "")
    binary = os.environ.get("NEXTEST_BINARY_ID", Path(arguments[0]).name.rsplit("-", 1)[0])
    role = "default" if timed_test(binary, name) else "utility"
    if arguments[:1] == ["--exec"]:
        arguments = arguments[1:]
    else:
        runners = json.loads(os.environ.get("BBOXY_QOS_INNER_TEST_RUNNERS", "{}"))
        inner = runners.get(triple, [])
        arguments = [*inner, *arguments]
        if sys.platform == "darwin":
            libc = ctypes.CDLL(None, use_errno=True)
            if libc.setpriority(4, os.getpid(), 0):
                raise OSError(ctypes.get_errno(), "cannot clear inherited Darwin background policy")
            if role == "utility":
                # A second unprotected Python restores DYLD after taskpolicy,
                # whose protected executable strips it on macOS.
                os.execv("/usr/sbin/taskpolicy", ["taskpolicy", "-c", "utility", sys.executable,
                         "-I", str(Path(__file__).resolve()), "--exec", *arguments])
        elif role == "utility":
            os.execvp("nice", ["nice", "-n", "10", *arguments])
    environment = dict(os.environ)
    # Runner policy belongs to this nextest test, not Cargo/app descendants.
    previous = json.loads(environment.pop("BBOXY_QOS_PREVIOUS_TEST_RUNNERS", "{}"))
    environment.pop("BBOXY_QOS_INNER_TEST_RUNNERS", None)
    for key, value in previous.items():
        if value:
            environment[key] = value
        else:
            environment.pop(key, None)
    for key, value in os.environ.items():
        if key.startswith("NEXTEST_DYLD_"):
            environment[key.removeprefix("NEXTEST_")] = value
    os.execvpe(arguments[0], arguments, environment)


if __name__ == "__main__":
    run_test(sys.argv[1:])
