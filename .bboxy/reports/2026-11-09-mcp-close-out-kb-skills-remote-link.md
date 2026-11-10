---
type: report
title: MCP close-out, KB skills, and remote supervisor link cut 1
task: '[[tasks/2026-11-09-mcp-first-close-out-and-kb-skills]]'
author: seat:01M70FBP1RV5AWZ5AE3ZZWKNHB
created: 2026-11-09T19:31:43.312Z
commits:
- 69a2eb19b995f6e57f0c5aa8e75524fb3aa5efea
- f6dee9b38d3e98d8c2667bb3c3f68d53fea72971
verify:
  command: cargo nextest run -p bboxy450-term-mux --test it -E 'test(task_close) | test(skill) | test(workspace_audit) | test(mcp_task_accept_requires)'
  exit_code: 0
id: 01M70WQ0XH4N0G946M2VS2NZXX
updated: 2026-11-09T19:31:43.312Z
number: 93377961366489
created_by: seat:01M70FBP1RV5AWZ5AE3ZZWKNHB
updated_by: seat:01M70FBP1RV5AWZ5AE3ZZWKNHB
authorship:
  signer: seat:01M70FBP1RV5AWZ5AE3ZZWKNHB
  kind: seat
  public_key: SkH0jBJLMgNQ8eyjFw4BxnIDzq9wMmgZUeBUslwMcCw=
  process_generation: '1'
provenance:
  digest: sha256:17e6d28a05108234a09d5bc1c59a8cd6aafacba125f23198b78794b06b34bd64
  frontmatter_digest: sha256:a80a3b9e76931785b4aeb4868c865bff2c1ea0f41f8c8d3d243667596f7d4b66
  signer: seat:01M70FBP1RV5AWZ5AE3ZZWKNHB
  sig: 47ThMZvzC60vdc3BA5WyqERraF4CAzxi1Y+2QLNW7pdSB+kDZiJNnU1CqSqedE1bcmRwgiah7DAXT68yPaeCDA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151121
  event:
    id: 01M70WQ1PYZWFV8B8VQJFQBN8E
    sequence: 151121
    hash: de547d404eafc03d749ad921d00f540dbd3505199bdd56f3e18cc55e445951ff
---
# MCP close-out, KB skills, and remote supervisor link cut 1

Both commits are signed (G) and on `origin/main`: 69a2eb19b and f6dee9b38. They were pushed directly, without `make land`, on the operator's instruction to bypass the load-flaky gate. The rebase onto `origin/main` was a no-op, so the pushed source is exactly the tested source.

## Changes (69a2eb19b)

- **`crates/bbt-mux/src/mcp/work_items.rs`:**
  - `integrated()`: `task_accept` and `task_close` refuse a `report.commit` that is absent or not an ancestor of `origin/main`. This check is skipped (not refused) when the root has no Git or no `origin/main`, or when the value is not a hex SHA.
  - The new Oracle tool `task_close` (`Action::Close`) moves accepted → done and records `closed{by, op_id}`. Any other status is refused.
  - `live_status()`.
  - Task results now carry `assignee`.
- **`mcp/hints.rs`:**
  - `task_accept` → `task_close`;
  - `task_close` → `seat_close{seat_id}`, then `workspace_audit{task_path}`;
  - `workspace_audit` → one `workspace_release{workspace_id, expected_revision}` per release-ready worktree, at most 8.
  - The doc comment records the Oracle-only exception to the read-only rule.
- **`mcp/workspaces.rs` and `workspaces/git.rs`:**
  - New Oracle read `workspace_audit`, with findings `release_due`, `release_failed`, `missing_worktree`, `multiple_live_attempts` and `unmanaged_worktrees`.
  - `workspace_prepare` now accepts every live status. Before, delegated (`claimed`) tasks were refused, because the check named `assigned`/`active`. This was reproduced live today by this seat's own task.
- **`mcp/skills.rs`, `knowledge/{types,paths,roots}.rs`:**
  - New `skill` type at `skills/<name>/SKILL.md`.
  - `kb_skills()` is bounded to 128 skills of 256 KiB each. The name must equal the directory, and an invalid skill is reported in `provider_error` without failing the rest.
  - Skills have origin `bboxy450-term/kb`.
  - New `skill_search`, ranked by name, then description, then body, returning metadata only.
