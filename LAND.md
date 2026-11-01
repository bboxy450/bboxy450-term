# Landing on main

One command, from your own branch in your own worktree:

```sh
make land
```

It runs `scripts/land.sh`: rebase on `origin/main`, the full signed gate on that exact
head, an incremental recheck of only what overlaps if `main` moved meanwhile, then
`git push origin HEAD:main`. It uses a private Cargo build dir and retries only transient
network or TLC-tool failures. Push and merge still need the operator's authorization.

Use `make land-full` when the brief requires the full normal `make test` gate.
It runs that gate after the initial rebase, for any cut tier, then retains the
same incremental rechecks and current-source formal receipt checks if main moves.
`sh scripts/land.sh --full` is the equivalent entry point. A supplied
`--gate-log` remains bound to its exact candidate; it cannot be combined with `--full`.

## Why this way: measured timings (2026-10-30 to 10-31)

| Measured | Value |
|---|---|
| Time between landings on `main` (25 landing events) | median 33 min, a quarter under 24 min, shortest 3.6 min |
| One full gate on this machine (`land.sh`, 7 runs) | median 24 min, p90 30 min, max 51 min |
| Plain `make test` then `make push` | lost the race to a newer `main` 2 of 2 times |

The gate takes about as long as `main` stays still, so "test, then push" usually fails
its own ancestry check. `land.sh` keeps the tested result and rechecks only the overlap,
so it converges.

What cost full reruns in that session, each now prevented:

| Failure (runs lost) | Prevention |
|---|---|
| Rebase conflicts on shared lines (5) | Rule 1 |
| Doctests "can't find crate" from other worktrees' artifacts (2) | `make land` sets a private `CARGO_BUILD_BUILD_DIR` |
| Fixture plugins over a fixed 2 s budget under load (3) | Rust fixture plugins; test-only budgets. Quarantine only per `.bboxy/quarantine.md` |
| DNS outage during the TLC download (1), TLC parse race (1) | Automatic transient retry |
| Free disk below the 10 GiB recovery check (1) | Rule 5 |
| Layering order (1), installed binary name (1), dangling record link (1) | Rules 2–4 |

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
- **Exit 77, gate failed:** the log names the failing check. A real failure is yours to
  fix. A test that fails only under load and passes alone may be quarantined only per
  `.bboxy/quarantine.md` (owner, task, at most 7 days).
- **Other:** `target/land-attempt-<n>.log` holds each attempt.
