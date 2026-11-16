---
type: decision
title: Check per-root projection writer ownership across admitted document effects
status: decided
decided: 2026-11-03
id: 01M6HP1ATX0WN8A7YRXR1F8VPM
updated: 2026-11-03T21:46:24.445Z
created: 2026-11-03T21:46:24.445Z
number: 18864
created_by: seat:01M6HF0S1BAJKHWT2N426W7328
updated_by: seat:01M6HF0S1BAJKHWT2N426W7328
authorship:
  signer: seat:01M6HF0S1BAJKHWT2N426W7328
  kind: seat
  public_key: NJfxjXALG4vfTtn9XxMx7XCD2GZh2WlrBe8FFTqIaLM=
  process_generation: '1'
provenance:
  digest: sha256:9f493a6dc75532ffaf0fcd6cb6b2ca6eb4116e664776bb270970ac38729fc242
  frontmatter_digest: sha256:f974e0454042c5c4cb7c6696d409ec642231282bae637394ea4a149af58cb8cd
  signer: seat:01M6HF0S1BAJKHWT2N426W7328
  sig: eD870avRasrfFW59IdO8cxWKro0tK2Jy9bNvRNIJumLQ2S5uicv8XzaEQq7I/eEg5jTbNH2c13ZqicxSUvhqDA==
  log: .bboxy/log/2026-11-03.part-86213.jsonl#104183
---
# Per-root projection writer ownership

Decided by Codex Oracle under [[tasks/2026-11-03-logic-recovery-write-route-witness]] after the b469df27 boundary failure, 2026-11-03.

Evidence: make test-boundaries exited 2; ctl_task_transitions_preserve_verification_and_operation_identity returned unknown/write_not_confirmed, File exists (os error 17), on an ordinary task report. In logic/knowledge_effects.rs, effect_roots excludes another effect for the same root while apply runs on spawn_blocking. In logic.rs maintain_workspaces, that set does not exclude step_provenance; both routes reach Index::project_activity. The actor set is broader than the incident model's assumed single owner. This race existed in the incident source too, but retained evidence does not identify it as the actual 11:14 trigger.

Choice: at most one JSONL projection writer per canonical root across admitted document IO, background provenance, and any relevant scan/reconciliation route. Coordinate per root, with nonblocking defer/refusal and existing operation identity retained. Other roots, reads, message service and shutdown must continue; no global writer lock, queue or worker wait is introduced. Reuse existing root admission/lifecycle state when it suffices; do not build a parallel scheduler without evidence that it is needed.

Ownership must cover the real async IO lifetime. Success, refusal/error, panic, cancellation and stale/out-of-order completions must not leak a permanent busy root or release a newer owner's claim. An admitted effect with an unknown outcome must not be silently redispatched. An old completion cannot release a current owner after a root/generation changes, including A-to-B-to-A. Explicitly state any process-lifetime assumptions if the existing owner guards already prove part of this.

Verification: model two roots and the competing effect/provenance writers, and check exclusivity as an invariant rather than an assumption. State finite bounds, atomic actions and fairness needed for progress. A removed-exclusivity control must yield a native and pinned Java TLC counterexample. Pair it with a bounded deterministic private API/process witness that fails when the production guard is removed. Assert independent-root service while another root's effect is held, plus cleanup and original operation identity. The existing ctl boundary regression must pass unchanged. Keep original bounded-recovery HOL models, honest wall-clock/Rust-refinement limits and all final source-bound gates.

Rejected: retrying File exists blindly, swallowing an unknown write as success, weakening the ctl assertion, a global mutex/queue, or adding another unproved single-owner assumption. Code and contract changes remain in the amended Executor brief; this decision grants no authority outside it.
