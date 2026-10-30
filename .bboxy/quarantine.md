---
id: 01M5SYC0G6XJG24W8S19Y730G8
type: quarantine
number: 324
title: "flaky quarantine: tests that do not block while being fixed, each with owner, task, date and expiry; still run, still logged; failing alone is a failure"
status: active
links:
  - { to: "[[landing]]", rel: spec }
  - { to: "[[d321-out-of-integration-hell]]", rel: decision }
created: "2026-10-25T16:30:16Z"
updated: "2026-10-25T16:30:16Z"
---
# Flaky quarantine (#321 rule 4)

A test is listed here only with an owner, a task and a date. It still runs everywhere. When a run's
only failures are listed tests, each is rerun once alone by `scripts/quarantine-test.sh`; passing alone
does not fail the run, failing alone does. An entry leaves when fixed or after seven days, whichever is
first; more than five entries is a review, not a fact of life. The name column is matched exactly
against cargo's `test <name> ... FAILED` line.

| Test | Crate | Owner | Task | Since | Expires | Evidence |
|---|---|---|---|---|---|---|
| `reference_provider_schemas_fit_a_multi_provider_inventory` | bboxy450-term-mux | Vesper Greywind | [[#377]] | 2026-10-30 | 2026-11-06 | Python fixture plugin exceeds its fixed 2 s start/initialize budget only under machine load 8-11 inside the parallel full gate; passes alone (3/3, ~3 s at load 8.6). Removed by Rust fixture plugins (task #377 P9). |
| `plugins::client::tests::demultiplexed_replies_reach_their_callers_and_plugin_requests_are_answered` | bboxy450-term-mux | Vesper Greywind | [[#377]] | 2026-10-30 | 2026-11-06 | Python fixture plugin exceeds its fixed 2 s start/initialize budget only under machine load 8-11 inside the parallel full gate; passes alone (3/3, ~3 s at load 8.6). Removed by Rust fixture plugins (task #377 P9). |
| `plugins::check::tests::cached_descriptor_survives_a_host_reload_and_a_new_digest_has_no_cache` | bboxy450-term-mux | Vesper Greywind | [[#377]] | 2026-10-30 | 2026-11-06 | Python fixture plugin exceeds its fixed 2 s start/initialize budget only under machine load 8-11 inside the parallel full gate; passes alone (3/3, ~3 s at load 8.6). Removed by Rust fixture plugins (task #377 P9). |
| `plugins::client::tests::progress_cannot_extend_a_deadline_and_cancellation_does_not_replay` | bboxy450-term-mux | Vesper Greywind | [[#377]] | 2026-10-30 | 2026-11-06 | Python fixture plugin exceeds its fixed 2 s start/initialize budget only under machine load 8-11 inside the parallel full gate; passes alone (3/3, ~3 s at load 8.6). Removed by Rust fixture plugins (task #377 P9). |
| `plugins::client::tests::real_stdio_keeps_opaque_signed_bytes_and_only_allowed_environment_names` | bboxy450-term-mux | Vesper Greywind | [[#377]] | 2026-10-30 | 2026-11-06 | Python fixture plugin exceeds its fixed 2 s start/initialize budget only under machine load 8-11 inside the parallel full gate; passes alone (3/3, ~3 s at load 8.6). Removed by Rust fixture plugins (task #377 P9). |
| `the_other_five_contracts_are_probed_and_cross_field_results_are_checked` | bboxy450-term-mux | Vesper Greywind | [[#377]] | 2026-10-30 | 2026-11-06 | Fresh pinned plugin copy exceeds the fixed 2 s --shape budget only inside the parallel full gate under load 7-11 (earlier sample: new copies parked at _dyld_start, consistent with macOS first-exec scanning); passes alone. Over-five review: task #377 P9 fixture follow-up makes fixture budgets injectable and measures first-exec latency. |
| `six_required_refusals_are_named_and_never_leave_a_child` | bboxy450-term-mux | Vesper Greywind | [[#377]] | 2026-10-30 | 2026-11-06 | Fresh pinned plugin copy exceeds the fixed 2 s --shape budget only inside the parallel full gate under load 7-11 (earlier sample: new copies parked at _dyld_start, consistent with macOS first-exec scanning); passes alone. Over-five review: task #377 P9 fixture follow-up makes fixture budgets injectable and measures first-exec latency. |
| `protocol_errors_command_errors_eof_bad_results_and_fixed_deadlines_stay_distinct` | bboxy450-term-mux | Vesper Greywind | [[#377]] | 2026-10-30 | 2026-11-06 | Fresh pinned plugin copy exceeds the fixed 2 s --shape budget only inside the parallel full gate under load 7-11 (earlier sample: new copies parked at _dyld_start, consistent with macOS first-exec scanning); passes alone. Over-five review: task #377 P9 fixture follow-up makes fixture budgets injectable and measures first-exec latency. |
