---
type: task
title: 'knowledge_write: let the delegating Oracle amend a claimed task''s brief/verify/allowed_paths'
role: executor
verify: cargo test -p bboxy450-term-mux --test mcp
id: 01M6H3CNDVPE7X28H6W0CM9ZEP
status: done
delegated_by: seat:01M6GVBAY1HW6P67GGDZM9XEBY
swarm: 01M6CQARWH5A79TE3H4NNWJ4XG
updated: 2026-11-03T16:22:29.892Z
created: 2026-11-03T16:20:32.722Z
number: 15233
created_by: seat:01M6GVBAY1HW6P67GGDZM9XEBY
updated_by: seat:01M6GVBAY1HW6P67GGDZM9XEBY
authorship:
  signer: seat:01M6GVBAY1HW6P67GGDZM9XEBY
  kind: seat
  public_key: UVOPOaWiKiIlf3/HRaRx9L4rRJKHPpaAXciElGQdQaE=
  process_generation: '1'
reconciliation:
- kind: historical
  fresh_accepted: false
  previous_status: open
  disposition: superseded
  reason: 'The fix already exists as accepted task #2189 on branch task-amend (9ac83912) but was never landed. Replaced by the landing task 2026-11-03-land-task-amend.'
  evidence:
  - .bboxy/tasks/2026-11-01-oracle-amends-live-task.md — status accepted, report.commit 9ac83912ec47cfaab103c6c1ab409948d7d771b1
  - git merge-base --is-ancestor task-amend origin/main — false (not merged)
  - git log origin/main..task-amend — 9ac83912 Allow delegating seats to amend live task briefs; d09dd25f Record live task amendment verification
  by: seat:01M6GVBAY1HW6P67GGDZM9XEBY
  op_id: 01M6H3G76KVSZX1N4J0SAWE8QQ
provenance:
  digest: sha256:f166a8815541bd71a32b4f04971e786f5e94c2ba0e5215742dce99604c79c27c
  frontmatter_digest: sha256:15527f811d3d9d414e3e8de8f5a0bb178fce0565da8bb1ea577037eaf3c6b225
  signer: seat:01M6GVBAY1HW6P67GGDZM9XEBY
  sig: A0aGmYefv+E+bzaBTi2SEC9oWByRMveGMX/bOpRzQ2rYVohrEGbcYa7WG6BtRh+17cvtDrsK9FITtP+uDUcXAA==
  log: .bboxy/log/2026-11-03.part-86213.jsonl#90905
---
# Delegating Oracle cannot amend a claimed task

ROLE: EXECUTOR. Worktree `~/mydev/bboxy450-term-task-amend`, branch `task-amend`, from `origin/main`.

## Bug (witnessed 2026-11-03, task #13806)

The delegating Oracle seat `01M6GVBAY1HW6P67GGDZM9XEBY` tried `knowledge_write` updates to its own `claimed` task twice. Both failed with `task_field_protected`. The tool description promises: "their delegating seat may amend only brief/verify/allowed_paths while nonterminal. Amendments wake the assignee in the task thread; changed verify requires a fresh report."

Cause: `crates/bbt-mux/src/mcp/knowledge.rs:337` requires `old.frontmatter["status"] == "open"`, and nothing checks that the caller is `delegated_by`. Also check `crates/bbt-mux/src/knowledge/effects.rs:150` (`tasks::authorize`) for the same rule.

## Outcome

- The seat named in `delegated_by` may change body/`verify`/`allowed_paths` on a task in any nonterminal status (open, claimed, reported, or whatever the lifecycle enum calls them). All other frontmatter must stay byte-equal. Other Oracle seats, workers and terminal tasks are still refused.
- A successful amendment of a claimed or reported task wakes the assignee in the task thread. A changed `verify` clears or invalidates any recorded report, so a fresh one is required. If either behaviour is missing, implement it with the existing mechanisms; do not write a second one.
- Callers can omit server-owned frontmatter (id, number, created*, updated*, authorship, provenance, status, assignee, swarm, delegated_by). The server carries them over; a caller that sends them unchanged still succeeds.
- `crates/bbt-mux/tests/mcp.rs`: add positive cases (delegator amends a claimed task; assignee woken; verify change invalidates the report). Keep the existing negatives near `:6267-6370`, and add negatives for a non-delegating Oracle and for a terminal task.
- If an existing TLA model covers the task lifecycle/authority (`verification/contracts.toml`), update it and its broken control; otherwise record in contracts.toml that the authority check is implementation-tested only.

Allowed paths: `crates/bbt-mux/**`, `verification/**`, `.bboxy/reports/**`.

## Rules

Work autonomously; one final `task_report` with a committed report under `.bboxy/reports/` and a `Report:` trailer. **The report's `verify.command` must be this task's `verify` string verbatim.** Strict clippy on touched crates, one `make test` on the final commit, signed commits, exact-path staging, land with `scripts/land.sh`.
