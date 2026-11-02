# Landing on main

One command, from your own branch in your own worktree:

```sh
make land
```

It runs `scripts/land-fast.sh` (`make push` is the same command):

1. Rebase on `origin/main` (exit 78 on conflict); refuse unsigned commits.
2. Compute the affected set with `scripts/affected.sh`: crates whose files changed plus
   every crate that depends on them. `Cargo.toml`, `Cargo.lock`, `.cargo/` and
   `rust-toolchain.toml` select every crate; docs/lab-only diffs select none.
3. One `cargo clippy -D warnings` and one `scripts/quarantine-test.sh` run over that set.
   The test run is cargo-nextest: all affected test binaries run concurrently, a hung test
   dies after 5 min, doctests follow. `.bboxy/` changes add the doc corpus test; model or
   `verification/` changes add `make test-formal`.
4. Push. If `main` moved, rebase, recheck the affected crates and push again (up to 5 times).

Cargo runs under `nice -n 10` with 4 jobs (`BBOXY_CARGO_JOBS`) and 2 test threads, reusing
your worktree's warm `target/` and the shared sccache, so live seats keep working.

`make land-full` is the full signed gate (`scripts/land.sh --full`): `make test` plus a
formal receipt. Gate slots keep their checkout and target warm between landings, so only
changed crates rebuild; one slot runs at a time (`BBOXY_GATE_SLOTS`, at most 3) and other
landings queue for it instead of failing (`BBOXY_GATE_WAIT=0` restores exit 75); it uses 4
Cargo jobs (`BBOXY_GATE_JOBS`). Use it for
a release, a toolchain change or when a brief asks for the full suite.

## Why diff-scoped by default

This laptop runs many seats at once (16 cores, load 7-36). The full gate took a median
of 24 min (p90 30, max 51) while `main` moved every ~33 min, so agents sat in the gate and
then raced. Measured cost was mostly compilation in a fresh, empty gate target (10-25 min)
and test binaries run one after another (8-15 min). The diff-scoped gate builds warm and
tests the affected crates in parallel. A red `main` is fixed forward by whoever broke it.

## Rules

1. **Stay off shared hot lines.** Every agent edits these, so touching them conflicts with
   every landing: the `families` array in
   `crates/bboxy450-term/tests/formal_contracts/models.rs` and the `bboxy450-term-mux`
   entry in `verification/coverage.toml`. Put new model families in their own module
   appended with one `cases.extend(...)` line (see `plugin_models.rs`), and register them
   under your own crate's coverage entry.
2. **New crate:** in the same commit, add it to `scripts/layering.sh` `ORDER`, classify it
   in `.bboxy/specs/licensing.md` (decision 362), and add a `verification/coverage.toml` entry.
3. **Installed binaries equal the package name** (decision 362, `scripts/brand.sh`). Make
   helper tools examples, not binaries.
4. **Links in committed `.bboxy` records must resolve.** Never link to an untracked local
   record.
5. **Keep at least 10 GiB free** before landing. Prune your own build dirs only.
6. **Every commit compiles on its own.** Run `cargo check --workspace --all-targets` per
   commit when you rebase several.
7. If your default `rustc` is older than the workspace `rust-version`, export
   `RUSTUP_TOOLCHAIN` for the run. Never change global Cargo configuration.

## When it stops

- **Exit 78, conflict:** resolve the rebase by keeping both sides (rule 1 makes this
  rare), then `make land` again.
- **Exit 77, gate failed:** the output names the failing check. A real failure is yours to
  fix. A test that fails only under load and passes alone may be quarantined only per
  `.bboxy/quarantine.md` (owner, task, at most 7 days).
- **Exit 75:** `main` kept moving; run `make land` again.
- `make land-full` keeps its logs in `target/land-attempt-<n>.log`.
