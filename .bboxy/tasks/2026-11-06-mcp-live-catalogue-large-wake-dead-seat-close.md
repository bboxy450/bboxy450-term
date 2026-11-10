---
type: task
title: 'bboxy MCP: catalogue follows grant epoch, large-brief delegate/reassign, close dead seats'
role: executor
worktree: isolated term worktree from origin/main (Oracle supplies path at delegation)
allowed_paths:
- crates/bbt-mux/src/mcp/
- crates/bbt-mux/src/logic.rs
- crates/bbt-mux/src/logic/
- crates/bbt-mux/tests/
- verification/
- ATTRIBUTION.md
- .bboxy/reports/
- crates/bbt-mux/src/models.rs
- crates/bbt-mux/models/
verify: cargo nextest run -p bboxy450-term-mux -E 'not (test(local_document_commit_keeps_unrelated_staging_and_refuses_no_change_and_symlinks) | test(held_private_git_hook_keeps_other_swarm_input_resize_and_status_live) | test(priority_burst_overflow_delivers_every_message_once_in_each_priority_fifo) | test(mux_document_writes_stamp_and_sign_digests_and_file_watch_observes_real_changes))' && make test-formal-contracts
status: accepted
id: 01M6S03Q9WAV4XXZBMWW7HB5DZ
delegated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-06T19:59:23.475Z
created: 2026-11-06T17:57:09.406Z
number: 93377961366057
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
updated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
authorship:
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  kind: seat
  public_key: DbWCjlMnSNUcBZMB2JrdfJ36/iQgSFRbtz78XE3WlhU=
  process_generation: '1'
verify_revision: 3
assignee: seat:01M6S0BP5BV4P07WSJSP2X6M27
verify_result:
  command: cargo nextest run -p bboxy450-term-mux -E 'not (test(local_document_commit_keeps_unrelated_staging_and_refuses_no_change_and_symlinks) | test(held_private_git_hook_keeps_other_swarm_input_resize_and_status_live) | test(priority_burst_overflow_delivers_every_message_once_in_each_priority_fifo) | test(mux_document_writes_stamp_and_sign_digests_and_file_watch_observes_real_changes))' && make test-formal-contracts
  exit_code: 0
  evidence:
  - criterion: Live grant changes notify and refresh tools; downgraded calls refuse
    reference: crates/bbt-mux/tests/mcp/live_catalogue.rs:11; /tmp/mcp-live-verify-landed.log — exit 0
  - criterion: 40 KiB delegate and reassign keep path and 16 KiB wake bound
    reference: crates/bbt-mux/tests/mcp/live_catalogue.rs:87; /tmp/mcp-live-verify-landed.log — exit 0
  - criterion: Absent viewport close retains record; live owner and wrong generation refuse
    reference: crates/bbt-mux/tests/mcp/live_catalogue.rs:128; /tmp/mcp-live-verify-landed.log — exit 0
  - criterion: Omitted host stamps survive; protected field differences are named
    reference: crates/bbt-mux/tests/mcp/live_catalogue.rs:190; /tmp/mcp-live-verify-landed.log — exit 0
  - criterion: Paired lifecycle/delegation contracts and broken controls
    reference: verification/contracts.toml:1336; /tmp/mcp-live-formal-landed.log — exit 0; evidence 0fc6016c69464134e6eb2e8ba983564ee208a4de52bee3b3123d0560ce8bb752
report:
  summary: Landed signed 24d03c76. Live grant/catalogue notifications, bounded 40 KiB delegation/reassignment, confirmed-orphan close metadata and protected-field amendment diagnostics are implemented. On landed HEAD the amended verify passed 761/761 tests plus 24 formal contracts; full native/TLC parity passed 562 models/34 crates and current-source evidence survived build cleanup. Four identical base failures are excluded per the amended brief. Full report committed as native checkpoint 87c54975. The report records land-fast's missing post-rebase gate refresh; I refreshed formal and assigned verification manually. The live host rejected the additional artifact claim as artifact_not_committed despite the committed report checkpoint.
  commit: 24d03c76b690a4e90b26c3fe81b7211fac051940
  reported_by: seat:01M6S0BP5BV4P07WSJSP2X6M27
  op_id: 01M6S72M114MQT44XCAN98ZZZJ
  verify_revision: 3
  report: '[[reports/2026-11-06-mcp-live-catalogue.md]]'
