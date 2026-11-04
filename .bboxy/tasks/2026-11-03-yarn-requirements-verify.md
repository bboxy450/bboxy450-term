---
type: task
title: 'Yarn requirements!/verify!: required claims, reachability, structured results, contract baseline'
role: executor
verify: make test-yarn-model && make test-formal
id: 01M6H3DE4S54HVSGXCAKPNQ093
status: done
delegated_by: seat:01M6GVBAY1HW6P67GGDZM9XEBY
swarm: 01M6CQARWH5A79TE3H4NNWJ4XG
updated: 2026-11-04T02:31:46.767Z
created: 2026-11-03T16:20:57.837Z
number: 15235
created_by: seat:01M6GVBAY1HW6P67GGDZM9XEBY
updated_by: seat:01M6HF0S1BAJKHWT2N426W7328
authorship:
  signer: seat:01M6HF0S1BAJKHWT2N426W7328
  kind: seat
  public_key: NJfxjXALG4vfTtn9XxMx7XCD2GZh2WlrBe8FFTqIaLM=
  process_generation: '1'
reconciliation:
- kind: historical
  fresh_accepted: false
  previous_status: open
  disposition: implemented
  reason: Operator explicitly requests closing the completed KB tasks as done. Implementation is on main. The final combined landing used the operator's explicit CI waiver; this is historical implementation disposition, not fresh acceptance or a claim of rerun current-source CI.
  evidence:
  - Signed integration 67dda793, ancestor of remote main f01428b8 and its later descendants.
  - 'Task #19660 records the operator''s one-time CI/formal/local-hook waiver and the normal fast-forward push. Root independently checked all new signatures, whitespace and clean Git state. Original source branch/test histories retained; no new combined-source CI pass claimed.'
  by: seat:01M6HF0S1BAJKHWT2N426W7328
  op_id: 01M6J6BVJKH3E1Q0G7DE5WDYWW
provenance:
  digest: sha256:8b3c4814014cd0523824a4fa5d16457471794e1ff7acdabd922f71e6170b8a6c
  frontmatter_digest: sha256:d19a7c79c474df51ed4c8447c6cf92fabfcae9175b6130415207212f66691ba0
  signer: seat:01M6HF0S1BAJKHWT2N426W7328
  sig: 7jfhVPKlunzhRRpbUcQy09oHqHl4rQp6dCHU78+qcBLyjNWKICcgWcAMmLPYfweg+gvQwVtxWB1EGFCGLlavBg==
  log: .bboxy/log/2026-11-04.jsonl#110701
---
# Yarn `requirements!` / `verify!` and honest results

ROLE: EXECUTOR. Worktree `~/mydev/bboxy450-term-yarn-verify`, branch `yarn-verify`, from `origin/main`. Put your macros in a **new** module (`crates/bbt-yarn-macros/src/verify.rs`) and runtime in `crates/bbt-yarn/src/verify.rs`. In `protocol.rs` (owned by the temporal executor) make only small, additive changes: exposing case labels, invariant ids and declaration spans as generated metadata.

## Outcome

1. `requirements! { pub ResultContract { "RESULT-001" => LatestResult::result_is_current; ... } }` names the claims a project requires, by stable id.
2. `verify! { #[test] fn ...() { contract: ..; protocol: ..; exploration: exhaustive; require_reachable: [Submit::fresh, Complete::current, ..]; limits { states: N; on_exhaustion: inconclusive; } } }` generates a deterministic test that:
   - fails if a required id has no matching invariant or property in the protocol, and fails if a protocol invariant is not required (no silent extras);
   - fails if a `require_reachable` case never fires in the exhaustive graph;
   - reports exhaustion of the state limit as **inconclusive**, which fails the test; it never passes.
3. **Structured result**: a `VerificationResult` serialisable to JSON (hand-rolled or `serde_json` behind the `model` feature; pinned version) with separate `outcome` (established | refuted | inconclusive | invalid_input | not_run), `method` (exhaustive_finite_model, ...), `scope` (domains and their sizes, bounds, fairness assumed), `implementation` (generated_core | replay_checked), `provenance` (source hash of the declaration, tool versions), and a `counterexample` as event names plus the failed condition. Written under `target/` for every `verify!` run; one example is committed under `verification/` as a golden file test.
4. **Contract baseline (requirement-weakening guard)**: `verification/protocol-contracts.toml` records each contract's required ids, domains/bounds and assumptions. A check (Make target `check-protocol-contracts`, wired into `make test-formal`) fails when a required id is removed, a domain shrinks, an assumption or fairness is added, or a waiver appears, unless the baseline file changes in the same commit. It prints the contract diff. Add a broken control: a test fixture that drops RESULT-001 and must fail.
5. Apply all of the above to `LatestResult` in `crates/bbt-yarn/tests/protocol.rs`; README section.

Out of scope: liveness syntax and effects (temporal executor), the CLI (next wave), Kani.

Allowed paths: `crates/bbt-yarn-macros/**`, `crates/bbt-yarn/**`, `verification/**`, `Makefile` (new targets only, plus wiring into `test-formal`), `scripts/**` (new files only, plus `OWNED_FORK_FILES` additions), `.bboxy/reports/**`.

## Rules

Work autonomously; one final `task_report` with a committed report under `.bboxy/reports/` and a `Report:` trailer. **The report's `verify.command` must be this task's `verify` string verbatim.** Source headers per `ATTRIBUTION.md`, with "Inspired by" credit (e.g. Quint, TLA-Verus statement-preservation checks). Strict clippy, one `make test` on the final commit, signed commits, exact-path staging, land with `scripts/land.sh`. Other Yarn executors run concurrently; rebase through land.sh.
