---
acceptance:
- The recovery fixture keeps real durable ring/journal data and all crash/replay assertions.
- The arbitrary 10 GiB guard is replaced by an explicit bounded fixture budget with real filesystem growth detection and measured usage.
- Normal and 50-seat private recovery runs pass; a sparse oversized fixture is rejected without allocating the limit on disk.
- Signed clean changes, required formal evidence and canonical lean landing complete; original chat fix is included.
allowed_paths:
- crates/bbt-mux/tests/recovery.rs
- Makefile
- LAND.md
- .bboxy/tasks/2026-11-02-recovery-disk-budget.md
- .bboxy/reports/2026-11-02-recovery-disk-budget.md
- verification/coverage.toml
- .bboxy/reports/2026-11-02-recovery-disk-budget-integration.md
- scripts/quarantine.py
- scripts/land-fast.sh
- scripts/test-landing.py
- .bboxy/reports/2026-11-02-lean-ignored-suite.md
- .bboxy/reports/2026-11-02-recovery-final-status.md
assignee: seat:01M6EW8HN9QTQJ7A7GD1ZYMWYR
authorship:
  kind: seat
  process_generation: '2'
  public_key: 8G8gIL1MJqmoRkKikaIqqCqCFW1VHfzquBV0bXH7xs8=
  signer: seat:01M6EW8HN9QTQJ7A7GD1ZYMWYR
created: 2026-11-02T20:02:42.151Z
created_by: seat:01M6EW8HN9QTQJ7A7GD1ZYMWYR
delegated_by: seat:01M6EW8HN9QTQJ7A7GD1ZYMWYR
id: 01M6EXPQTBR9CT8MEETNEDG1ZQ
number: 5002219
provenance:
  digest: sha256:a2b6cba1c4bfe41d6198632ed060c90a9b1fed7d094e7de01fb7c335bac1d9de
  frontmatter_digest: sha256:0e0553ab1474504e08e58c9ba4ba992815dd84708a7b783db03d8ef3559a0416
  log: .bboxy/log/2026-11-02.jsonl#8377
  sig: TB4n9yvsL4ff/2yYKWBzIEfEgpAlApJlGmv2DuFWfY8sbtGzL6oJl16tqUYsQOEJ+A+nI6vzkR+3kOCowdv7Ag==
  signer: seat:01M6EW8HN9QTQJ7A7GD1ZYMWYR
role: solo
status: claimed
swarm: 01M64ADP8CDYN9VX5CR31SZ629
title: Bound recovery fixture disk use for lean landing
type: task
updated: 2026-11-02T21:07:45.147Z
updated_by: seat:01M6EW8HN9QTQJ7A7GD1ZYMWYR
verify: make test-recovery-disk
---
ROLE: SOLO

Direct operator brief: clean disk by removing all worktrees aged at least 14 hours (authorised process kills), then make lean landing smarter about the recovery test's disk demand and avoid unnecessary per-state logs. Work alone, ask no further cleanup approval questions.

Implementation worktree: /Users/feral/mydev/bboxy450-term-chat-mute, existing fix/chat-mute-notices branch contains the verified but not yet landed chat fix. Rebase it on current main, preserve that fix and existing report. Work within the named paths. Trace actual writes, keep crash-recovery persistence, bound/report the fixture's real bytes rather than demanding 10 GiB for every four-seat run. Add the focused Make workflow and appropriate documentation, run native budget/negative-control and recovery checks, mandatory formal verification and make land. Do not alter peers' in-progress landing scripts. Cleanup audit and salvaged local state are outside Git.

Integration amendment: rebasing onto the newly pinned harness introduced HarnessFoldedCard and BrokenHarnessFoldedCard without the parent crate coverage entries. Register that existing contract/control pair under bboxy450-harness in verification/coverage.toml, preserving the model and its implementation witnesses. This is necessary for the clean-source mandatory formal gate. Record the initial formal failure and corrected result in an append-only integration report, superseding the earlier report; do not weaken or skip the gate.

Lean-runner integration amendment: make land failed on the changed swarm_workdir target because every test in that target is deliberately ignored, although the changed native test passed via its Make command. Add an explicit lean-only opt-in for a verified nonempty all-ignored integration target. Use nextest's actual list metadata to distinguish ignored tests from empty suites or filters; fail closed for explicit filters, malformed metadata, build errors and real test failures. Add focused runner controls, preserve normal/full-gate strictness, and record the corrected canonical landing result. Do not use a blanket no-tests=pass override.

Final evidence amendment: append a current-source report at reports/2026-11-02-recovery-final-status.md recording the canonical landing outcome after a moving-main rebase and pinned-harness initialization. Preserve successful cleanup and native test evidence; identify any upstream mandatory-gate failure without bypassing or weakening it. Do not change the upstream harness or JSON feature-isolation contract in this task.