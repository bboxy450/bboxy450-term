---
type: task
title: 'Yarn protocol!: one declaration -> generated reducer + finite model + completeness errors (supersedes #13806)'
role: executor
verify: cargo test -p bboxy450-yarn --all-features --test protocol && make test-formal
id: 01M6GY2WHWAT8NK2X67M4MA9M8
status: done
delegated_by: seat:01M6GVBAY1HW6P67GGDZM9XEBY
swarm: 01M6CQARWH5A79TE3H4NNWJ4XG
updated: 2026-11-03T16:08:57.896Z
created: 2026-11-03T14:47:49.350Z
number: 14637
created_by: seat:01M6GVBAY1HW6P67GGDZM9XEBY
updated_by: seat:01M6GVBAY1HW6P67GGDZM9XEBY
authorship:
  signer: seat:01M6GVBAY1HW6P67GGDZM9XEBY
  kind: seat
  public_key: UVOPOaWiKiIlf3/HRaRx9L4rRJKHPpaAXciElGQdQaE=
  process_generation: '1'
assignee: seat:01M6GVDER4NMBZMRX9RF479SV9
artifacts:
- '[[01M6H1QBXPQ8D7PDWPVC8JBW33]]'
- '[[01M6H1QCXH1Y826858B1ZBN4S0]]'
verify_result:
  command: make test; sh scripts/land.sh --gate-log target/ci-local.log; make check-formal-evidence
  exit_code: 0
  evidence:
  - criterion: Generated pure Rust, finite types and completeness diagnostics
    reference: crates/bbt-yarn-macros/src/protocol.rs:853,1338,1514; crates/bbt-yarn/src/protocol.rs:14; make test-yarn-model exit 0; 14 protocol ui-fail cases and protocol_hygiene ui-pass
  - criterion: 21 reachable states, RESULT-001 and exhaustive reducer replay
    reference: crates/bbt-yarn/tests/protocol.rs:112 — all 21 states and 105 edges; target/ci-local.log — LatestResult pass, 21 states, 105 edges match
  - criterion: Broken freshness control and exact counterexample replay
    reference: crates/bbt-yarn/tests/protocol.rs:179 — Submit(A), Submit(B), Complete(A), desired=B/visible=A, invariant false; target/ci-local.log — BrokenLatestResult safety, 31 states, 155 edges match
  - criterion: Native/Java TLC graph/verdict parity on current clean source
    reference: target/ci-local.log — 264 models, 34 crates, 108 contracts; source 0bb8e56b2a13b9b7c6104a1bb89be26070cc794c, dirty=false; receipt sha256 3a684ad96335ac586666b51ec3c5dda97cb4acbea8c0d92ad4b632f9a1463de2; target/protocol-formal-receipt.log — make check-formal-evidence exit 0
  - criterion: Source/model mapping, broken control, syntax and scope documentation
    reference: verification/coverage.toml:131; verification/contracts.toml:747; verification/corpus.toml; crates/bbt-yarn/README.md — Pure finite protocols; effect execution/async adapters/larger domains explicitly unmodeled
  - criterion: Signed final commit, canonical full gate and landing
    reference: 'target/ci-local.log — ci-local: result 0bb8e56b2a13b9b7c6104a1bb89be26070cc794c normal 0; target/protocol-land.log — LANDED 0bb8e56b2a13b9b7c6104a1bb89be26070cc794c and exact-candidate full gate reuse; git log G with 9B07E849480C68328CA20684844ECC0587DE7AB0; origin/main equals 0bb8e56b2a13b9b7c6104a1bb89be26070cc794c; git status --short empty'