provenance:
  digest: sha256:c28f96b5ca628e97dbb9c29d1a9632111105eca7dde856d181bf148c1434b703
  frontmatter_digest: sha256:3dd46789fc84f1b14b6adc7ce913f4aef95bca9df882a100f07d5b1ea5a75568
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  sig: 5kp3rMjhJPrjA0vBHNG+kZTY7hsshW6fD3caHP14QnIVoH65mSh3CBxuil/H9JnUjvptbXc2EflPiu5CnKW2AA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/149799
  event:
    id: 01M6S73HY0Z22AM1HPMJ0RJWAM
    sequence: 149799
    hash: f296453cb1bb6b7ecba7c2f3f818213bfa90b949f3f32d6aceb3d98a75fdc1ad
---
# bboxy MCP: live catalogue, large-brief delegation, closing dead seats

Three defects that blocked the Oracle resuming the swarm after the 2026-11-06 crash (Oracle seat 01M6RZRGC3194VVFBC4M7XW4MF, live evidence).

## 1. The catalogue does not follow a live role change

The operator changed this seat's role executor → oracle live (grant epoch 2). `orient` lists 64 tools, including seat_spawn, task_reassign and workspace_prepare. The seat's existing `bboxy450-term mcp` bridge still serves the 45 launch-time tools, and the bridge declares `"tools":{"listChanged":false}` (`crates/bbt-mux/src/mcp/bridge.rs:284`). The harness cannot discover the new authority without a reconnect, which the operator cannot do. A fresh bridge process for the same seat lists all 64, which proves the authority is real and only the live connection is stale.

**Required:** the bridge declares `listChanged: true`. When the seat's grant epoch or role changes, it sends `notifications/tools/list_changed`, and `tools/list` is computed against the current grant. A tool removed by a downgrade is refused on call. Keep `plugin_tools.rs:32` behaviour consistent and update its comment.

## 2. task_reassign / task_delegate refuse briefs over 16 KiB

`crates/bbt-mux/src/mcp/work_items.rs:1446-1451` embeds the whole `doc.body` in the delegate wake (`"ROLE: … Execute this: {path}\n\n{body}"`), and `:1486` refuses any wake over 16384 bytes with `invalid or oversized Task wake`. All three live Dream briefs (22–31 KB) therefore cannot be reassigned, and their tasks still name dead seats.

**Required:** the wake carries the role line, the path and a bounded excerpt (or no body), plus an instruction to read the task through `task_status`/`knowledge_fetch`. Delegation and reassignment of a task of any size succeed. The 16 KiB wake bound stays as a hard limit on what is sent.

## 3. seat_close refuses dead seats

`seat_close` on a seat whose process died (`screen_read` → `target viewport unavailable`, adapter null, lifecycle still `open`) is rejected with `target viewport unavailable` (`crates/bbt-mux/src/logic.rs:5022`/`5175`). Dead seats cannot be closed, so they linger as `open` forever.

**Required:** closing a seat with no live viewport or process records the close (`closed_retained`, with the reason that the process was gone) without touching any live process. The generation check stays enforced. A live seat keeps its current close path.

## Acceptance

- [ ] Test: a live grant-epoch change makes the bridge emit `tools/list_changed`, and the next `tools/list` includes the newly granted tools. A downgrade removes them and calls are refused.
- [ ] Test: delegate and reassign a task whose body is 40 KB. Both succeed, and the wake is ≤16 KiB with the path.
- [ ] Test: seat_close on a seat with no viewport closes its record. Live-seat close is unchanged. A wrong generation is still refused.
- [ ] Formal: update the affected seat-lifecycle/delegation contracts in `verification/contracts.toml` (or a model), each with a broken control: closing a live seat as dead must fail, and a wake that drops the path must fail.

## Verify / landing

`cargo nextest run -p bboxy450-term-mux -E 'not (test(local_document_commit_keeps_unrelated_staging_and_refuses_no_change_and_symlinks) | test(held_private_git_hook_keeps_other_swarm_input_resize_and_status_live) | test(priority_burst_overflow_delivers_every_message_once_in_each_priority_fifo) | test(mux_document_writes_stamp_and_sign_digests_and_file_watch_observes_real_changes))' && make test-formal-contracts`, then land with `make land` (authorized once green, per AGENTS.md autonomous operation). Write the report `reports/2026-11-06-mcp-live-catalogue.md` and call `task_report` once. Do not reinstall the app.


## 4. Amend refusal does not say why

`crates/bbt-mux/src/mcp/knowledge.rs:413-435` refuses a task amend with the generic `task_field_protected` whenever the full frontmatter differs from the stored one in anything except verify/allowed_paths, including host-stamped fields (id, number, created, authorship, provenance). The delegating Oracle sent only the authored fields and got the misleading "only the delegating Oracle seat may amend".

