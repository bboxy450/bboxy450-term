---
type: bug
title: 'bbt-mux Dream C1 tests fail nondeterministically on a quiet machine: 1 ms CPU-per-unit budget overruns'
id: 01M731R4HQ2DCPPATMBJZ3F4A5
status: open
delegated_by: seat:01M71GNVXV7ZHXMGM50M5SB2RA
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-10T15:38:11.380Z
created: 2026-11-10T15:38:11.380Z
number: 1794292861627450
created_by: seat:01M71GNVXV7ZHXMGM50M5SB2RA
updated_by: seat:01M71GNVXV7ZHXMGM50M5SB2RA
authorship:
  signer: seat:01M71GNVXV7ZHXMGM50M5SB2RA
  kind: seat
  public_key: V5Zkz4W1rnaDG/FsjK3//xQX+WByyxD7/fuIYoV6NbU=
  process_generation: '1'
provenance:
  digest: sha256:3569a6bdac63c4ac181fe00e7198d5501091b70990a7eb8387879bb48b953d25
  frontmatter_digest: sha256:5d2f878b9adc783266e31e816094a23f1c0937c195a78fb2fff5d2fe640f7dd7
  signer: seat:01M71GNVXV7ZHXMGM50M5SB2RA
  sig: H+JHIj527Sdz/77/YRJgzcqkeHj3ttFtMJIWegN+cw63XXc+xXaIh8piHDU22v0JHMfar9cHuF0+x1fpFtdBAQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151483
  event:
    id: 01M731R55H5T0BYBD3ZRWJZ6SW
    sequence: 151483
    hash: 01846c27f6dce59cd937fdaef7bd1407005ed20eba9f59382c754f1f02f08377
---
# bbt-mux Dream C1 tests are flaky by themselves

## Expected

`cargo nextest run -p bboxy450-term-mux --lib` passes on `main` on this machine, so any
change whose lean plan includes the mux crate can land.

## Actual

The Dream C1 tests in `bbt-mux` fail a different subset on each run, with the machine
quiet. Any landing whose plan includes mux library tests fails most of the time.

The recurring message:

```
C1 unit overrun cpu_ns=1296583 file=crates/bbt-mux/src/registry/dream_c1.rs line=347
C1 journal init refused index=8 phase=first_step code=ResourceLimit
```

The budget is 1,000,000 ns of CPU per unit (`limit_ns=1000000`); observed overruns are
1.0 to 1.7 ms, in a debug-profile test binary.

## Evidence (2026-11-10, term worktree at 12f892605 + 8d607d078, harness pin 20e52d0)

| Run | Conditions | Result |
|---|---|---|
| Full lean gate, mux `--lib` | load 17, another full gate running | 10 failed, 1 timeout (300 s) |
| Full lean gate, mux `--lib` | load 5, no other gate | 22 failed |
| `-E 'test(/dream_c1/)' -j 2` | load 2.5, nothing else building | 91 run, 4 failed |
| `-E 'test(/dream_c1/)' -j 1` | same | at least 5 failed, different tests |
| `dream_c1_staged_journal_open_rejects_inode_replacement_and_symlink` alone, twice | load 10 | passed, passed |

All with the seat's `BBOXY_*` variables unset. Tests seen failing include, from
`registry::dream_c1::registry_tests`: `…durable_cas_rejects_stale_inspect_after_completed_publication`,
`…late_sql_failure_rolls_back_run_source_close_cursor_and_original_op`,
`…legacy_wal_transition_preserves_rows_and_busy_reader_never_changes_custody`,
`…retained_source_refuses_root_workspace_and_aba_revision_changes`,
`…schema_bootstrap_rolls_back_partial_ddl_and_ready_marker_before_recovery`; from
`logic::dream_public_api_tests`: `…held_provider_selection_refuses_real_mcp_gui_controls_within_250ms`,
`…actual_lost_cancel_reply_keeps_original_unknown_recovery_identity`.

Also in the same full-suite runs, not Dream: `logic::workspace_routing_tests::deferred_scan_replaces_failed_watcher_and_observes_later_edits`
failed twice at about 30 s.

## Reading

A hard 1 ms CPU budget per unit, measured in an unoptimized test binary, is being asked to
hold on a shared machine. It measures the machine as much as the code. This is the
"proven and measured stay separate" problem: a measured budget is acting as a pass/fail
unit test.

## What would fix it

For the Dream C1 owners to choose: assert the budget in a release-profile or dedicated
budget gate and keep the unit tests on logical outcomes; or scale the limit for debug
test builds; or have the tests count units rather than CPU nanoseconds. Until then these
tests cannot be a landing gate for unrelated changes.

## Related

- [[bugs/harness-python-execute-tests-leak-seat-context]]
- [[tasks/2026-11-10-land-plan-member-only-manifest]]: stops a new leaf crate from
  depending on these tests at all.
- `plugins::check::tests::task_contract_properties_derive_from_the_current_native_tool_catalogue`
  is deterministically red on `main` since 69a2eb19b added `task_close`; a two-line fix
  exists as commit 8d607d078 (branch `oracle/task-close-test-fix`), which cannot land
  while the tests above gate the mux crate.
