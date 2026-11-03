---
type: task
title: Prove document writes remain responsive during recovery and record exact recovery triggers
role: executor
verify: make test
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
id: 01M6HGAE1XXGRJ58SFATHX9R18
status: claimed
delegated_by: seat:01M6HF0S1BAJKHWT2N426W7328
swarm: 01M6CQARWH5A79TE3H4NNWJ4XG
updated: 2026-11-03T21:29:13.305Z
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
provenance:
  digest: sha256:7548f5c0c78d6000e28ed1f89f07c29c6e4a08311d6896127fd58bcf19d20697
  frontmatter_digest: sha256:7577e89e564890aa23cb0a3c8d88a46f439c00d0fbe96b23f8c9057e9966ee97
  signer: seat:01M6HF0S1BAJKHWT2N426W7328
  sig: ji5Wom14FCjpLn2NEfDez8lYC6V15XN2IAqBZkX+9Y48h9Rm1k+vz/GBFWOTAB84t4ZhkRqv8UHggsTPKesJCA==
  log: .bboxy/log/2026-11-03.part-86213.jsonl#103504
---
# Check the real document-write route and recovery trigger evidence

ROLE: EXECUTOR. Worktree `/Users/feral/mydev/bboxy450-term-hol-write-witness`, branch `hol-write-witness`, from signed 0d879fbb47991e2107b869113b39cb64bfabe365. Work only there. Read AGENTS.md, ~/.agents/AGENTS.md, ATTRIBUTION.md, [[tasks/2026-11-03-oracle-handoff-codex]], the existing recovery code and process witness. The original hol-fix Executor is still running in its own worktree; do not write there or recruit/duplicate it. You are now the designated bug integration lander: landing via scripts/land.sh is authorized with the signed main repair included in your candidate; no prerequisite branch must land first.

The original fix is independently witnessed: GUI/MCP ~26 ms, shutdown 147 ms and durable resume, on 0d879fbb. But Index::apply_author at incident index.rs:1035-1040 can call project_activity in a 64-iteration loop while handling a knowledge write. Test this route using the real admitted knowledge_write/MCP command, not only self_status. During a large in-flight recovery, issue a write and concurrent GUI hello/control query; assert bounded replies and shutdown under the actual existing constants. A timely explicit backlog/unknown outcome with retained write intent may be valid; do not report a write success if it did not commit. If the route already remains within its budget, keep its implementation and record evidence; if it fails, fix the smallest owner-blocking mechanism without weakening signatures, durable intents, admission or crash safety. Add the relevant removed-guard/process negative control. Keep filesystem calls/process children private and unattended. Bound/kill only exact children created by your test; no desktop/Keychain/operator input.

Exact 11:14:24 trigger was NOT logged by old code. Oracle evidence: abandoned `.bboxy/log/.projection-01M6HDB1BT547A3XKCM72EW0R0` born 2026-11-03T19:14:24.762990Z; canonical scoped index last successful task write at 19:14:21.039Z. Current projection stamps match the correct root after recovery; this cannot reconstruct the old branch. Add compact diagnostic evidence at recovery entry that distinguishes missing/invalid state, cursor/head/hash mismatch, file missing/mtime/length mismatch and AlreadyExists as appropriate; retain the specific part and expected/observed stamp where useful, no sensitive contents. Record once per recovery transition, not per page. Tests must assert distinct reasons on private corrupt/missing/partial-write fixtures. Do not invent the incident's original trigger or blame SQLite from fcntl: the sample has 3053/3412 samples in Projection::append -> File::sync_all -> fcntl.

Update the relevant implementation witness mapping in verification/contracts.toml without duplicating model contracts; rebase onto landed model/fix when available. Model-owned files are out of scope. The formal model abstracts page costs and cannot prove wall-clock latency or arbitrary Rust refinement: keep this limit explicit. Preserve attribution.

Run focused tests first, then canonical make test and needed formal/source evidence through scripts/land.sh on final source; no gate skips. Use two Cargo jobs/worktree-local target. For #16404 the inherited global full-test lock is no longer required. Use a worktree-local target, at most two Cargo jobs and two test threads, retaining all canonical per-command resource/timeout gates. Leave every peer's running command and claim untouched.

Copy this task record into the branch unchanged for resolving report links. Write the full report with host-allocated number (knowledge_write) at .bboxy/reports/2026-11-03-logic-recovery-write-route-witness.md, copy it into your worktree, signed commit with Report trailer, real exits/source/evidence/gaps. task_report to this Oracle. Do not wait for operator input.


## Integration amendment (2026-11-03 20:24 UTC)

The inherited hol-fix process PID 90320 finished; no new headless process was started. Its final signed candidate is 1037f3ca71fb403d33f185a0a84d98792936a025, clean and not landed because upstream layering is red. Your base 0d879fbb already contains its bounded recovery implementation; compare actual diffs and retain any needed signed follow-up without duplicating the original code. Its report is available in the hol-fix worktree/commit, with focused checks and old-code failure evidence. The main-green headless process is also finished; #16394 is repairing/landing main including the source-only layering change and current upstream fixes.

