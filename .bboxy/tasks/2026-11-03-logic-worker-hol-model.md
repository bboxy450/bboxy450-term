---
type: task
title: 'Formal model: logic worker head-of-line blocking (provenance recover vs requests vs shutdown), with broken control'
role: executor
verify: make test-formal
id: 01M6HEH0XF19CP3RY4QW75VBX8
status: done
delegated_by: seat:01M6HEDKGMGNSJYV4KEYAH4C70
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-04T14:14:54.172Z
created: 2026-11-03T19:35:09.896Z
number: 16368
created_by: seat:01M6HEDKGMGNSJYV4KEYAH4C70
updated_by: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
authorship:
  signer: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
  kind: seat
  public_key: JlpXQZ4/Qy0yh5dcFR+iK7RgP5q83vzdNaLijybKm+Y=
  process_generation: '1'
reconciliation:
- kind: historical
  fresh_accepted: false
  previous_status: open
  disposition: implemented
  reason: The eight finite logic-worker recovery/audit models and named broken controls were integrated and verified; the old open model-only task is stale. This closes that delivered model assignment, not arbitrary Rust or current merge conformance.
  evidence:
  - Signed1c499ff2 integrated logic_worker_model.rs; its model file is byte-identical to signed model1aaa322e67ff9c7974e96ce50565db8e0476626f and present in published/installed ca7cde96.
  - Four correct/broken families LogicWorkerRequests, LogicWorkerShutdown, LogicWorkerAuditRequests, LogicWorkerAuditShutdown are registered with source mappings in ca7cde96 verification/contracts.toml/coverage/corpus.
  - Model report from1aaa322e records all eight complete native/Java graph and safety/liveness counterexamples; full normal gate40f45c58 later passed290 models/119 contracts/34 crates with retained receiptca67c9bedfd076917a63b87217ddb0633a4d4e0d844734d7c41cd7314795eb01. This is prior-source evidence, not a fresh ca7cde96 receipt.
  by: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
  op_id: 01M6KEK9HQP9TVN1SANW3Z9V3Y
provenance:
  digest: sha256:08eb17f25c67257ae84066ce176d02a9aec6cd5d8a1680254a7db6496362274a
  frontmatter_digest: sha256:d7b6d93a1c71c9872a4ec051e8155042cc2aeb155ddeb73c192edf74795f43c5
  signer: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
  sig: IB8HN1QacVoUBNFWId0P5RAO7wijLiVNWrLKvx4x5BBZLHlvPwJOvaNbDWkGhRvTujpuDI3ayIKB9vEyV4u9AA==
  log: .bboxy/log/2026-11-04.jsonl#114028
---
# Model the 2026-11-03 logic hang before any fix

ROLE: EXECUTOR. Worktree `~/mydev/bboxy450-term-hol-model`, branch `hol-model`, from `origin/main`. **Model only; do not touch product code.** A separate Executor writes the fix. Operator asked: can our TLA+/Yarn system catch this class at test time, before code runs?

## Incident (2026-11-03 ~11:14, term 1affb986 + harness 0c40316)

Logic stopped serving: MCP got "Seat control transport unavailable", GUI hello deadline elapsed, and shutdown timed out and was killed. The sample (`/private/tmp/claude-501/-Users-feral-mydev-bboxy450-term/47f8a74d-c00f-4cc9-a2c5-b87221bdf01b/scratchpad/logic-sample2.txt`) shows thread `bbt-logic` in `provenance::engine::Engine::step -> projection::step -> projection::recover -> Projection::append` for 3336 of 3412 samples. The main thread sits in `Logic::shutdown -> JoinHandle::join`.

Oracle source reading: `crates/bbt-mux/src/provenance/projection.rs:246-266` (`step`) appends one page, but falls back to `recover()` at `:250`, `:254` and `:266`. `recover()` (`:205-243`) does `verify_chain()` over the whole log, then `while state.cursor < head` over **all** history (440,727 events). The 8 ms budget (`provenance/engine.rs:37`) is only checked between steps, so one step is unbounded. Startup audit: `store/events.rs:444` `verify_history_after_ready` spawns `spawn_blocking` per events.sqlite.

## Outcome

A finite model in the style of `crates/bboxy450-term/tests/formal_contracts/provenance_models.rs` / `task_amend_model.rs`, registered in `verification/contracts.toml` and `corpus.toml` so that `make test-formal` runs it with native/Java TLC parity. Use `bboxy450_tlc::Model` directly, or Yarn `protocol!` if it expresses what you need (fairness/leads_to syntax for protocol! is not landed yet; do not depend on unlanded branches).

- State: worker phase (idle / serving request / in step with `work_left`), pending requests (bounded count with their wait age in worker ticks), a shutdown flag, a deadline counter, and history size H (a small finite instance, e.g. H ≤ 4 pages).
- Actions: request arrives; the worker serves one request; the worker runs a provenance step; shutdown requested; a tick (age and deadline advance).
- **Safety**: no pending request's age exceeds bound B while the worker is in a provenance step. **Liveness**: shutdown requested ~> worker stopped before deadline D. Fairness: weak fairness on worker actions only; state the assumption in the contract.
- **Correct variant**: a step does at most one page then yields to requests and shutdown (resumable cursor). **Broken control**: `recover` loops until `cursor == head` inside one step. It must be refuted on both safety and liveness for H > B, with the counterexample asserted. Also include the startup-audit variant (N parallel audits contending for the worker's store lock) if it changes a verdict; otherwise say why it doesn't.
- Record in `contracts.toml`: the source mapping (projection.rs recover/step, engine.rs budget, logic.rs shutdown/join), the abstraction (time = worker ticks; one page = one tick), what is unmodeled, and the **honest limit**: the model catches the design. It cannot see that `projection.rs:250` calls an unbounded `recover`; the implementation witness written by the fix Executor connects the two. Name that witness slot in the contract.

Allowed paths: `crates/bboxy450-term/tests/formal_contracts/**`, `verification/**`, `.bboxy/reports/**`.

`make test-formal` exit 0 on the final commit (needs main green; if main is still red, report your model's passing native/TLC verdicts and the exact unrelated blocker), then land with `scripts/land.sh`. Report with `verify.command` = `make test-formal` verbatim.
