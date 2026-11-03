---
type: task
title: 'Land the accepted task-amendment fix (branch task-amend, 9ac83912, task #2189) onto main'
role: executor
verify: make test
id: 01M6H3G4MQ8H5627F4Q9GCPD6Z
status: open
delegated_by: seat:01M6GVBAY1HW6P67GGDZM9XEBY
swarm: 01M6CQARWH5A79TE3H4NNWJ4XG
updated: 2026-11-03T16:22:26.494Z
created: 2026-11-03T16:22:26.494Z
number: 15241
created_by: seat:01M6GVBAY1HW6P67GGDZM9XEBY
updated_by: seat:01M6GVBAY1HW6P67GGDZM9XEBY
authorship:
  signer: seat:01M6GVBAY1HW6P67GGDZM9XEBY
  kind: seat
  public_key: UVOPOaWiKiIlf3/HRaRx9L4rRJKHPpaAXciElGQdQaE=
  process_generation: '1'
provenance:
  digest: sha256:ccedfed297f2859736ff6d3553f7c8162b721f079e1b971aac830658cb80dc98
  frontmatter_digest: sha256:6968c339ab81e0228e52e908bbff2fa2f6e708a31253edfa38dd6bbf313f522f
  signer: seat:01M6GVBAY1HW6P67GGDZM9XEBY
  sig: fCOD1pQGqx0nmC0YJS99thH9RbD3apGB0cSdjh9sYXfQpyZQuPEvLFZirQWVarYl8sP0NKpsQF2HQ93WNsp8AA==
  log: .bboxy/log/2026-11-03.part-86213.jsonl#90900
---
# Land the accepted task-amendment fix

ROLE: EXECUTOR. Use the existing worktree `~/mydev/bboxy450-term-task-amend`, branch `task-amend` (clean). Rebase it onto current `origin/main`.

## Why

Main still refuses a delegating Oracle's amendment of a `claimed` task: `crates/bbt-mux/src/mcp/knowledge.rs:337` requires `status == "open"`. This was witnessed again on 2026-11-03 against #13806. The fix already exists: task #2189 (`.bboxy/tasks/2026-11-01-oracle-amends-live-task.md`, status accepted, untracked in the shared checkout) was implemented in `9ac83912 Allow delegating seats to amend live task briefs`, with report commit `d09dd25f`. It was verified with make test exit 0 on 9ac83912 but **never landed**.

## Outcome

- Rebase `task-amend` (9ac83912, d09dd25f) onto `origin/main` and resolve conflicts, keeping both main's later changes and the fix's semantics: delegator-only amendment of nonterminal tasks, an assignee wake, a fresh report on a verify change, and refusals preserved.
- Add the canonical record `.bboxy/tasks/2026-11-01-oracle-amends-live-task.md` (copy byte-exact from `/Users/feral/mydev/bboxy450-term/.bboxy/tasks/`) in the report-bearing cut, so the records gate resolves `[[reports/2026-11-02-task-amend]]`'s task. This task's record is already on main.
- Write a new `.bboxy/reports/` entry for this task with the rebase/conflict account.
- `make test` green on the final commit, then land with `scripts/land.sh`.

Allowed paths: everything the two existing commits touch, plus `.bboxy/tasks/2026-11-01-oracle-amends-live-task.md` and `.bboxy/reports/**`. Change semantics only where needed to resolve a conflict, and name each such change in the report.

## Rules

Work autonomously; one final `task_report`. **The report's `verify.command` must be `make test` verbatim.** Signed commits, exact-path staging, land with `scripts/land.sh`.
