---
type: task
title: 'Oracle handoff to Codex: Yarn protocol program + logic hang (2026-11-03)'
role: oracle
verify: git -C /Users/feral/mydev/bboxy450-term fetch -q && git -C /Users/feral/mydev/bboxy450-term log --oneline -1 origin/main
id: 01M6HF0QHFXDCNPJZ3G0PQ37DE
status: done
delegated_by: seat:01M6HEW5SQKXBZ2ABF5TE4F0XQ
swarm: 01M6CQARWH5A79TE3H4NNWJ4XG
updated: 2026-11-04T02:34:21.964Z
created: 2026-11-03T19:43:44.483Z
number: 16390
created_by: seat:01M6HEW5SQKXBZ2ABF5TE4F0XQ
updated_by: seat:01M6HF0S1BAJKHWT2N426W7328
authorship:
  signer: seat:01M6HF0S1BAJKHWT2N426W7328
  kind: seat
  public_key: NJfxjXALG4vfTtn9XxMx7XCD2GZh2WlrBe8FFTqIaLM=
  process_generation: '1'
assignee: seat:01M6HF0S1BAJKHWT2N426W7328
reconciliation:
- kind: historical
  fresh_accepted: false
  previous_status: claimed
  disposition: superseded
  reason: 'Operator requests closing the tasks worked in this handoff. The completed incident integration and four finished Yarn branches are recorded by #16404 and #19660 and their landed evidence. This handoff umbrella is superseded by those completed records. Its unstarted cargo yarn CLI, typestate and Verus proposals remain future backlog in the original brief; this does not claim they were implemented.'
  evidence:
  - 'Signed incident implementation 40f45c58 and report 184b770f are on main; #16404 independently verified and accepted.'
  - 'Temporal a51f1f3d, requirements/verify 67dda793, Loom/Shuttle cdbb7d6f, Kani f01428b8 are on main; #19660 records the operator''s CI waiver and completed normal fast-forward landing.'
  - 'Prior component task amendment tasks #15233/#15241 and startup historical task #16564 are already done; #16394 ledger and #19660 integration are done; accepted #16417/#18977 retain their verified terminal records.'
  by: seat:01M6HF0S1BAJKHWT2N426W7328
  op_id: 01M6J6GJ5NXRWY7SC21WDZX2HG
provenance:
  digest: sha256:2df11472ca34bb1df22787e705f2e8702095906b73e4ffa7d5a2d0ef3007a3ea
  frontmatter_digest: sha256:6b29b6b0a005815228da44f0fe1e81b412792f0a4588768716b66976d8cb8c9c
  signer: seat:01M6HF0S1BAJKHWT2N426W7328
  sig: GfIlCqyH2QAw1fmvHJlExeoHSHFqBKRFGWUsai3fag5fwH+uDxzGv6cLMepWg+PYP9sjfyj3eTcvs6j9EOZRAw==
  log: .bboxy/log/2026-11-04.jsonl#111251
---
# Oracle handoff: Claude (Vesper Greywind) → Codex Oracle

ROLE: ORACLE. The operator moved the swarm Oracle to Codex: Claude's weekly window is at 98%, Codex's at 68% (usage_read 2026-11-03 11:43). You supervise: brief, verify and land. Implementation goes to Codex Executors. Landing is authorized ("autonomous landing authorized" 2026-11-02). Never trust a report: check the diff, the source lines, and a re-run of the task's verify command before accepting.

## Running right now (headless `codex exec`, not seats; started by the Claude Oracle while seat MCP was down)

| Task | Worktree / branch | PID | Codex session | Log / final message |
|---|---|---|---|---|
| #16366 main green (layering `dfb21967` cherry-pick, `HarnessTextZoom` coverage, startup-modal report record) | `~/mydev/bboxy450-term-main-green` / `main-green` | 90316 | 01a1a2e9-50ca-7a72-82ef-54ed4037775e | `S/main-green.log`, `S/main-green.last.md` |
| #16368 logic-worker head-of-line **model** (sliced passes; unbounded recover broken control refuted) | `~/mydev/bboxy450-term-hol-model` / `hol-model` | 90318 | 01a1a2e9-50ca-7f93-b05c-df1879d3a979 | `S/hol-model.*` |
| #16371 logic hang **fix** + process witness | `~/mydev/bboxy450-term-hol-fix` / `hol-fix` | 90320 | 01a1a2e9-50ca-7283-b4a3-94ad018113ce | `S/hol-fix.*` |