report:
  summary: 'Landed signed commit 0bb8e56b2a13b9b7c6104a1bb89be26070cc794c on origin/main. protocol! generates private finite state, typed events/effects/reasons, a pure simultaneous reducer and the optional symbolic model, with stable completeness/type diagnostics. LatestResult passes at 21 states/105 edges; the broken declaration fails at 31/155 and replays Submit(A), Submit(B), Complete(A). All reachable reducer edges are compared; ProtocolLanguage adds 48 states/2352 edges. Final make test exited 0 (2566 tests passed, 211 skipped), strict Clippy and license checks passed, and native/Java TLC matched all 264 models across 34 crates/108 contracts with clean source-bound evidence. The first full run''s two missing-task record failures were resolved by inheriting Oracle commit cf7c96aa and rebasing; the complete final gate then passed. Landing reused that exact-commit full log and exited 0; make check-formal-evidence exited 0; worktree is clean. Effect execution, async adapters and larger domains remain unmodeled. Full report: [[reports/2026-11-03-yarn-protocol-frontend]].'
  commit: 0bb8e56b2a13b9b7c6104a1bb89be26070cc794c
  reported_by: seat:01M6GVDER4NMBZMRX9RF479SV9
  op_id: 01M6H1SR1V5VBPJQZSC4CE1HJM
  verify_revision: 0
  report: '[[reports/2026-11-03-yarn-protocol-frontend]]'
reconciliation:
- kind: historical
  fresh_accepted: false
  previous_status: reported
  disposition: implemented
  reason: Landed as 0bb8e56b. The executor's report (#14652) records the verify command `make test-yarn-model && make test-licenses` instead of the task's, so task_accept refused. The Oracle ran the task's exact verify command on 0bb8e56b in a fresh worktree; both halves exited 0.
  evidence:
  - git log origin/main — 0bb8e56b2a13b9b7c6104a1bb89be26070cc794c, signature G (Boxy), descendant of cf7c96aa
  - 'git show --stat 0bb8e56b — 43 files: crates/bbt-yarn*, verification/*, .bboxy/reports, scripts/source-notices.py; all within the allowed paths'
  - git show 0bb8e56b -- scripts/source-notices.py — only adds 5 protocol files to OWNED_FORK_FILES; classification logic unchanged
  - cargo test -p bboxy450-yarn --all-features --test protocol @0bb8e56b — 3 passed, exit 0
  - make test-formal @0bb8e56b — exit 0; LatestResult pass 21 states/105 edges match; BrokenLatestResult safety 31/155 match; ProtocolLanguage pass 48/2352 match; 264 models, 34 crates, receipt evidence_sha256 3a684ad96335ac586666b51ec3c5dda97cb4acbea8c0d92ad4b632f9a1463de2, dirty false
  - make test-licenses @0bb8e56b — exit 0
  - crates/bbt-yarn/tests/protocol.rs:116-117 — asserts 21 states, 105 edges; :175 checked_edges == 105 (every edge replayed through generated step); :203-213 broken counterexample == [Submit(A), Submit(B), Complete(A)], visible A, desired B, invariant false
  - 'trybuild ui-fail: missing_event, missing_update, missing_init, overlap, uncovered, duplicate_invariant, arbitrary_call and others — ok in the test-formal log'
  by: seat:01M6GVBAY1HW6P67GGDZM9XEBY
  op_id: 01M6H2QD3YZFQVWRZPPBPPDWXX
provenance:
  digest: sha256:4f3a93fdf50f58bdf2aaea7c6f1bce97437e3285385d452ddc034852de69bf5a
  frontmatter_digest: sha256:a68ccd7f88e999c6e8c0f1f2104c2a1164b9181c47c99471c779bebd6a14cce0
  signer: seat:01M6GVBAY1HW6P67GGDZM9XEBY
  sig: PK1RZnhPLOJ9K/bq3XGHLHaNEWB5crRVgea4krGC0iC/4phSEbCrGmB7g2nUPm9zyp+5BGsj1gE3h2WYnoiJBg==
  log: .bboxy/log/2026-11-03.part-86213.jsonl#89424
---
# Yarn `protocol!` — first vertical chain

Supersedes #13806 (same brief, plus the `scripts/source-notices.py` exception in Rules; the amendment gate refused the delegating Oracle twice with `task_field_protected`). Continue the existing work in place; nothing restarts.

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

Allowed paths: `crates/bbt-yarn-macros/**`, `crates/bbt-yarn/**`, `crates/bboxy450-tlc/**` (only additive, if lowering needs it), `verification/**`, `.bboxy/reports/**`, and `scripts/source-notices.py` **only** to add the new independently written protocol files to `OWNED_FORK_FILES` (Oracle ruling 2026-11-03: project-owned like `model.rs`, not kittens-old forks; `source-notices.py:62-65` classifies unlisted `crates/bbt-yarn*` files as forks; do not change classification logic, fork headers on inherited files, or upstream notices). Source headers per `ATTRIBUTION.md`; credit "Inspired by" Verus `state_machine!` and machine-check near the macro. Strict clippy `-D warnings --all-targets` on touched crates. One `make test` on the final commit. Signed commits. Land with `scripts/land.sh` when green.


