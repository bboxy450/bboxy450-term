---
type: task
title: Land the four completed Yarn branches sequentially with current-source evidence
role: executor
verify: make test-yarn-model && make test-yarn-concurrency && make test-yarn-kani && make test-formal
allowed_paths:
- crates/bbt-yarn/**
- crates/bbt-yarn-macros/**
- crates/bboxy450-tlc/**
- Cargo.lock
- Makefile
- scripts/source-notices.py
- scripts/check-protocol-contracts.py
- scripts/test-protocol-contracts.py
- scripts/test-yarn-kani.py
- scripts/test-yarn-kani-controls.py
- verification/**
- .bboxy/reports/**
- .bboxy/tasks/2026-11-03-yarn-four-branch-landing.md
- .bboxy/tasks/2026-11-03-yarn-protocol-temporal-effects.md
- .bboxy/tasks/2026-11-03-yarn-requirements-verify.md
- .bboxy/tasks/2026-11-03-yarn-runtime-concurrency.md
- .bboxy/tasks/2026-11-03-yarn-kani-generated-core.md
- .bboxy/decisions/d19663-yarn-temporal-unpublished-report-number-repair.md
id: 01M6J0SXWCR1WB1WQQ7H8PEBMD
status: claimed
delegated_by: seat:01M6HF0S1BAJKHWT2N426W7328
swarm: 01M6CQARWH5A79TE3H4NNWJ4XG
updated: 2026-11-04T01:26:04.030Z
created: 2026-11-04T00:54:35.964Z
number: 19660
created_by: seat:01M6HF0S1BAJKHWT2N426W7328
updated_by: seat:01M6HF0S1BAJKHWT2N426W7328
authorship:
  signer: seat:01M6HF0S1BAJKHWT2N426W7328
  kind: seat
  public_key: NJfxjXALG4vfTtn9XxMx7XCD2GZh2WlrBe8FFTqIaLM=
  process_generation: '1'
assignee: seat:01M6HGAF8KJMA0860GJ6VYM9E2
provenance:
  digest: sha256:81c18bf2628aae9cda7fda64ecb4cc8920729e44bd1987b7020bd891681f4175
  frontmatter_digest: sha256:d47952f0337a5137e697682db3056c9886cfcff58d8717fb303d5e9ac1fe6492
  signer: seat:01M6HF0S1BAJKHWT2N426W7328
  sig: lXHfP7Xv4LNY7ISX/FUqClswN1Uo4ddP3xhWEDEsYuHnRcYx7iciofHQqwBwwctGCGJwPEGQgNEM19bavigrAQ==
  log: .bboxy/log/2026-11-04.jsonl#109887
---
# Land the four completed Yarn branches

ROLE: EXECUTOR. Follow-on to [[tasks/2026-11-03-oracle-handoff-codex]]. The incident integration is accepted and landed at 40f45c58 with signed report 184b770f. You are the single integration lander. Do not spawn or resume the four original Executors.

Work in these existing clean worktrees, one writing worktree at a time, in this order:
1. /Users/feral/mydev/bboxy450-term-yarn-temporal, branch yarn-temporal, signed handoff e4fae8f2. Source brief [[tasks/2026-11-03-yarn-protocol-temporal-effects]].
2. /Users/feral/mydev/bboxy450-term-yarn-verify, branch yarn-verify, signed handoff ba0cbd9e. Source brief [[tasks/2026-11-03-yarn-requirements-verify]].
3. /Users/feral/mydev/bboxy450-term-yarn-loom, branch yarn-loom, signed handoff 4bc9ba4f. Source brief [[tasks/2026-11-03-yarn-runtime-concurrency]].
4. /Users/feral/mydev/bboxy450-term-yarn-kani, branch yarn-kani, signed handoff c0f722dc. Source brief [[tasks/2026-11-03-yarn-kani-generated-core]].

## Outcome and authority

Inspect the actual source, reports, clean Git state and retained logs before changing each branch. Rebase onto fresh origin/main and land each sequentially using sh scripts/land.sh and its existing signed, source-bound checks. Landing and pushes through that script are explicitly authorized. Preserve the original branches' implementations, negative controls and honest proof limits; resolve mechanical overlapping metadata/Make/formal-inventory changes by retaining both additions. Temporal precedes verify because both change protocol.rs. Do not reimplement finished work.

Use valid author evidence where the canonical script permits it; run a missing/stale required check once. Use its existing current-source full gate when no valid reuse path exists, rather than running a standalone full suite and repeating it for landing. No gate changes, skipped controls, historical green claims for rebased source, unsigned commits, force pushes, broad staging or unrelated repairs. A substantive new semantic conflict/gate failure outside these scopes gets one concise blocker with source/log evidence and the exact ruling needed; continue independent in-scope work.

Read the four original briefs and satisfy their acceptance lines. Required focused targets on the final combined source: make test-yarn-model, make test-yarn-concurrency and make test-yarn-kani; make test-formal must produce complete native/TLC parity and a clean current-source retained receipt after rebase. Keep all prior incident models/ownership/JSONL recovery controls and formal coverage. Kani must use the pinned repository-local installation and fail honestly on unsupported behavior. Do not infer arbitrary Rust or scheduler/wall-clock correctness from finite exploration.

Copy this task and the four cited task records verbatim from the authenticated knowledge tools where needed for record links. Existing published reports remain immutable. The specific unpublished temporal metadata migration below is authorized. Write one new final report .bboxy/reports/2026-11-03-yarn-four-branch-landing.md with exact landed SHAs, source/log/receipt hashes, actual per-command exits, negative-control and acceptance evidence, remaining gaps and final clean state. Commit signed with Report trailer. Land that records-only report through the existing checked reuse path; do not start a fifth full baseline solely for the report. Then task_report with this task's verify string verbatim. The Oracle will inspect each landed change and verification before reconciliation of the four historical tasks.

## Budget and boundaries

One Executor, two Cargo jobs; export BBOXY_GATE_JOBS=2 RUST_TEST_THREADS=2 SWARM_TEST_THREADS=2 for gate runs. Use worktree-local targets and existing shared cache via Make/scripts/cache-env.sh. No inherited global full-test lock, new shared queue, new cohort or CLI/typestate/Verus work in this assignment. Current machine has about 33 GiB available; Claude is 99% used and Codex 77%. Preserve peer targets and archived formal evidence. If disk becomes insufficient, stop the allocating command and report exact measurements; do not delete another worktree or shared cache. Do not poll peers, restart the live app, install a product build or contact the operator.

## Checks

The final verification command is:
make test-yarn-model && make test-yarn-concurrency && make test-yarn-kani && make test-formal

The normal canonical landing gates remain mandatory for each landed branch. Record final focused results honestly; report-only source changes may reuse only the existing checked records path.


## Signed ruling: unpublished temporal report numbers

Follow [[decisions/d19663-yarn-temporal-unpublished-report-number-repair]] (#19663). This is the specified alternative to modifying immutable reports through native knowledge_write, whose host catalogue has no such legacy-report record and grants no renumber reservation API.

The current failed gate must finish before its source changes. Preserve original signed e4fae8f2 and rebased/tested dc58a7c3c1163c0dbe9c7dd215f39f7b9524ab24 under named backup refs. You are authorized to create one new unpublished temporal integration branch in the same temporal worktree from fresh origin/main and carry the reviewed temporal delta as one candidate commit. Do not rewrite published history, force-push, delete original history, or touch another writing worktree concurrently.

Only these three legacy report number lines may change:
- .bboxy/reports/2026-11-03-yarn-protocol-temporal-effects.md: 5002229 -> 93377961359200; exact blob 1625b59344b353594407924a8d7cb80b108c3255 -> f9e102f9b103226f9bc1756d625a5ebbdb1586a7.
- .bboxy/reports/2026-11-03-yarn-protocol-temporal-final-blocked.md: 5002230 -> 93377961359201; exact blob 1d42639bf5224f88f68c63a0b416c3e335512882 -> d30f6e122366e609912afe1021564ff4a9688956.
- .bboxy/reports/2026-11-03-yarn-protocol-temporal-landing-blocked.md: 5002228 -> 93377961359202; exact blob 1907d606ecc2f8c13430a80642f33620d4fb8554 -> dd167c47c8e94cca8ebb391810d097833bb5332b.

Preserve every other byte in those reports and verify this by executable assertions plus the exact blob pins. Confirm these literal reserved numbers are unique before commit. Preserve the reviewed temporal source delta; compatible rebase onto newer main remains authorized. Copy the authenticated new decision and amended task into the candidate with the four already-required component tasks.

This new candidate must add all three reports and resolve their links within its own tree. Their earlier invalid unpublished prefixes must not be ancestors in the published candidate. The three reports are A relative to main, so do NOT append CORRECTIONS.toml entries for them: unused entries are errors. Existing ledger bytes and published reports remain unchanged. A tip-only M/ledger repair would leave earlier candidate commits invalid because reports.rs validates every report-changing commit's own tree.

Run focused corpus and committed-report checks over the complete candidate range before the next changed-source canonical landing gate. Use the existing scripts/Make commands that run that corpus (discover their exact commands); do not invent a missing Make target. Once green, land through the canonical gates as originally authorized. Preserve the failed dc58a7c3 results as failures; no identical retry, metadata waiver or gate weakening.

This exception is limited to these three exact reports and numbers. Continue verify, Loom, Kani in the original order only after temporal lands. The task verify string and all original acceptance criteria remain unchanged.