`S` = `/private/tmp/claude-501/-Users-feral-mydev-bboxy450-term/01a1a1b5-abd9-407f-91ae-52f5892bad96/scratchpad`. A process has finished when `ps -p <pid>` is empty; then read `<name>.last.md`. A process that died without a final message can be continued with `codex exec resume <session>` in its worktree. Full `make test`/`make test-formal` runs take the lock `/Users/feral/mydev/.bboxy-fulltest.lock` (mkdir/rmdir); a stale lock left by a dead PID (see its `pid` file) may be removed.

## Finished, waiting only for main green (do NOT respawn their executors; the operator ruled this)

All four were blocked solely by main's red gates (layering, then the startup-modal report record). Once #16366 lands, brief **one** Codex Executor to rebase each branch onto origin/main and land it with `scripts/land.sh`, one at a time. Then verify and accept each.

| Task | Worktree / branch | Signed HEAD | Gate evidence claimed (unverified by the Oracle) |
|---|---|---|---|
| #15234 protocol! temporal properties + effect lifecycle | `-yarn-temporal` / `yarn-temporal` | e4fae8f2 (impl 84f92fbb) | `make test-yarn-model && make test-formal` 0 pre-rebase; 272 models |
| #15235 requirements!/verify!, JSON results, contract baseline | `-yarn-verify` / `yarn-verify` | ba0cbd9e (impl 39740e7d) | same gate 0 pre-rebase; 264 models |
| #15236 Loom/Shuttle runtime | `-yarn-loom` / `yarn-loom` | 4bc9ba4f | `make test-yarn-concurrency` 0 (66 Loom schedules, 4 broken controls, 2048 Shuttle runs, 5.0 s) |
| #15237 Kani over generated reducers | `-yarn-kani` / `yarn-kani` | c0f722dc (impl 7bc2da36) | `make test-yarn-kani` 0, Kani 0.68.0 |

Overlap: temporal and verify both touch `crates/bbt-yarn-macros/src/protocol.rs`. Land temporal first, then rebase verify.

## Already done / obsolete

- #14637 `protocol!` landed as 0bb8e56b; verified by the Claude Oracle (21/105, broken 31/155, TLC parity, 264 models) and reconciled done.
- Task amendment fix: already on main as a8b12e2e. Tasks #15233/#15241 are obsolete; reconcile them superseded with that evidence. The branch `task-amend` holds only report records; it can be deleted after confirming it is not needed.
- `layering-harness-desktop` (dfb21967) is folded into #16366.

## Incident to report to the operator once #16368/#16371 land

Cause, confirmed in source: `crates/bbt-mux/src/provenance/projection.rs:246-266` `step()` falls back to `recover()` (`:205-243`). `recover()` verifies the whole chain and then loops over all history (`:219`) inside one step. The 8 ms budget (`provenance/engine.rs:37`) is only checked between steps, so the single `bbt-logic` worker is held for minutes. GUI hello deadline, MCP "transport unavailable", `Logic::shutdown` join timeout. Sample: 3336/3412 in `recover`. The operator wants: whether the model caught it (the model catches the design; the process witness connects it to the code), commit IDs, and the test evidence.

## Backlog after that (from the operator's "get all of this done")

From the original Yarn proposal, still unbriefed: the `cargo yarn` CLI (obligations / verify / replay / diff-contract / explain, behind Make); generated typestate handles (`Operation<Prepared>` style); optional Verus output for selected protocols. Brief each to a Codex Executor after temporal + verify land, since they build on them.

## Seat housekeeping

Seats with no work and no live viewport, where seat_close was refused with "target viewport unavailable": 01M6GVBAY1HW6P67GGDZM9XEBY, 01M6HEK2JH5YC18C45W1VP56JN, 01M6ERVT8NACX7S49CGY961QP5. Close them when the UI is attached. The Claude Oracle seat 01M6HEW5SQKXBZ2ABF5TE4F0XQ steps down after this handoff.
