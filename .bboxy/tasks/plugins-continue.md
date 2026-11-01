---
id: "01M6AD23JS424P4FGVQCQFEDZT"
type: "task"
number: 505
title: "plugins-continue: native checker gaps and many-plugin plugin models (follow-up to #377)"
status: "open"
role: "executor"
verify: "make test-formal"
created: "2026-11-01T02:00:00Z"
updated: "2026-11-01T02:00:00Z"
---
# plugins-continue

Pick this up on any machine. Everything it needs is on `main`.

## Start here

1. Clone or update `https://github.com/bboxy450/bboxy450-term` and make a branch in a
   worktree of your own (other agents may share the main checkout).
2. Read `AGENTS.md`, then `LAND.md` (one-command landing, why it works that way, the rules),
   then task #377 (`.bboxy/tasks/2026-10-30-plugin-sdk-rust.md`) for the history and
   decisions, and `verification/README.md` for the checker.
3. Use the workspace toolchain (`rust-version` in `Cargo.toml`; export `RUSTUP_TOOLCHAIN`
   if your default is older) and a private `CARGO_BUILD_BUILD_DIR`. `make land` sets the
   build dir for you.
4. `make test-formal` must pass before you change anything (baseline), and after each
   item. Land each item on its own with `make land` once authorized.

## State on main (2026-11-01)

- Plugin system: Rust SDK (`bboxy450-term-plugin-sdk`) with a trait, adapter and example
  per contract. Rust GitKB plugin. PluginHost with live connections, seat tools, hooks with
  production vetoes, a host API (records, events, messages), operator grants, call/hook
  measurements with merged trace spans, and setup with a visible login seat. GitHub #57,
  #58 and #61 are closed.
- Models ship in the crates that own them (`model` feature, `models/*.tla`/`.cfg`):
  - PluginActivation, PluginConnection, PluginHookGate, PluginSetup and PluginHookQueue in
    `bboxy450-term-mux`;
  - PluginHook and PluginHostApi in `bboxy450-plugin`;
  - PluginRuntime in the SDK.
- Native checker constructs so far: IF-THEN-ELSE, bounded quantifiers over constant
  ranges, dynamic array reads, several initial states, weak and strong fairness, and `~>`.

Follow-up to task #377 (`.bboxy/tasks/2026-10-30-plugin-sdk-rust.md`). The plugin system,
the Rust plugin SDK and the shipped plugin models are on `main`. This task closes what #377
deferred. The operator directive stands: every model runs on our native checker
(`bboxy450-tlc`) and on Java TLC through `make test-formal`, with exact state and edge
parity. Each broken control must fail in both checkers.

## Scope, in priority order

1. **First-class finite sets, records and sequences** in `bboxy450-tlc`, including bounded
   sequences for queues. Today they are written as arrays of variables read through `Pick`
   and updated with `IF`.
2. **Quantifier ranges that depend on state.** Today `Forall`/`Exists` range only over
   constants.
3. **Temporal operators beyond `~>`**, for example `<>[]` and `[]<>`, plus action
   properties, following `verification/README.md` "Native replacement criteria".
4. **Imported upstream TLA+ examples and generated adversarial models**, with Java parity.
5. **Many-plugin plugin models:** upgrade PluginHook and PluginConnection (shipped in
   `bboxy450-plugin` and `bboxy450-term-mux` behind their `model` feature) from one plugin to
   several, using items 1–2. PluginHookQueue already covers two plugins.
6. **G8, only if a real plugin needs it:** multi-file plugins, non-Unix hosts, or a file name
   carrying more than one contract (`bbt-plugin/src/trust.rs`, `shape.rs` `PluginName`).

## Acceptance

- [ ] Each new construct has corpus entries (pass and Broken), Java TLC parity, and a broken control that fails in both checkers.
- [ ] The `verification/README.md` native-checker scope and the replacement criteria are updated.
- [ ] PluginHook and PluginConnection are checked for at least two plugins. Their shipped `.tla` files are regenerated, and the drift test stays green.
- [ ] `make test-formal` and `make land` pass. Prior model results are unchanged unless deliberately upgraded, with the change explained in the commit.

## Pointers and pitfalls

- New model families go in their own module; never edit the shared `families` array or the `bboxy450-term-mux` coverage line.
- Native constructs added under #377 (commit "Add IF-THEN-ELSE, bounded quantifiers…"): `Expr::Ite`, `Forall`/`Exists` with `Bound`, `Pick`, `Variable::choice`, `Model.strong_fair` (Emerson-Lei).
- Shipped models: `crates/bbt-plugin/models`, `crates/bbt-plugin-sdk/models` and `crates/bbt-mux/models`. `BBOXY_UPDATE_MODELS=1` rewrites shipped files; the drift test fails otherwise.
- Adding a field to the `Model` struct literal touches one line of `model()` in the shared `formal_contracts/models.rs`.
