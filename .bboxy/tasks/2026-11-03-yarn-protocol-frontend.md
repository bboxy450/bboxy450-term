---
assignee: seat:01M6GVDER4NMBZMRX9RF479SV9
authorship:
  kind: seat
  process_generation: '1'
  public_key: nhSn9xEBWtiSbyjsoGp+NiY6RbpHlYFKdrDAwuakGD8=
  signer: seat:01M878M634NG1V0HXT0SRPWWY5
created: 2026-11-03T14:00:59.442Z
created_by: seat:01M6GVBAY1HW6P67GGDZM9XEBY
delegated_by: seat:01M878M634NG1V0HXT0SRPWWY5
id: 01M6GVD3TZVFF49A423FX4VBYB
number: 13806
reconciliation:
- by: seat:01M878M634NG1V0HXT0SRPWWY5
  disposition: superseded
  evidence:
  - successor tasks/2026-11-03-yarn-protocol-frontend-v2.md (done) — commit cf7c96aa5, crates/bbt-yarn-macros/src/protocol.rs
  fresh_accepted: false
  kind: historical
  op_id: 01M878M95T6MXD71SVGV1CSZNY
  previous_status: rejected
  reason: Rejected in favour of v2, which is done.
rejection:
  by: seat:01M6GVBAY1HW6P67GGDZM9XEBY
  reason: 'Superseded by #14637: same brief, plus the scripts/source-notices.py OWNED_FORK_FILES exception. The amendment gate refused the delegating Oracle with task_field_protected. Not a rejection of the work; continue it in place under #14637.'
role: executor
status: done
swarm: 01M6CQARWH5A79TE3H4NNWJ4XG
title: 'Yarn protocol!: one declaration -> generated reducer + finite model + completeness errors'
type: task
updated: 2026-11-24T17:11:04.655Z
updated_by: seat:01M878M634NG1V0HXT0SRPWWY5
verify: cargo test -p bboxy450-yarn --all-features --test protocol && make test-formal
provenance:
  digest: sha256:2da421d6038570c1cc49ece6d7406b70d5766d9dd17cfd26b729d972f7249e4a
  frontmatter_digest: sha256:8ce8a144d93d973b0e055e98c694960af71c535ba9919f3ec3416909abe28fd4
  signer: seat:01M878M634NG1V0HXT0SRPWWY5
  sig: w97DATWWwtROydsc7SY4arjBXSF9leGp3ZFY+Yq6Gw24IWoF9eWwnUJBOfHO1KMFcNLpJ7W+mXaxQnA/rNqUBg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1901
  event:
    id: 01M878M9TZV8NXRE4D6D8GF385
    sequence: 1901
    hash: 863dbed51585d2c1987ea3bf929ca4d5dd78090ed27e500dd46ce2169f41a23f
---
# Yarn `protocol!` — first vertical chain

ROLE: EXECUTOR. Worktree `~/mydev/bboxy450-term-yarn-protocol`, branch `yarn-protocol`, from `origin/main`. Work autonomously; report once on completion (committed report under `.bboxy/reports/`, `Report:` trailer).

## Problem

`reactor!`/`reactor_model!` (`crates/bbt-yarn-macros/src/lib.rs:28-99`, `crates/bbt-yarn/src/model.rs`) model scheduling only; handler bodies are opaque (`model.rs:8`, boundary tests `crates/bbt-yarn/tests/ui-pass/constraint_erasure_boundaries.rs`). Application state transitions have no checked language.

## Outcome (one chain, nothing more)

A `protocol!` proc macro (new module in `bbt-yarn-macros`, re-exported from `bboxy450-yarn`) where **one declaration produces both**:

1. **Generated Rust**: `State` (private fields, `Default` = declared init), `Event`, `Effect` enums, and a pure `fn step(&State, Event) -> Decision<State, Effect>` where `Decision` is `Accepted { next, effects } | Rejected { reason } | Ignored { reason }` (reasons are generated enums). `reject`/`ignore` keep the state unchanged and emit no effects.
2. **Finite model**: a lowering to `bboxy450_tlc::Model` (behind the existing `model` feature) with declared invariants. Every right-hand side reads the pre-state; updates are simultaneous.

Supported language, v1 only: `bool`, finite `domain X { A, B, .. }`, `Option<Domain>`, `Set<Domain>` with `contains/with/without`, `==`/`!=`/`&&`/`||`/`!`, guards, `set`, `preserve`, `emit`, `reject`, `ignore`, `#[invariant(id = "...")]` over state. Anything else is a compile error with a stable message and a repair hint, in the `reactor!` diagnostic style. Calls to arbitrary functions are rejected.

## Completeness = compile errors (trybuild `ui-fail` cases, one per rule)

- an event with no `on` arm;
- a case that neither sets nor preserves some field;
- case guards of one event that overlap, or do not cover the event's domain (check by enumerating the finite parameter × state values the guard reads; say in the message which universe was checked);
- a field without an initial value;
- a duplicate invariant id.

## Acceptance

- `crates/bbt-yarn/tests/protocol.rs` declares the `LatestResult` example: domain `{A,B}`; fields desired/visible/submitted/pending; events Submit/Complete/Cancel; effect Fetch; invariant `RESULT-001` `visible.is_none() || visible == desired`. Tests assert:
  - native check: **21 reachable states**, RESULT-001 holds;
  - **broken control**: the same protocol without the `pending`/`desired == Some(id)` freshness guard on `Complete::current` (a second declaration in the test) is refuted with the counterexample `Submit(A), Submit(B), Complete(A)`;
  - **replay**: that counterexample, run through the generated `step`, reaches a state violating the invariant fn, and every model state reached in the correct model's exhaustive check equals the state the generated `step` produces along the same action path (compare all edges, not a sample).
- TLA+ export of the model goes through the existing Java TLC parity path in `make test-formal`, agreeing on states/edges/verdict.
- `verification/coverage.toml` and `verification/contracts.toml` gain the new source/model mapping and its broken control. Record what stays unmodeled: effect execution, async adapters, domains larger than declared.
- README section in `crates/bbt-yarn/README.md`: syntax, semantics, and which scope a result covers.

## Out of scope

Kani/Verus/Loom, `requirements!`/`verify!` macros, `cargo yarn` CLI, JSON reports, typestate/GADT handles, effect adapters. These are later stages; do not stub them.

## Rules

Allowed paths: `crates/bbt-yarn-macros/**`, `crates/bbt-yarn/**`, `crates/bboxy450-tlc/**` (only additive, if lowering needs it), `verification/**`, `.bboxy/reports/**`. Source headers per `ATTRIBUTION.md`; credit "Inspired by" Verus `state_machine!` and machine-check near the macro. Strict clippy `-D warnings --all-targets` on touched crates. One `make test` on the final commit. Signed commits. Land with `scripts/land.sh` when green.
