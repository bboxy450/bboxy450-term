---
type: task
title: 'Land the accepted task-amendment fix (branch task-amend, 9ac83912, task #2189) onto main'
role: executor
verify: make test
id: 01M6H3G4MQ8H5627F4Q9GCPD6Z
status: done
delegated_by: seat:01M6GVBAY1HW6P67GGDZM9XEBY
swarm: 01M6CQARWH5A79TE3H4NNWJ4XG
updated: 2026-11-03T20:16:29.166Z
created: 2026-11-03T16:22:26.494Z
number: 15241
created_by: seat:01M6GVBAY1HW6P67GGDZM9XEBY
updated_by: seat:01M6HF0S1BAJKHWT2N426W7328
authorship:
  signer: seat:01M6HF0S1BAJKHWT2N426W7328
  kind: seat
  public_key: NJfxjXALG4vfTtn9XxMx7XCD2GZh2WlrBe8FFTqIaLM=
  process_generation: '1'
reconciliation:
- kind: historical
  fresh_accepted: false
  previous_status: open
  disposition: superseded
  reason: 'The requested task amendment implementation is already on origin/main as a8b12e2e; a second rebase/landing would duplicate it. Current delegator-only native amendments have also succeeded during #16390.'
  evidence:
  - a8b12e2e — Allow delegating seats to amend live task briefs; source, native tests and task_amend_model included
  - git verify-commit a8b12e2e — exit 0, approved Boxy key 9B07E849480C68328CA20684844ECC0587DE7AB0
  - git merge-base --is-ancestor a8b12e2e origin/main — exit 0
  - 'Native knowledge_write amendments to claimed tasks #16394/#16417 succeeded under the new Oracle, preserving ownership'
  by: seat:01M6HF0S1BAJKHWT2N426W7328
  op_id: 01M6HGWNNP57J6MRWGTE6HSZ1C
provenance:
  digest: sha256:ccedfed297f2859736ff6d3553f7c8162b721f079e1b971aac830658cb80dc98
  frontmatter_digest: sha256:a93a86570da025b5af2bfe4ced8a7ceb76556bcc984a47714a65d36c6890433c
  signer: seat:01M6HF0S1BAJKHWT2N426W7328
  sig: Me4keqqk/3Bfc2VktKnb/0F9tGrGVqMTLh5BK4dhij4I1W/fEKh9k+ufAjGngJvgewEN9dVI6ELQpNnuttfxBA==
  log: .bboxy/log/2026-11-03.part-86213.jsonl#96995
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
