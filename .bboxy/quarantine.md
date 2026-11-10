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
| seat_plugin_calls_carry_their_own_attribution_and_operator_calls_none | bboxy450-term-mux | Vesper Greywind | flaky under full `make land` load on 2026-11-02; alone `cargo test -p bboxy450-term-mux --test plugin_attribution` 2 passed | 2026-11-02 | 2026-11-09 | land gate fail then solo pass, worktree bboxy450-term-rich-harness |
| `task_migration_250_records_authenticated_collision_restart_and_replay` | `bboxy450-term-mux` | Maksim Soltan | 10 s MCP transport deadline under `make land` load on 2026-11-02 (twice, unrelated diffs); alone `cargo nextest run -p bboxy450-term-mux --test mcp -E test(task_migration_250_records_authenticated_collision_restart_and_replay)` passed, first_publication 4.6 s | 2026-11-02 | 2026-11-09 | land gate fail then solo pass, worktrees provenance-pacing and mcp-seat-resume |
| `logic::task_import::tests::task_migration_held_import_keeps_messages_tasks_and_other_roots_live_and_refuses_stale_publication` | `bboxy450-term-mux` | Maksim Soltan | control-reply timeout (task_import.rs:366) under load 17–25 on 2026-11-03; fails on unmodified main 2d73acbd (2/2 alone, 33–41 s); passed alone once (41 s) | 2026-11-03 | 2026-11-10 | land gate fail twice on provenance-gaps, then main-check worktree solo runs |
| `index::tests::sibling_build_writes_do_not_dirty_watch_or_trigger_reconciliation` | bboxy450-term-mux | Vesper Greywind | [[#1794292861627479]] | 2026-11-10 | 2026-11-17 | isolated retry at f23a4df791a569a2e9f276b03e0c3f8afea328dc; /Users/feral/.local/state/bboxy450-term/landing/quarantine/2d508055a9944360875f8e92ab69b412.log |
