# O — open-source libraries (epic)

Owner: Vesper Greywind <ORACLE>. Spec: [[open-source]]. One step per Executor assignment, one signed commit each; the Oracle publishes.

## O1 — `bboxy450-rt` ready to publish

DEFERRED (operator, 2026-10-22: velocity first; release when ready, never inside the build loop). Assignee when picked up: the runtime's owner. Allowed: `crates/bbt-rt/**` → renamed `crates/bboxy450-rt/**`, every `Cargo.toml` that names it, `Cargo.lock`, `clippy.toml`, `scripts/ui-lint.sh`, `.bboxy/runs/o1-*.log`, LICENSE-MIT and LICENSE-APACHE at the repo root and in the crate.
1. Rename the crate to `bboxy450-rt` (package and directory); update its users (app, mux if any). Library name `bboxy450_rt`.
2. Docs to [[open-source]] §2. README as crate doc, all sections, `deny(missing_docs)`, a doctest per entry point, and `examples/streaming_command.rs` plus `examples/ui_wake.rs`.
3. Cargo metadata: description, repository `https://github.com/bboxy450/bboxy450-rt`, documentation, keywords (`async`, `ui`, `runtime`, `tokio`, `gui`), categories, readme, `rust-version`.
4. `cargo package -p bboxy450-rt` builds from the packaged tarball alone; `cargo doc -p bboxy450-rt --no-deps` has zero warnings.
Verify: tests, clippy `-D warnings`, doc tests, `cargo package`, `cargo publish --dry-run`. Hand off the docs.rs-style page path (`target/doc/bboxy450_rt/index.html`) for review.

## O2 — split `bbt-ui-model` and `bbt-ui` into public and private parts

After W3.2d.7.10. Move domain types (model/seat, swarm, tab, window; layout/sidebar) and `bbt-ui/src/modules` into `crates/bbt-term-ui`; rename the rest to `bboxy450-ui-core` and `bboxy450-ui`; design data moves into `bboxy450-ui-core`; the theme becomes a parameter. Pixel-identical galleries are the acceptance.

## O3 — `bboxy450-ui-core` docs and publish; O4 — `bboxy450-render`; O5 — `bboxy450-ui`; O6 — `bboxy450-book`

Each one: the O1 checklist applied to that crate.
