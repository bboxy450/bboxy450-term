---
type: task
title: 'Yarn: Kani proofs over generated protocol reducers, repo-local pinned toolchain'
role: executor
verify: make test-yarn-kani
id: 01M6H3E1Q6EE9YK3JTZM2PA6VD
status: done
delegated_by: seat:01M6GVBAY1HW6P67GGDZM9XEBY
swarm: 01M6CQARWH5A79TE3H4NNWJ4XG
updated: 2026-11-04T02:31:53.973Z
created: 2026-11-03T16:21:17.937Z
number: 15237
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
  - Signed integration f01428b8867ddfeee33105022710ffb92920b101, ancestor of remote main f01428b8 and its later descendants.
  - 'Task #19660 records the operator''s one-time CI/formal/local-hook waiver and the normal fast-forward push. Root independently checked all new signatures, whitespace and clean Git state. Original source branch/test histories retained; no new combined-source CI pass claimed.'
  by: seat:01M6HF0S1BAJKHWT2N426W7328
  op_id: 01M6J6BYDBVG68Z16WQ5GVQGJ8
provenance:
  digest: sha256:9920988b275a6f0cf04703686874f90474d19d2e38f3b0397db1d9119cf66ad9
  frontmatter_digest: sha256:58292919584b544f0b5edeff667485c351e3f20e8933ee706f9bbdc92c89159f
  signer: seat:01M6HF0S1BAJKHWT2N426W7328
  sig: bEyMQ9yOhoMaH9r4DOAl5mqde4vpvlyEoZId9UwpVdcWiwXJe1ySMmc9wEWwIBbYW3kv6dJsEwQEMosLL/CHBA==
  log: .bboxy/log/2026-11-04.jsonl#110732
---
# Kani over generated `protocol!` code

ROLE: EXECUTOR. Worktree `~/mydev/bboxy450-term-yarn-kani`, branch `yarn-kani`, from `origin/main`.

## Outcome

- A Make target `test-yarn-kani` that installs a **pinned** Kani version into a repo-local tool directory (under `target/` or the existing cache dir from `scripts/cache-env.sh`; never `~/.cargo/bin` or a global path). The first run downloads it; there is no operator prompt. It then runs the harnesses.
- Harnesses in a new `crates/bbt-yarn/tests/kani/` (or `crates/bbt-yarn/kani/`, whichever Kani's cargo integration supports) over the generated `LatestResult` (`crates/bbt-yarn/tests/protocol.rs` declaration; reuse it, do not copy it):
  - `step` never panics for **any** `State` value reachable through the public API and any `Event` (`kani::any` over the finite domains);
  - for every accepted step from a state satisfying `result_is_current`, the next state satisfies it too (an inductive step, with no finite reachability needed). Report whether Kani proves it outright or needs a strengthened invariant, and add that strengthened invariant as a declared `#[invariant]` only if the protocol owner's file permits it; otherwise keep it in the harness and say so;
  - reject/ignore return a state equal to the input.
  - Broken control: the freshness-broken declaration must fail the inductive harness.
- Results are reported honestly: the outcome per harness and the Kani version. If Kani cannot run on this macOS arm64 machine, the target fails loudly (never a silent pass) and the report names the exact error. In that case, deliver the harnesses plus the CI-ready target anyway.
- `verification/coverage.toml`/`contracts.toml`: add the method `kani_bounded_proof` with its unwinding bounds.

Allowed paths: `crates/bbt-yarn/**` (new kani dir, Cargo.toml `[package.metadata.kani]`/cfg only), `Makefile` (new target and help line), `scripts/**` (new files only, plus `OWNED_FORK_FILES` additions), `verification/**`, `.bboxy/reports/**`. Do not edit `crates/bbt-yarn-macros/**` or `crates/bbt-yarn/src/protocol.rs`; if generated code blocks Kani, report the exact blocker.

## Rules

Work autonomously; one final `task_report` with a committed report under `.bboxy/reports/` and a `Report:` trailer. **The report's `verify.command` must be this task's `verify` string verbatim.** Source headers per `ATTRIBUTION.md`, with "Inspired by" credit. Strict clippy, one `make test` on the final commit, signed commits, exact-path staging, land with `scripts/land.sh`. Other Yarn executors run concurrently; rebase through land.sh.