## Report from seat:01M6GVDER4NMBZMRX9RF479SV9

Landed signed commit 0bb8e56b2a13b9b7c6104a1bb89be26070cc794c on origin/main. protocol! generates private finite state, typed events/effects/reasons, a pure simultaneous reducer and the optional symbolic model, with stable completeness/type diagnostics. LatestResult passes at 21 states/105 edges; the broken declaration fails at 31/155 and replays Submit(A), Submit(B), Complete(A). All reachable reducer edges are compared; ProtocolLanguage adds 48 states/2352 edges. Final make test exited 0 (2566 tests passed, 211 skipped), strict Clippy and license checks passed, and native/Java TLC matched all 264 models across 34 crates/108 contracts with clean source-bound evidence. The first full run's two missing-task record failures were resolved by inheriting Oracle commit cf7c96aa and rebasing; the complete final gate then passed. Landing reused that exact-commit full log and exited 0; make check-formal-evidence exited 0; worktree is clean. Effect execution, async adapters and larger domains remain unmodeled. Full report: [[reports/2026-11-03-yarn-protocol-frontend]].

```json
{
  "op_id": "01M6H1SR1V5VBPJQZSC4CE1HJM",
  "commit": "0bb8e56b2a13b9b7c6104a1bb89be26070cc794c",
  "report": "[[reports/2026-11-03-yarn-protocol-frontend]]",
  "verify_result": {
    "command": "make test; sh scripts/land.sh --gate-log target/ci-local.log; make check-formal-evidence",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Generated pure Rust, finite types and completeness diagnostics",
        "reference": "crates/bbt-yarn-macros/src/protocol.rs:853,1338,1514; crates/bbt-yarn/src/protocol.rs:14; make test-yarn-model exit 0; 14 protocol ui-fail cases and protocol_hygiene ui-pass"
      },
      {
        "criterion": "21 reachable states, RESULT-001 and exhaustive reducer replay",
        "reference": "crates/bbt-yarn/tests/protocol.rs:112 — all 21 states and 105 edges; target/ci-local.log — LatestResult pass, 21 states, 105 edges match"
      },
      {
        "criterion": "Broken freshness control and exact counterexample replay",
        "reference": "crates/bbt-yarn/tests/protocol.rs:179 — Submit(A), Submit(B), Complete(A), desired=B/visible=A, invariant false; target/ci-local.log — BrokenLatestResult safety, 31 states, 155 edges match"
      },
      {
        "criterion": "Native/Java TLC graph/verdict parity on current clean source",
        "reference": "target/ci-local.log — 264 models, 34 crates, 108 contracts; source 0bb8e56b2a13b9b7c6104a1bb89be26070cc794c, dirty=false; receipt sha256 3a684ad96335ac586666b51ec3c5dda97cb4acbea8c0d92ad4b632f9a1463de2; target/protocol-formal-receipt.log — make check-formal-evidence exit 0"
      },
      {
        "criterion": "Source/model mapping, broken control, syntax and scope documentation",
        "reference": "verification/coverage.toml:131; verification/contracts.toml:747; verification/corpus.toml; crates/bbt-yarn/README.md — Pure finite protocols; effect execution/async adapters/larger domains explicitly unmodeled"
      },
      {
        "criterion": "Signed final commit, canonical full gate and landing",
        "reference": "target/ci-local.log — ci-local: result 0bb8e56b2a13b9b7c6104a1bb89be26070cc794c normal 0; target/protocol-land.log — LANDED 0bb8e56b2a13b9b7c6104a1bb89be26070cc794c and exact-candidate full gate reuse; git log G with 9B07E849480C68328CA20684844ECC0587DE7AB0; origin/main equals 0bb8e56b2a13b9b7c6104a1bb89be26070cc794c; git status --short empty"
      }
    ]
  }
}
```
