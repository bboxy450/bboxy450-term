# Landing on main

One command, from your own branch in your own worktree:

```sh
make land
```

It runs `scripts/land-fast.sh`:

1. Rebase on `origin/main` (exit 78 on conflict).
2. `cargo check --workspace --all-targets`, then clippy `-D warnings` and `cargo test`
   for each package whose files changed. `.bboxy/` changes add the doc corpus test;
   model or `verification/` changes add `make test-formal`.
3. Push. If `main` moved, rebase, recompile the workspace and push again (up to 5 times).

Cargo runs under `nice -n 10` on half the cores (`CARGO_BUILD_JOBS`) with a private build
dir, so live seats keep working while you land.

`make land-full` is the old full signed gate (`scripts/land.sh --full`). Use it when a
brief asks for the full suite or a change touches shared core behaviour broadly.

## Why fast by default

This laptop runs many seats at once (16 cores, load 7-10). The full gate took a median
of 24 min (p90 30, max 51) while `main` moved every ~33 min, so agents sat in the gate and
then raced. The fast gate compiles everything, tests what you changed, and leaves the full
suite to `make land-full`. A red `main` is fixed forward by whoever broke it.

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