**Required:** the refusal names the differing field(s), and host-stamped fields the caller omits are carried over from the stored record instead of counted as changes. Changing a protected field (status, assignee, delegated_by, role) is still refused. Test both.


**Lean landing (operator 2026-11-06: CPU and disk are scarce):** run only the targeted tests your diff affects plus the verify command; no full `make test`; land fast with `make land`. Clean your own large build outputs when done.

**Amend (Oracle):** the package is `bboxy450-term-mux` (my `-p bbt-mux` was wrong, Cargo exit 101); verify corrected. `crates/bbt-mux/src/models.rs` and `crates/bbt-mux/models/` granted for formal registration.

**Amend 2 (Oracle, verification disposition):** Ruling (Oracle): prove baseline before blame. In a detached worktree at base 85aa28c2 (git worktree add --detach, sharing your CARGO_TARGET_DIR so only mux recompiles), run ONLY the 7 failing tests once: storage_bus_index mux_document_writes_stamp…, harmony_host local_document_commit…, harmony_host held_private_git_hook…, mcp mcp_artifact_number_uses_unindexed…, mcp mcp_artifact_partial_write…, mcp task_migration_250…, messaging priority_burst_overflow…. Any that PASS at base are your regressions: fix them. Any that FAIL identically at base are pre-existing: your verify may exclude exactly those by name (nextest -E 'not (...)'), record both runs with exits in the report, and land. Remove the detached worktree afterwards. I will file the pre-existing ones as a bug task.

**Amend 3 (Oracle):** base 85aa28c2 reproduces exactly 4 failures (/tmp/mcp-live-baseline-seven.log: 3 pass / 4 fail, Oracle read the log); verify excludes exactly those 4 by name. They are tracked in [[tasks/2026-11-06-mux-preexisting-test-failures]]. The two artifact tests and task_migration passed at base, so they are this task's regressions and must pass.


## Report from seat:01M6S0BP5BV4P07WSJSP2X6M27

Landed signed 24d03c76. Live grant/catalogue notifications, bounded 40 KiB delegation/reassignment, confirmed-orphan close metadata and protected-field amendment diagnostics are implemented. On landed HEAD the amended verify passed 761/761 tests plus 24 formal contracts; full native/TLC parity passed 562 models/34 crates and current-source evidence survived build cleanup. Four identical base failures are excluded per the amended brief. Full report committed as native checkpoint 87c54975. The report records land-fast's missing post-rebase gate refresh; I refreshed formal and assigned verification manually. The live host rejected the additional artifact claim as artifact_not_committed despite the committed report checkpoint.

```json
{
  "op_id": "01M6S72M114MQT44XCAN98ZZZJ",
  "commit": "24d03c76b690a4e90b26c3fe81b7211fac051940",
  "report": "[[reports/2026-11-06-mcp-live-catalogue.md]]",
  "verify_result": {
    "command": "cargo nextest run -p bboxy450-term-mux -E 'not (test(local_document_commit_keeps_unrelated_staging_and_refuses_no_change_and_symlinks) | test(held_private_git_hook_keeps_other_swarm_input_resize_and_status_live) | test(priority_burst_overflow_delivers_every_message_once_in_each_priority_fifo) | test(mux_document_writes_stamp_and_sign_digests_and_file_watch_observes_real_changes))' && make test-formal-contracts",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Live grant changes notify and refresh tools; downgraded calls refuse",
        "reference": "crates/bbt-mux/tests/mcp/live_catalogue.rs:11; /tmp/mcp-live-verify-landed.log — exit 0"
      },
      {
        "criterion": "40 KiB delegate and reassign keep path and 16 KiB wake bound",
        "reference": "crates/bbt-mux/tests/mcp/live_catalogue.rs:87; /tmp/mcp-live-verify-landed.log — exit 0"
      },
      {
        "criterion": "Absent viewport close retains record; live owner and wrong generation refuse",
        "reference": "crates/bbt-mux/tests/mcp/live_catalogue.rs:128; /tmp/mcp-live-verify-landed.log — exit 0"
      },
      {
        "criterion": "Omitted host stamps survive; protected field differences are named",
        "reference": "crates/bbt-mux/tests/mcp/live_catalogue.rs:190; /tmp/mcp-live-verify-landed.log — exit 0"
      },
      {
        "criterion": "Paired lifecycle/delegation contracts and broken controls",
        "reference": "verification/contracts.toml:1336; /tmp/mcp-live-formal-landed.log — exit 0; evidence 0fc6016c69464134e6eb2e8ba983564ee208a4de52bee3b3123d0560ce8bb752"
      }
    ]
  }
}
```
