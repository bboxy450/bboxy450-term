---
type: task
title: Prove document writes remain responsive during recovery and record exact recovery triggers
role: executor
verify: sh scripts/land.sh --full
allowed_paths:
- crates/bbt-mux/src/index.rs
- crates/bbt-mux/src/provenance/projection.rs
- crates/bbt-mux/src/provenance/projection/recovery.rs
- crates/bbt-mux/src/logic/hol_tests.rs
- verification/contracts.toml
- .bboxy/reports/2026-11-03-logic-recovery-write-route-witness.md
- .bboxy/tasks/2026-11-03-logic-recovery-write-route-witness.md
- crates/bboxy450-term/tests/formal_contracts/**
- verification/corpus.toml
- verification/coverage.toml
- .bboxy/tasks/2026-11-03-logic-worker-hol-model.md
- .bboxy/tasks/2026-11-03-logic-worker-hol-fix.md
- .bboxy/tasks/2026-11-03-oracle-handoff-codex.md
- .bboxy/reports/2026-11-03-logic-worker-hol-fix.md
- .bboxy/reports/2026-11-03-logic-worker-hol-model.md
- crates/bbt-mux/src/logic.rs
- crates/bbt-mux/src/store/events.rs
- scripts/layering.sh
- .bboxy/invariants/layering.md
- .bboxy/specs/architecture.md
- .bboxy/decisions/d16393-hash-pinned-report-corrections.md
- .bboxy/reports/CORRECTIONS.toml
- .bboxy/reports/2026-11-03-startup-modal.md
- .bboxy/tasks/2026-11-03-startup-modal-operator-brief.md
- .bboxy/tasks/2026-11-03-report-correction-ledger.md
- .bboxy/tasks/2026-11-03-startup-modal-input-regressions.md
- .bboxy/reports/2026-11-03-startup-modal-input-regressions.md
- crates/bbt-doc/tests/corpus/**
- Makefile
- crates/bboxy450-term/src/main.rs
- crates/bbt-mux/src/logic/**
- crates/bbt-mux/src/knowledge/effects.rs
- crates/bbt-mux/src/knowledge/routing.rs
- crates/bbt-mux/src/knowledge/workspaces.rs
- crates/bbt-mux/src/provenance/engine.rs
- crates/bbt-mux/tests/logic.rs
- crates/bbt-mux/tests/mcp.rs
- crates/bbt-mux/tests/provenance.rs
- crates/bboxy450-term/tests/ctl.rs
- .bboxy/decisions/d18864-per-root-projection-writer-ownership.md
- .bboxy/reports/2026-11-03-logic-recovery-write-route-witness-*.md
- crates/bboxy450-term/src/startup_modal.rs
id: 01M6HGAE1XXGRJ58SFATHX9R18
status: claimed
delegated_by: seat:01M6HF0S1BAJKHWT2N426W7328
swarm: 01M6CQARWH5A79TE3H4NNWJ4XG
updated: 2026-11-03T22:34:10.705Z
created: 2026-11-03T20:06:30.913Z
number: 16404
created_by: seat:01M6HF0S1BAJKHWT2N426W7328
updated_by: seat:01M6HF0S1BAJKHWT2N426W7328
authorship:
  signer: seat:01M6HF0S1BAJKHWT2N426W7328
  kind: seat
  public_key: NJfxjXALG4vfTtn9XxMx7XCD2GZh2WlrBe8FFTqIaLM=
  process_generation: '1'
assignee: seat:01M6HGAF8KJMA0860GJ6VYM9E2
verify_revision: 1
provenance:
  digest: sha256:66c743559225e10dd705108490187df6bd8de52799d33f3eff273e746d4593d4
  frontmatter_digest: sha256:693abb837e51ecb178db34acd33404b2c4b20c48a71c2ddd923f7f355e3939de
  signer: seat:01M6HF0S1BAJKHWT2N426W7328
  sig: jbvknMvzZvMQAdf86/MZwf42074OrHWSXDea6f6K9Ik7jdPcwuAmpUSwzhsrBDIbb54j3LjGdHMQKrEXXpcFCg==
  log: .bboxy/log/2026-11-03.part-86213.jsonl#107392
---
# Integrated logic recovery, real write witnesses, and projection ownership

ROLE: EXECUTOR. Work only in /Users/feral/mydev/bboxy450-term-hol-write-witness, branch hol-write-witness. Continue from signed b469df27; preserve the existing integration. Read AGENTS.md, ~/.agents/AGENTS.md, ATTRIBUTION.md and [[decisions/d18864-per-root-projection-writer-ownership]]. This consolidated brief supersedes the earlier append-only amendment history. Final verify is now `sh scripts/land.sh --full`, whose initial canonical gate runs make test; `make test-boundaries` and a clean current-source complete formal receipt are also required. Landing through scripts/land.sh is authorized.

## Outcome and proof boundary

Finish and land one self-contained cut containing the bounded recovery fix, its eight head-of-line models, the actual knowledge-write/process witness, compact recovery-trigger diagnostics, and the per-root ownership correction below. The original 11:14 mechanism is synchronous full-history replay on the request owner, dominated in the sample by per-record File::sync_all. The exact historical entry trigger remains unproven; do not infer it from the new race.

Preserve bounded resumable verified recovery, durable cursor/crash-safety tests, timely GUI/MCP/shutdown service, restart/resume, and startup audits capped at two independent WAL readers. Keep trigger reasons for missing/invalid state, cursor/head/hash mismatch, file missing/mtime/length mismatch and append AlreadyExists, with once-per-transition evidence and no document contents. Preserve original operation IDs, unknown-outcome semantics and signed write-intent custody. Ordinary ready-fixture task reporting must succeed; do not turn its confirmation assertion into an allowed unknown/backlog result.

The incident model is hand-written bboxy450_tlc::Model, not a protocol! generated product reducer. Native/TLC parity verifies that finite declaration. It does not prove arbitrary Rust refinement or bound a stalled filesystem/OS scheduler. Keep source mappings, cost/fairness assumptions and implementation-only limits honest.

## Newly authorized ownership implementation

Canonical failure: /tmp/hol-integration-boundaries.log, make test-boundaries exit 2 on b469df27; ctl_task_transitions_preserve_verification_and_operation_identity returns unknown/write_not_confirmed, File exists (os error 17). Oracle source review confirms logic/knowledge_effects.rs excludes another admitted document effect via effect_roots, then runs writes on spawn_blocking, while logic.rs maintain_workspaces still steps that same root's provenance/project_activity. This violates the model's assumed single owner.

Implement the smallest correct per-canonical-root projection ownership rule across actual async document IO and background provenance, plus any relevant scan/reconciliation path. Reuse existing admission/lifetime state if sufficient; a nonblocking per-root lease is another option. The owner loop must defer/skip busy work, never wait on a blocking global or per-root mutex. Independent roots, messages, reads, GUI/MCP and shutdown continue. No global writer queue, silent retry of File exists, swallowed unknown result or success without confirmed write.

Ownership covers the real IO lifetime and is released correctly on success, preparation/apply refusal/error, panic, cancellation and stale/out-of-order completion. A still-running blocking job must not lose its ownership merely because its awaiter was cancelled. A stale completion cannot release a newer claim after root/generation changes (include A→B→A). If existing context/root admission checks already prove a lifecycle case, retain them, demonstrate it, and document the exact assumption instead of inventing another scheduler.

The new production ownership edits are authorized in logic.rs, logic/**, index.rs, knowledge/effects.rs, knowledge/routing.rs, knowledge/workspaces.rs, provenance/engine.rs and the already scoped projection/recovery files. Other integration-only source paths remain limited to the signed repairs below. Existing ctl assertions are unchanged; ctl.rs is in scope for a new deterministic witness or necessary private fixture setup, never weakening the reported failing assertion.

## Required model and real witnesses

Add/extend a finite model with two roots and competing admitted-effect/provenance writers. Name initial state, finite bounds, owner/lease phases and relevant generation identities, atomic actions, fairness and conditional progress assumptions. Check writer exclusivity as an invariant, not an assumption. Check independent-root availability while one root's IO is held. Preserve the eight existing HOL models/counterexamples.

Register the new pair(s) in corpus/contracts/coverage. A broken control that removes the actual exclusivity guard must yield native and pinned Java TLC counterexamples with asserted/replayed traces. New model files/registration and the original model files are now authorized for this ownership contract; unrelated models remain out of scope.

Add a bounded deterministic private API/process witness that forces the disputed interleaving, fails with the production ownership guard removed and passes restored. Do not rely on a lucky repeat of the flaky ctl test. While root A's document IO is held, prove no same-root projector overlaps, root B/read/GUI/MCP service still progresses, and completion/error/cancel cleans up without duplicate operations or stale-owner release. Use existing private identities/homes and scripted child control; no desktop, Keychain or operator input. Kill only exact recorded fixture children you created.

Then rerun the unchanged ctl_task_transitions_preserve_verification_and_operation_identity regression and canonical make test-boundaries. Do not bypass its gate. Final make test must also pass and include the complete native/Java formal receipt for the exact committed integrated source.

## Existing signed inputs and scope

Your branch already integrated the original fix (0d879fbb; later report-bearing candidate 1037f3ca) and exact rebased model 2e69a2f1. Oracle independently verified its 278-model/34-crate/115-contract clean formal receipt (SHA256 368981ccecbd885954ef686d059ea6dab54016b7f66473a1b3ab235371d3973a) and the earlier actual process witness. Those are supporting evidence, not the final ownership cut's receipt.

The signed report-ledger/input integration through 262ce5d7 is authorized for direct consumption: ca07d897 layering, 9b6b0ebd hash-pinned report correction and records, 7a6dc40e normalized startup input (accepted 7eb6ead8 rebased), 262ce5d7 task copies. Preserve correction-ledger exact before/after blob bindings. These document/test/Makefile/startup paths are for consuming signed repairs and necessary integration conflicts only. If fresh main already includes a repair, omit duplicates. Never wait for #16394 or another branch to land first.

Copy this task, the ownership decision, and any canonical task records required by your report links verbatim into the branch. Use plain-text private evidence paths when a record is read-only context and not part of the candidate; do not create dangling commit-tree wiki links. Never fabricate an old seat identity or edit an immutable report. Source headers/attribution remain attached.

## Gates, resources and handoff

Run focused ownership/model/process checks first; then each missing/stale canonical final gate once. Preserve a clean/current-source complete formal receipt. Reuse only through the existing source-bound verification/landing gates, never by an agent claim. No exceptions or new quarantine. Land via scripts/land.sh and report real final source/landing evidence.

No legacy global full-test lock is required for this task. Use a local target and at most CARGO_BUILD_JOBS=2, RUST_TEST_THREADS=2, SWARM_TEST_THREADS=2. Keep existing canonical resource/timeout controls. Leave all peer processes/claims/caches/targets untouched; this gate may run concurrently with #16394. Resource failure still fails honestly.

Allocate the full report number with knowledge_write/Index and copy it into this worktree at .bboxy/reports/2026-11-03-logic-recovery-write-route-witness.md (or an authorized appended correction if a report already exists). Include source lines, original failing log, deterministic broken control, commands with real exits, model counts/assumptions, unknown original trigger, unmodeled behavior and skipped checks. Signed commit with Report trailer, then task_report to this Oracle. No operator action is needed.


## Delivery/resource amendment: one final landing loop (2026-11-03 22:07 UTC)

The operator is low on disk and inference. #16394 is no longer attempting an independent landing or a third full baseline. It is handing you the verified ledger/input component, with two actual green full runs on 262ce5d7 and 633e9169 (2584 tests, 270-model full receipts). You remain the single integrator of the incident fix. Consume needed signed fixes and retained evidence; don't duplicate its runs merely for component bookkeeping. Keep your two-Cargo-job/two-test-thread limit, local target, all ordinary resource gates and the unchanged ownership/boundary acceptance criteria.

After the ownership fix and focused tests are ready, use the existing canonical `sh scripts/land.sh --full` directly for the final gate/landing, instead of a standalone make test followed by --gate-log on a moving base. scripts/landing.py wide_check(canonical=True) runs make test on the rebased candidate, then the same land() loop computes affected-set/tool/parity intersections for incremental rechecks when main moves. Its --full help explicitly retains incremental rechecks. Existing source/receipt/record/signature gates and the maximum-five-recheck refusal remain intact; do not alter landing.py, handcraft a filtered plan, force push, or bypass a refusal. This is the canonical path, not an exception. Changed-source evidence is never inherited by assertion.

verify is now exactly `sh scripts/land.sh --full`; a fresh report is required for it. Run the unchanged required make test-boundaries after the ownership correction. Avoid another broad standalone make test duplicating the lander's full gate. If source churn is disjoint, let the canonical loop choose the exact missing rechecks and refresh formal evidence; report the actual resulting scope. If the loop refuses or an affected path overlaps, report the exact reason before another unchanged baseline.

The full report must contain real outcomes. If the final landing's outcome cannot truthfully be included in its own source commit, first land the signed implementation through --full, then append the factual host-numbered full report in a signed records-only commit and use the existing checked records-only gate-log reuse path for that report. Do not predeclare an unexecuted exit 0, edit an immutable report or rerun Rust simply for report prose. Commit the report before task_report. The report must distinguish tested/landed implementation SHA, subsequent record SHA and any actual incremental rechecks.


## Mechanical upstream lint repair (2026-11-03)

Oracle verified the canonical --full failure on signed 8011ef8b: startup_modal.rs:107 defines cfg(test) mod tests before region() and impl App; workspace -D warnings therefore rejects items_after_test_module. You are authorized to edit ONLY this item's order in crates/bboxy450-term/src/startup_modal.rs: move the existing test module intact to the end (or place existing production items before it). Preserve every test assertion, production statement and origin notice. Do not add a lint allow, alter behavior/style/geometry, or weaken a test. No new test/model is needed for this mechanical ordering fix. If fresh main already has the equivalent signed repair, consume it instead of duplicating it.

Check the affected existing test and strict lint, then resume sh scripts/land.sh --full for the genuinely changed candidate, retaining valid canonical reuse/incremental checks. This is one newly evidenced failure, not authorization for an unchanged repeat or another standalone broad make test. All ownership/boundary/formal/landing requirements and resource limits remain.
