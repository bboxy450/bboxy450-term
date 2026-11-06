# Landing on main

One command, from your own branch in your own worktree:

```sh
make land
```

It runs `scripts/land-fast.sh` (`make push` is the same command):

1. Rebase on `origin/main` (exit 78 on conflict); refuse unsigned commits.
2. Compute the plan with `scripts/landing.py lean`: the crates whose files changed
   ("touched") and every crate that depends on them. `Cargo.toml`, `Cargo.lock`, `.cargo/`
   and `rust-toolchain.toml` touch every crate; docs/lab-only diffs touch none.
3. **Lean gate (default):** `cargo clippy -D warnings` over the touched crates; for each
   touched crate, nextest over its lib and bin tests (when its source changed) plus each
   changed integration-test file; `cargo check` of the dependents. A test-only change runs
   only that test. `.bboxy/` changes add the doc corpus test; model or `verification/`
   changes add `make test-formal`. A nonempty integration target skipped entirely by
   `#[ignore]` is metadata-checked and reported as compiled only; its native Make check
   remains explicit. Empty filters and build/test failures still fail the gate.
   `LAND_MODE=wide make land` instead lints and tests the
   whole affected set, doctests included.
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
5. **Keep build headroom and bounded fixture space.** Recovery reserves 128 MiB plus
   1 MiB per seat (132 MiB for the default four-seat run), checks retained file bytes
   against that budget, and reports usage. It does not require a blanket 10 GiB free.
   Prune your own build dirs only.
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

## Formal evidence retention

After a successful `make land` or `land.sh`, evidence larger than 5 GiB schedules
maintenance detached at background QoS (`taskpolicy -b`, nice 19 on macOS).
`FORMAL_EVIDENCE_BUDGET_BYTES` changes the trigger. Maintenance failure is logged
in the private evidence store's `prune.log` and does not change a landing verdict.

`make prune-formal-evidence` keeps the newest `KEEP=10` receipts, all receipts
for `origin/main` and extant worktree HEADs, and evidence/source hashes cited by
reports whose tasks remain unaccepted (unknown tasks are conservatively pending).
`DRY_RUN=1` lists the counts without writing. Other receipts go to one monthly
`bboxy-formal-evidence-archive/receipts-YYYY-MM.tar.zst` beside the Git evidence
store. Tar hardlinks also deduplicate identical compressed graphs across monthly
appends. Its `.tar.sha256.json` manifest binds the archive and every member; frame
integrity, exact file count and all hashes must pass before any original is
removed. Appends verify the previous archive first. Broken custody retains the
originals. Pins are refreshed after archive IO, before removal.

New receipts store `.dot`, `.states` and `.edges` as zstd files hardlinked to
`by-model/graphs/<uncompressed-sha256>.zst`. Hashing and reuse decode the original
bytes. Each receipt owns a hardlink, so index cleanup cannot invalidate readers.
Legacy raw receipts remain readable and follow the same retention policy. The
zstd executable is required for writing or reading compressed evidence; no
product-runtime dependency is added. Concurrent maintenance skips its busy lock;
formal producers and receipt readers never acquire that lock.