- **`.agents/skills`** is a symlink to `../.bboxy/skills`.
- **AGENTS.md:** the close-out procedure now uses the MCP steps.
- **`tests/formal_contracts/task_amend_model.rs`:** new `CloseAccepted` action (status 6 → 7 only).

## Changes (f6dee9b38)

New crate `bboxy450-term-remote`:
- `frame.rs`: the codec.
- `delivery.rs`: Yarn `RemoteDelivery` and `BrokenRemoteDelivery`.
- `link.rs`: `Outbox`, `Inbox` and `FileJournal`.
- `tests/delivery_witness.rs`.

Registered in Cargo, `verification/{corpus,coverage,contracts}.toml`, the bbt-yarn `verify` example and the licensing spec. The design is in the ICD `interfaces/remote-supervisor-link`.

## Commands and exits

- **Task verify:** `cargo nextest run -p bboxy450-term-mux --test it -E 'test(task_close) | test(skill) | test(workspace_audit) | test(mcp_task_accept_requires)'` → exit 0, 4/4 passed.
  - **Filter gap:** this filter does not select `acceptance_requires_origin_main_commit_and_audit_finds_unmanaged_worktrees`, because its name has no `workspace_audit`. That test passed in a wider run of 14/14.
- **Broken controls:** with `integrated()` stubbed to Ok and the accepted-only guard removed, both tests fail (2/2 FAIL).
- **Clippy:** `cargo clippy -p bboxy450-term-mux --all-targets -D warnings` → 0.
- **Formal contracts:** the `formal_contracts` run of `project_contracts_and_broken_controls` → 1/1 passed, which includes `TaskAmendment` and its controls.
- **Remote crate tests:** `cargo nextest run -p bboxy450-term-remote` → 4/4. Its clippy (`-D warnings`) → 0.
- **Native export:** the bboxy450-tlc and bboxy450-yarn `verify` examples gave RemoteDelivery pass with 52 states and BrokenRemoteDelivery safety with 221 states.
- **Pinned Java TLC** (21.0.13, `tla2tools.jar`): RemoteDelivery "No error", 52 distinct states, matching native. BrokenRemoteDelivery "Invariant Inv0 is violated".
- **Licenses:** `make test-licenses` → 0.

## Evidence per acceptance criterion

- **`task_accept` refuses off-main commits:** `work_items.rs` `integrated()`. Test `acceptance_requires_origin_main_...` expects "is not on origin/main".
- **`task_close` moves accepted → done, refuses other states, and hints seat_close then workspace_audit:** `Action::Close` and `hints.rs` `task_close`. The `mcp_task_accept_requires_...` assertions check the hint names `["seat_close","workspace_audit"]`, the seat_id, the second-close refusal and the worker refusal.
- **`workspace_audit` lists worktrees, records and task status:** `workspaces.rs` `workspace_audit`. The test sees a hand-added worktree in `unmanaged_worktrees` and refuses a worker.
- **The skill type is served with origin kb:** `kb_skills_are_listed_searched_and_read_with_kb_origin`.
- **`skill_search` returns URIs:** the same test (`make land` ranks first; a nonsense query returns empty; no content is returned).
- **`.agents/skills` symlink:** present in commit 69a2eb19b.
- **AGENTS.md close-out:** in commit 69a2eb19b.

## Not done or not checked

- `make land` and full `make test` were not run (operator bypass; Cargo.lock gained only the new local package). The full `make test-formal` pipeline was not run; only the targeted native and Java checks above.
- The installed app still runs the old binary (no reinstall without the operator's word), so `task_close`, `workspace_audit` and `skill_search` are not live until a rebuild.
- Not built yet: the launch-time skill index (design §4), `task_create`, the remaining CLI-only MCP tools, and remote cuts 2–5 (resume model, `serve` daemon, SSH provider, MCP relay, cross-supervisor routing).
- The UI task-status string lists are not unified yet.
