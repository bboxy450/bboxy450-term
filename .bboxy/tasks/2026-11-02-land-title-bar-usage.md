---
assignee: seat:01M6EK48WKEEPDMMY07B3R82SK
authorship:
  kind: seat
  process_generation: '1'
  public_key: uf32RsAmZxd9Ki2nPj3bXKgGOQxKZTkkaymTCBOc2CM=
  signer: seat:01M6CQB5DB5RA6QDF9N162NF8S
created: 2026-11-02T16:58:26.008Z
created_by: seat:01M6CQB5DB5RA6QDF9N162NF8S
delegated_by: seat:01M6CQB5DB5RA6QDF9N162NF8S
id: 01M6EK5APZPAYV60TH8Q4Q15DE
number: 3351
provenance:
  digest: sha256:33e6a6cba27a5723860eeb86d88c992b30b6fccab36493071e77e900e77c63aa
  frontmatter_digest: sha256:e4938f1e75034c4a19ccfea96f4003d6163e2209eaf6850ff25aaf0b475c2170
  log: .bboxy/log/2026-11-02.jsonl#15085
  sig: VYacmHUmiMMWWSHIUPSeBEc3tHYVnkN2fnLjGj9O1XqMhPp8IcU/BVUwyTLPt6/SxxhdBgCLAQef28u3gGNRDA==
  signer: seat:01M6CQB5DB5RA6QDF9N162NF8S
role: executor
status: claimed
swarm: 01M6CQARWH5A79TE3H4NNWJ4XG
title: Land the title-bar usage branch (#2197) from c5aa7df2
type: task
updated: 2026-11-02T17:00:55.872Z
updated_by: seat:01M6CQB5DB5RA6QDF9N162NF8S
verify: make test
---
# Land the title-bar usage branch

ROLE: EXECUTOR. Oracle Vesper Greywind, 2026-11-02. This continues #2197 (`.bboxy/tasks/2026-11-01-title-bar-machine-and-ai-usage.md`; read its Outcome, Scope and Verify for context). The first Executor stalled when its Codex auth expired.

Worktree: `/Users/feral/mydev/bboxy450-term-usage`, branch `title-bar-usage`, head `c5aa7df2`. You own that worktree alone. The implementation is done and committed; don't redo or extend it.

## Do
1. Rebase onto current `origin/main` (signed; resolve only conflicts in records or this branch's own files).
2. Under the CPU cap: start once at most 3 other `ci-local.sh` runs are going (`pgrep -f "sh scripts/ci-local.sh" | wc -l`), or after 45 min regardless. Then run `make test` once in this worktree's own target.
3. If it's green: `sh scripts/land.sh --gate-log target/ci-local.log`.
4. `task_report` this task with the landed main SHA, the gate log path and SHA256, and the land.sh exit code.

## Scope
Allowed: the rebase; importing byte-for-byte any existing `.bboxy` record (task, artifact, report) from `/Users/feral/mydev/bboxy450-term/.bboxy/` that the committed record graph needs, including this task file; a report under `.bboxy/reports/`.
Not allowed: code changes, more than one full run, publishing beyond land.sh, or installing the app.

## If it fails
One attempt. If a test this branch touches fails, stop and report it with the log line. If an untouched test fails, stop and report it with the log line; the Oracle decides. Don't retry or fix out of scope.

## Communication
Quiet. One final `task_report`.
