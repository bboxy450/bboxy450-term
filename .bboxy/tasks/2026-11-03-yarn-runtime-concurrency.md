---
type: task
title: 'Yarn runtime: Loom and Shuttle exploration of reactor/source synchronisation'
role: executor
verify: make test-yarn-concurrency
id: 01M6H3DQX6TJ27H2AX1E1KB35R
status: done
delegated_by: seat:01M6GVBAY1HW6P67GGDZM9XEBY
swarm: 01M6CQARWH5A79TE3H4NNWJ4XG
updated: 2026-11-04T02:31:48.640Z
created: 2026-11-03T16:21:07.843Z
number: 15236
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
  disposition: implemented
  reason: Operator explicitly requests closing the completed KB tasks as done. Implementation is on main. The final combined landing used the operator's explicit CI waiver; this is historical implementation disposition, not fresh acceptance or a claim of rerun current-source CI.
  evidence:
  - Signed integration cdbb7d6f, ancestor of remote main f01428b8 and its later descendants.
  - 'Task #19660 records the operator''s one-time CI/formal/local-hook waiver and the normal fast-forward push. Root independently checked all new signatures, whitespace and clean Git state. Original source branch/test histories retained; no new combined-source CI pass claimed.'
  by: seat:01M6HF0S1BAJKHWT2N426W7328
  op_id: 01M6J6BWKR6BKTVQQYCTG7FVRQ
provenance:
  digest: sha256:4454b30f30659d656f7fef3333983d234f7ef0082ef5b511114336bf923cc5fd
  frontmatter_digest: sha256:8687f406f3a0180fa05c1784f50653e06e51ee6c0c96579fbdfc28d69ff54584
  signer: seat:01M6HF0S1BAJKHWT2N426W7328
  sig: xT6z1W+Em+d1hsOJtF1H5UnDR03qsgETYFNWGBZ4ng+8CeVAkcn+Ra3HD6cq59JSTlScDX3wi2CRoeHj0oDcCA==
  log: .bboxy/log/2026-11-04.jsonl#110708
---
# Yarn runtime concurrency checks (Loom + Shuttle)

ROLE: EXECUTOR. Worktree `~/mydev/bboxy450-term-yarn-loom`, branch `yarn-loom`, from `origin/main`.

## Problem

The reactor model (`crates/bbt-yarn/src/model.rs`) assumes atomic arbitration and wake delivery. The real synchronisation in `crates/bbt-yarn/src/reactor.rs` and `crates/bbt-yarn/src/source/**` (wakers, readiness, shutdown/close, source replacement) has never had its interleavings explored.

## Outcome

- Read the runtime and list every shared-state synchronisation point (atomics, wakers, channels, locks) as `file:line — mechanism` in the report.
- **Loom** tests (`--cfg loom`, behind a `loom` cfg shim so normal builds are unchanged) for the small primitives: lost wake, close racing with ready, double-service, waker replacement. Each test gets a **broken control** that weakens one ordering or removes one check, behind a test-only cfg, and must fail.
- **Shuttle** randomized/PCT tests for larger reactor scenarios (several sources, shutdown priority, bounded drain) with fixed seeds and a recorded iteration count; failures print a replayable schedule.
- New Make target `test-yarn-concurrency` runs both; add it to `make test` only if its runtime stays under ~60 s on this machine (measure and report the time). Otherwise keep it separate and document it in `make help`.
- Pin `loom` and `shuttle` as exact-version dev-dependencies. Record coverage in `verification/coverage.toml`/`contracts.toml` as implementation-level evidence with explicit bounds (preemption bound, iterations). It is not a proof.
- If a real bug is found, fix it in the runtime, keep its test, and call it out first in the report.

Allowed paths: `crates/bbt-yarn/**` (runtime, tests, Cargo.toml), `Cargo.lock`, `Makefile` (new target and help line), `verification/**`, `.bboxy/reports/**`, `scripts/source-notices.py` (`OWNED_FORK_FILES` additions only). Do not touch `crates/bbt-yarn/src/protocol.rs` or `crates/bbt-yarn-macros/**`.

## Rules

Work autonomously; one final `task_report` with a committed report under `.bboxy/reports/` and a `Report:` trailer. **The report's `verify.command` must be this task's `verify` string verbatim.** Source headers per `ATTRIBUTION.md`, with "Inspired by" credit. Strict clippy, one `make test` on the final commit, signed commits, exact-path staging, land with `scripts/land.sh`. Other Yarn executors run concurrently; rebase through land.sh.
