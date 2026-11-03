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
id: 01M6HGAE1XXGRJ58SFATHX9R18
status: claimed
delegated_by: seat:01M6HF0S1BAJKHWT2N426W7328
swarm: 01M6CQARWH5A79TE3H4NNWJ4XG
updated: 2026-11-03T20:06:38.874Z
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
  digest: sha256:864a0a57a707e1b8f6b188db071a1978fd96082b99473d487b7a5142418e7e33
  frontmatter_digest: sha256:9cb13bb5836a2739c93bc3e275760ef4197ad884964bd53343b3f0a54cc3b8b1
  signer: seat:01M6HF0S1BAJKHWT2N426W7328
  sig: AXIb9R7LV9kEnk0yWKB3nxxQL/ANmA6ZlGQIXsChq6jALYVCSFF/zmH6WL883SR/9lTUwsSsiSCY1kkQyw87Cw==
  log: .bboxy/log/2026-11-03.part-86213.jsonl#95135
---
# Check the real document-write route and recovery trigger evidence

ROLE: EXECUTOR. Worktree `/Users/feral/mydev/bboxy450-term-hol-write-witness`, branch `hol-write-witness`, from signed 0d879fbb47991e2107b869113b39cb64bfabe365. Work only there. Read AGENTS.md, ~/.agents/AGENTS.md, ATTRIBUTION.md, [[tasks/2026-11-03-oracle-handoff-codex]], the existing recovery code and process witness. The original hol-fix Executor is still running in its own worktree; do not write there or recruit/duplicate it. Landing via scripts/land.sh is authorized after its fix and the main report repair land.

The original fix is independently witnessed: GUI/MCP ~26 ms, shutdown 147 ms and durable resume, on 0d879fbb. But Index::apply_author at incident index.rs:1035-1040 can call project_activity in a 64-iteration loop while handling a knowledge write. Test this route using the real admitted knowledge_write/MCP command, not only self_status. During a large in-flight recovery, issue a write and concurrent GUI hello/control query; assert bounded replies and shutdown under the actual existing constants. A timely explicit backlog/unknown outcome with retained write intent may be valid; do not report a write success if it did not commit. If the route already remains within its budget, keep its implementation and record evidence; if it fails, fix the smallest owner-blocking mechanism without weakening signatures, durable intents, admission or crash safety. Add the relevant removed-guard/process negative control. Keep filesystem calls/process children private and unattended. Bound/kill only exact children created by your test; no desktop/Keychain/operator input.

Exact 11:14:24 trigger was NOT logged by old code. Oracle evidence: abandoned `.bboxy/log/.projection-01M6HDB1BT547A3XKCM72EW0R0` born 2026-11-03T19:14:24.762990Z; canonical scoped index last successful task write at 19:14:21.039Z. Current projection stamps match the correct root after recovery; this cannot reconstruct the old branch. Add compact diagnostic evidence at recovery entry that distinguishes missing/invalid state, cursor/head/hash mismatch, file missing/mtime/length mismatch and AlreadyExists as appropriate; retain the specific part and expected/observed stamp where useful, no sensitive contents. Record once per recovery transition, not per page. Tests must assert distinct reasons on private corrupt/missing/partial-write fixtures. Do not invent the incident's original trigger or blame SQLite from fcntl: the sample has 3053/3412 samples in Projection::append -> File::sync_all -> fcntl.

Update the relevant implementation witness mapping in verification/contracts.toml without duplicating model contracts; rebase onto landed model/fix when available. Model-owned files are out of scope. The formal model abstracts page costs and cannot prove wall-clock latency or arbitrary Rust refinement: keep this limit explicit. Preserve attribution.

Run focused tests first, then canonical make test and needed formal/source evidence through scripts/land.sh on final source; no gate skips. Use two Cargo jobs/worktree-local target. Before a full make test/test-formal use the existing `/Users/feral/mydev/.bboxy-fulltest.lock` claim, release only your own; no such claim is needed for targeted checks. If another run owns it, continue the independent code/test work and review instead of repeated unchanged gate attempts.

Copy this task record into the branch unchanged for resolving report links. Write the full report with host-allocated number (knowledge_write) at .bboxy/reports/2026-11-03-logic-recovery-write-route-witness.md, copy it into your worktree, signed commit with Report trailer, real exits/source/evidence/gaps. task_report to this Oracle. Do not wait for operator input.
