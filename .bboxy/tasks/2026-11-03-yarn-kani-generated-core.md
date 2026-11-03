---
type: task
title: 'Yarn: Kani proofs over generated protocol reducers, repo-local pinned toolchain'
role: executor
verify: make test-yarn-kani
id: 01M6H3E1Q6EE9YK3JTZM2PA6VD
status: open
delegated_by: seat:01M6GVBAY1HW6P67GGDZM9XEBY
swarm: 01M6CQARWH5A79TE3H4NNWJ4XG
updated: 2026-11-03T16:21:17.937Z
created: 2026-11-03T16:21:17.937Z
number: 15237
created_by: seat:01M6GVBAY1HW6P67GGDZM9XEBY
updated_by: seat:01M6GVBAY1HW6P67GGDZM9XEBY
authorship:
  signer: seat:01M6GVBAY1HW6P67GGDZM9XEBY
  kind: seat
  public_key: UVOPOaWiKiIlf3/HRaRx9L4rRJKHPpaAXciElGQdQaE=
  process_generation: '1'
provenance:
  digest: sha256:9920988b275a6f0cf04703686874f90474d19d2e38f3b0397db1d9119cf66ad9
  frontmatter_digest: sha256:f2767fe0c73924e857c22e34467707cf76ccd0d786ab126f1bb66998d267239b
  signer: seat:01M6GVBAY1HW6P67GGDZM9XEBY
  sig: bIVi22gLRlbIugyJXAoPZ1vtiA8BR0758duuJsewWsI+4FpXkYLM3xFhiKccPGzliRQKZ1K3YAlkxcKaNj3gBg==
  log: .bboxy/log/2026-11-03.part-86213.jsonl#90869
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