You are the sole lander of the combined logic-worker model, bounded-recovery fix, your actual-write witness and trigger diagnostics. Consume the model's signed source implementation 954ce04ecee71f806cf19592d78ddbce975b22d9 (crates/bboxy450-term/tests/formal_contracts/logic_worker_model.rs, its models.rs registration, corpus/coverage/contract entries) into your own branch; the model Executor may still be completing checks, so read Git blobs rather than modifying its worktree. The added model paths are for integrating that signed implementation and necessary conflicts only. The Oracle independently ran its native counterexample test (exit 0) and all eight cases through canonical formal.check_model with pinned Java/TLC and reuse=False (exit 0, exact graphs/verdicts agree); this is scoped evidence, not the complete formal receipt. Full source evidence still belongs to your final integrated cut. See `research/2026-11-03-logic-hang-formal-gap.md` in the selected root for evidence and honest limits; it need not be copied into your commit.

Rebase onto fresh origin/main and consume any missing signed #16394 repair commits directly; do not wait for any prerequisite branch to land separately. Combine contract entries once, preserving your updated real witness slot and model assumptions. Copy the canonical task records needed by any reports into your branch unchanged. Avoid copying malformed/unneeded headless report records solely for bookkeeping; cite signed source commits and include the actual handoff evidence in your own host-numbered full integration report. Do not edit an existing report or fabricate original seat attribution. Verify signatures and inspect diffs of every source commit you consume.

Final canonical make test must pass on your exact committed integrated source and produce the clean/current-source complete formal receipt (make test includes make test-formal; do not repeat that full check when valid evidence already exists). Run any required process-boundary gate if your final changes alter the real boundary. Land via scripts/land.sh with valid gate reuse, then task_report your full report/commit/evidence to this Oracle. This supersedes waiting for an independent #16371 landing; keep all existing request/deadline/crash-safety/diagnostic acceptance criteria.


## Exact integration inputs

The inherited bounded-recovery changes in `crates/bbt-mux/src/logic.rs` and `crates/bbt-mux/src/store/events.rs` are explicitly in scope for consuming the signed original fix/rebase conflicts, not unrelated implementation. The research pointer above is deliberately plain text: it is read-only context in the selected KB, not a required commit-tree link or required record copy.

The complete model gate has now passed on clean source 2e69a2f1a7f4c858374613bafd25a7f61f2b0d73 (the rebased version of 954ce04e). Oracle `make check-formal-evidence` independently exited 0 at that exact source, 278 models/34 crates/115 contracts, evidence SHA256 368981ccecbd885954ef686d059ea6dab54016b7f66473a1b3ab235371d3973a. Prefer the current signed model implementation 2e69a2f1 when consuming it; the eight model bodies are unchanged. The inherited model Executor is adding its report and attempting the landing gate; read the actual outcome before assuming it landed. Reuse valid unchanged-model evidence via the existing gate, but refresh final integrated source/receipt as required.


## Remove the separate-landing dependency (2026-11-03 21:27 UTC)

Your signed clean 3c67e936 and completed focused evidence are acknowledged. The previous requirement to wait for #16394 to land is removed. It measured branch ordering rather than correctness and caused an unnecessary 30-minute wait. You are authorized to consume the signed report-ledger integration cut directly into your own branch, then rebase and complete final source-bound gates/landing. The existing #16394 full gate is still legitimately running; this amendment does not classify it as hung or cancel it.

Verified source cut: report-ledger HEAD 262ce5d7, built on 6afd6314, consists of ca07d897 (layering), 9b6b0ebd (hash-pinned report correction/ledger + required records), 7a6dc40e (Oracle-accepted normalized startup input fix, rebased 7eb6ead8), and 262ce5d7 (amended task copies). Oracle git verify-commit exited 0 for all four using approved Boxy signatures. Consume the missing signed commits or equivalent exact changes, preserve immutable correction-ledger blob bindings, and resolve only necessary integration conflicts. The newly allowed paths are ONLY for consuming these signed repairs and resolving their integration conflicts; do not implement unrelated work there. If a repair lands on main meanwhile, use that main version and omit duplicate patches. Do not cherry-pick a blocked headless report merely to acquire its code.

Do not wait for another branch's landing or for an arbitrary 30-minute timer. Continue focused work and assemble your final candidate now. Leave the peer's live command/claim untouched; #16404 may run its separately budgeted gate concurrently. Reusing another branch's evidence is permitted only through the existing source-bound gates, never by assertion. Run each missing/stale required gate once, retain a current-source clean complete formal receipt, and land through scripts/land.sh. If #16394 lands first, rebase onto it and consume only your remaining model/recovery/witness changes. Final verification and reporting remain make test exit 0 plus the required complete receipt and actual landing evidence; no exception or red gate is authorized.


## Resource decision: no global full-test queue (2026-11-03 21:29 UTC)

The legacy mkdir lock must not serialize all future work. Measured now: 10 logical cores, 24 GiB RAM, 31% CPU, 64% sampled memory use (Mach memory_pressure reports 57% free), 26 GiB filesystem capacity available. #16404 is authorized to run its final canonical gate concurrently with #16394, using at most CARGO_BUILD_JOBS=2, RUST_TEST_THREADS=2 and SWARM_TEST_THREADS=2, with a worktree-local target. This allocates at most four Cargo jobs across these two gate runs and reserves remaining CPU for the product/other work; each canonical gate retains its existing memory/time/disk controls. Do not change global Cargo settings, delete peers' targets, remove/claim the old lock, or stop a peer. If your own wrapper is merely waiting for that legacy lock, cancel only your recorded waiting wrapper before Make begins and launch the canonical command under this budget. No global lock approval or elapsed-time permission is needed. Resource failure still fails honestly; source/receipt/CI/landing gates remain mandatory.
