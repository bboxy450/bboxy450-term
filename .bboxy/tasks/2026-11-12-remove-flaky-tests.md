---
type: task
title: 'Remove flaky tests: the quarantined six and the swarm_workdir timing witnesses'
verify: make land
allowed_paths:
- crates/bbt-mux/**
- crates/bboxy450-term/**
- Makefile
- verification/**
- scripts/**
- .bboxy/quarantine.md
- .bboxy/specs/**
- LAND.md
id: 01M78P5SGHVRK4149PTHN65TVP
status: open
delegated_by: seat:01M78E743PF9RK3DA9Y3WRWTWD
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-12T20:11:23.317Z
created: 2026-11-12T20:11:23.317Z
number: 1794292861627494
created_by: seat:01M78E743PF9RK3DA9Y3WRWTWD
updated_by: seat:01M78E743PF9RK3DA9Y3WRWTWD
authorship:
  signer: seat:01M78E743PF9RK3DA9Y3WRWTWD
  kind: seat
  public_key: lZh8jo1S9rWtfvb9+EUefnBFfbJ6FtBuf6dXC19gMpk=
  process_generation: '1'
provenance:
  digest: sha256:046b81929735e7db9fdf146da1a3e9997a8b03cbd41fd09a5bdc847adedc4770
  frontmatter_digest: sha256:1d694cdad748bd2fe0ede64fa58aca049b5bf5e326ea70197b76ce26ef6850b4
  signer: seat:01M78E743PF9RK3DA9Y3WRWTWD
  sig: kOzAS9+zO2OIhOjozycnZT39kdWbBiUX1AdiGqSzDidONv5IjMlnCovLqc+mdobUnzTnuVLP0ygkQuDUNmJxDw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151642
  event:
    id: 01M78P5VKD0ZDYBWD219FPQR76
    sequence: 151642
    hash: 0e9e4eac91995bc4001510704a61ae03a16d246b11f88124340a62325a6ca6d6
---
## Goal

Operator, 2026-11-12: "remove flaky tests", "reliability over wasted inference".

A test that fails under load and passes alone costs a landing rerun (about 50 minutes of
machine and an agent's attention) every time it fires, and trains everyone to ignore red.
Delete the tests with recorded flake evidence. Do not rewrite them and do not quarantine
them again.

## Scope

Tests with evidence, to delete with any helper that only they use:

| Test | Evidence |
|---|---|
| `seat_plugin_calls_carry_their_own_attribution_and_operator_calls_none` (mux `tests/plugin_attribution.rs`) | quarantined 2026-11-02, expired |
| `task_migration_250_records_authenticated_collision_restart_and_replay` (mux `tests/mcp.rs`) | quarantined 2026-11-02, expired |
| `task_migration_held_import_keeps_messages_tasks_and_other_roots_live_and_refuses_stale_publication` (mux `logic/task_import.rs`) | quarantined 2026-11-03, expired |
| `sibling_build_writes_do_not_dirty_watch_or_trigger_reconciliation` (mux `index.rs`) | quarantined 2026-11-10; failed twice in retained logs |
| `canvas_native_mcp_three_batches_restart_sidebar_and_headless_capture` (app `tests/headless/canvas.rs`) | quarantined 2026-11-10 |
| `deferred_scan_replaces_failed_watcher_and_observes_later_edits` (mux `knowledge/routing_tests.rs`) | quarantined 2026-11-10; flaked again in the 2026-11-12 landing |
| `swarm_workdir::launch_context::launch_children_receive_workflow_and_call_their_own_admitted_mcp` (app) | 6 failures in retained logs |
| `swarm_workdir::rich_detach::rich_gui_handoff_preserves_turn_and_rolls_back_failed_activation` (app) | 3 failures in retained logs |
| `swarm_workdir::rich_detach::rich_app_routes_composer_approval_scroll_and_focus` (app) | 1 failure, 1 flaky pass in retained logs |

Then empty the quarantine register, and remove every reference (Makefile filters,
`verification/contracts.toml` witnesses, LAND.md timing lists) so nothing names a test
that no longer exists.

## Acceptance criteria

- [ ] None of the nine tests exists; no file names one.
- [ ] `.bboxy/quarantine.md` lists no test.
- [ ] Any contract that loses its only implementation witness is named in the report.
- [ ] `make land` exits 0.

## Verification

`make land`
