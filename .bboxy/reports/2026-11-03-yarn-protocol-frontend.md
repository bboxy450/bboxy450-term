---
type: report
title: 'Yarn protocol!: one declaration, pure reducer and finite model'
task: '#14637'
commits: []
author: seat:01M6GVDER4NMBZMRX9RF479SV9
created: 2026-11-03T14:50:24.047Z
verify:
  command: make test-yarn-model && make test-licenses
  exit_code: 0
id: 01M6GY7KHFQYKWSKY57KEP4P82
updated: 2026-11-03T14:50:24.047Z
number: 14652
created_by: seat:01M6GVDER4NMBZMRX9RF479SV9
updated_by: seat:01M6GVDER4NMBZMRX9RF479SV9
authorship:
  signer: seat:01M6GVDER4NMBZMRX9RF479SV9
  kind: seat
  public_key: OP2j4PFDYJxszYX3NskoHvNggJq2vKDQxOBddx4Qf7g=
  process_generation: '1'
provenance:
  digest: sha256:32699b1a5544009c244c167760fe7faa8643549899646636c590743ecc432f4f
  frontmatter_digest: sha256:9d8d551436860a391f0ec311bff8743b1dc0d4bec9a1fb0cba0d6151a4812ffe
  signer: seat:01M6GVDER4NMBZMRX9RF479SV9
  sig: ho92X8i8O3FGfF+ftdDuTtnohU63o4wXHdYYGLEWapLA+pSNQmMQ8XLBz/H2ZAuozBH2aULKdP+wUt/OChIZBA==
  log: .bboxy/log/2026-11-03.part-86213.jsonl#86904
---
# Yarn protocol!: pure reducer and finite model

Task #14637 supersedes #13806 in place. The Oracle's 2026-11-03 ruling classifies the five new independent protocol files as project-owned. The first amendment was refused twice with task_field_protected; the replacement Delegate explicitly authorizes only adding these files to scripts/source-notices.py OWNED_FORK_FILES. That is the only change to that script; classification logic and inherited notices remain intact.

The closed finite protocol language now generates private State/getters/declared Default, typed Event/Effect/reason enums, a pure step, invariant functions and an optional symbolic native/TLA+ model from one typed AST. No arbitrary Rust calls execute within the language.

- crates/bbt-yarn-macros/src/protocol.rs:36 — parses the closed grammar; check validates finite types, field/event completeness, unique invariant IDs and disjoint/exhaustive guards over all declared event parameters crossed with all guard-read state values (including short-circuit operands). No reachability or invariant pruning; universes over 65,536 fail.
- crates/bbt-yarn-macros/src/protocol.rs:1338 — generated reducer uses pre-state for all assignments and emitted arguments; rejects/ignores prohibit updates and effects. Rust primitive/Default paths and internal bindings are hygienic.
- crates/bbt-yarn-macros/src/protocol.rs:1514 — symbolic lowering uses existing TLC expressions, scalar ordinals/None=-1, and one Boolean variable per set member. No graph is tabulated into the exported model.
- crates/bbt-yarn/src/protocol.rs:14 and crates/bbt-yarn/src/lib.rs:30 — finite decisions/sets, macro re-export and dependency-feature model selection; reducers use core+alloc and do not require std/model.
- crates/bbt-yarn/tests/protocol.rs:11,58,112,179 — LatestResult and the freshness-broken declaration, exhaustive replay and the exact counterexample.
- crates/bbt-yarn/examples/verify.rs:16 — exports these same test declarations through the existing native/TLC path, with the gate's existing contract metadata.
- verification/corpus.toml, verification/coverage.toml, verification/contracts.toml — inventory and bounded contract pair/source mapping; README documents grammar, semantics, results and limits.

Acceptance evidence:

1. LatestResult has exactly 21 reachable states, RESULT-001 holds, and all 105 enabled labeled edges are replayed through generated step. The test checks all reachable runtime states, every concrete event once per state, effect values, rejected/ignored self-loops and immutable input state.
2. BrokenLatestResult removes the pending/current-desire freshness checks from Complete::current and adjusts the complementary stale case. Native BFS yields Submit(A), Submit(B), Complete(A); replay produces desired=B, visible=A and a false generated invariant.
3. Native and pinned Java TLC agree: LatestResult pass/21 states/105 edges; BrokenLatestResult safety/31/155; ProtocolLanguage pass/48/2352. ProtocolLanguage additionally checks bool/domain/Option/Set event and state values, simultaneous swapping, pre-state effect arguments and effect order.
4. Fourteen new ui-fail cases include one each for missing event, missing field accounting, overlapping guards, uncovered guards (with explicit parameter × state universe), missing initializer and duplicate invariant ID. Other controls cover valid arbitrary Rust calls, unsupported syntax/type, domain mismatch, effectful reject, private fields, model identifiers and resource exhaustion. A ui-pass fixture covers primitive/Default/internal-binding name collisions.
5. Effect execution, async adapters and domains larger than declared remain unmodeled. No fairness/liveness, arbitrary Rust refinement, or full TLC/frontend completeness claim is made. Id={A,B}; each is submitted once. Cancellation preserves in-flight pending IDs; A → B → A reuse is rejected within this universe.

Completed commands and actual exits:

- make init-submodules — 0; initialized this worktree's exact pinned harness after the first cargo check found it absent.
- cargo test -p bboxy450-yarn --all-features --test protocol — 0, three tests.
- cargo nextest run --locked -p bboxy450-yarn-macros — 0, nine tests.
- cargo test --locked -p bboxy450-yarn --no-default-features --features macros --test protocol_runtime — 0; reducer without model/Tokio.
- cargo test --locked -p bboxy450-yarn --all-features --doc — 0, three doctests.
- make test-yarn-model — 0 on the final reducer source, including normal (non-overwrite) trybuild, native/model tests, core/model feature checks and strict all-target Clippy for yarn/macros/tlc. Logs: target/protocol-yarn-model.log.
- make test-formal — 0, 264 models/34 crates/108 contracts; target/protocol-formal.log. This exploratory precommit receipt records dirty=true. The later primitive/name-hygiene changes alter no model transition expressions. The final committed-source full gate must refresh clean formal evidence.
- make test-licenses — 0 after the authorized five-file ownership inventory addition; 34 crates, six policy tests and nine source-notice controls. Earlier exit 2 was the ownership-classification blocker, resolved under #14637 without changing any source license or upstream notice. Log: target/protocol-licenses.log.
- An initial make test-formal exited 2 because contract corpus entries were labeled yarn rather than project; corrected their inventory classification while retaining the Yarn exporter. Earlier Clippy findings were fixed; final strict lint passed.

Scope excludes effect adapters, Kani/Verus/Loom, requirements!/verify!, CLI/JSON report formats and typestate handles. bboxy450-tlc required no changes. Source headers preserve project policy and credit independent inspiration from Verus state_machine! and machine-check.

The required exact-committed-source make test (including formal, unit/integration and private smoke), followed by sh scripts/land.sh --gate-log target/ci-local.log, runs after this report's containing signed commit. Their actual exits, final signed SHA, formal receipt and worktree state must be included in the final native task_report. This pre-gate report does not claim a future gate passed. commits: [] avoids self-reference; the signed Report trailer identifies the containing source commit.
