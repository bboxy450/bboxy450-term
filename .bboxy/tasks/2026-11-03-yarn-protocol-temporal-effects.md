---
type: task
title: 'Yarn protocol!: liveness/fairness properties and effect lifecycle (admission, commit, unknown outcome)'
role: executor
verify: make test-yarn-model && make test-formal
id: 01M6H3D24MTW3923FT6VVMQHAG
status: open
delegated_by: seat:01M6GVBAY1HW6P67GGDZM9XEBY
swarm: 01M6CQARWH5A79TE3H4NNWJ4XG
updated: 2026-11-03T16:20:45.984Z
created: 2026-11-03T16:20:45.984Z
number: 15234
created_by: seat:01M6GVBAY1HW6P67GGDZM9XEBY
updated_by: seat:01M6GVBAY1HW6P67GGDZM9XEBY
authorship:
  signer: seat:01M6GVBAY1HW6P67GGDZM9XEBY
  kind: seat
  public_key: UVOPOaWiKiIlf3/HRaRx9L4rRJKHPpaAXciElGQdQaE=
  process_generation: '1'
provenance:
  digest: sha256:78ae90c29c36ebe6264f17620b45987bc3419788a45f3e95a478ae5a9aa37ff1
  frontmatter_digest: sha256:5a9eb09513b87fcdee6e75cc693a91e6bac350f6743b31f319c17b715918de7b
  signer: seat:01M6GVBAY1HW6P67GGDZM9XEBY
  sig: QxJtmn+Y2gPCXGtbU6DpnzAGkke48+4q3xY46S8oI0iYkTDPYlW4/CFgHE38vVpFGVRjjMGB+eXo9HaotgeIDQ==
  log: .bboxy/log/2026-11-03.part-86213.jsonl#90859
---
# Yarn `protocol!` stage 4: temporal properties and effects

ROLE: EXECUTOR. Worktree `~/mydev/bboxy450-term-yarn-temporal`, branch `yarn-temporal`, from `origin/main`. **You own the semantics of `crates/bbt-yarn-macros/src/protocol.rs`**; the `requirements!/verify!` executor makes only small, additive metadata changes there.

## Outcome

1. **Temporal properties in the declaration**, lowered onto the existing `bboxy450_tlc` response/fairness support (`Model.responses`, `Model.weak_fair`):
   - `#[property(id = "...", leads_to(P, Q))]` over state predicates;
   - fairness attached to **named cases**: `#[fair(weak)]` on a case (and `#[fair(strong)]`, only if `bboxy450-tlc` supports it or you add it to the native checker and the TLA export together, with parity). There is no global "runtime is fair";
   - explicit `terminal`/`quiescent` state predicates, so a self-loop on ignore does not count as progress. Stuttering on ignore/reject must not discharge `leads_to`.
2. **Effect lifecycle as protocol steps.** An effect intent emitted by a case is admitted and then completed by later events, never in the same step. Provide a declarative form that generates the completion events (e.g. `effect Fetch(id) completes { Succeeded, FailedBeforeEffect, UnknownOutcome, CancelledBeforeStart }`), so that every outcome must be handled by the completeness checks: each generated completion event needs `on` coverage. Revalidation at commit time is expressed as an ordinary guard on the completion case (e.g. generation equality).
3. **Witness protocol** `crates/bbt-yarn/tests/protocol_temporal.rs`, at exact small bounds:
   - a generation-checked `Present { frame, expected_generation }`-style protocol: a stale completion (generation moved on) is ignored, and the safety invariant holds;
   - liveness: "if the current request remains selected and its fetch completes (weak fairness on the completion case), it eventually becomes visible" holds;
   - **broken controls, each refuted with a counterexample asserted in the test**: (a) remove the generation guard → safety violation; (b) remove the fairness annotation → liveness violation with a lasso/cycle; (c) a retry on `UnknownOutcome` without an idempotency/dedup guard → duplicate-commit invariant violated.
   - All edges replayed through the generated `step`, as `tests/protocol.rs` does today.
4. New trybuild ui-fail cases: an unhandled completion outcome; fairness on an unknown case; `leads_to` naming an unknown predicate.
5. Native/TLC parity for every new model through `make test-formal`; entries in `verification/coverage.toml` and `contracts.toml` with the broken controls; README section stating what a liveness result assumes.

Out of scope: `requirements!`/`verify!`, the CLI, Kani, typestate handles.

Allowed paths: `crates/bbt-yarn-macros/**`, `crates/bbt-yarn/**`, `crates/bboxy450-tlc/**` (additive), `verification/**`, `.bboxy/reports/**`, `scripts/source-notices.py` (`OWNED_FORK_FILES` additions only).

## Rules

Work autonomously; one final `task_report` with a committed report under `.bboxy/reports/` and a `Report:` trailer. **The report's `verify.command` must be this task's `verify` string verbatim.** Source headers per `ATTRIBUTION.md`, with "Inspired by" credit beside borrowed designs. Strict clippy on touched crates, one `make test` on the final commit, signed commits, exact-path staging, land with `scripts/land.sh`. Other Yarn executors run concurrently; rebase through land.sh.
