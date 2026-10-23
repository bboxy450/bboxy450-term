---
id: 01M5MR5NCX1EZRXQMF2FDBC44Y
type: spec
title: "yarn — the reactor macro: declared event-loop topology, checked by the compiler"
status: decided
links:
  - { to: "[[2026-10-22-architecture-round-1]]", rel: decisions }
  - { to: "[[runtime]]", rel: constrains }
  - { to: "[[open-source]]", rel: spec }
  - { to: "[[layering]]", rel: invariant }
  - { to: "[[c1-live-seats]]", rel: adopts }
  - { to: "[[mux-daemon]]", rel: adopts-later }
created: 2026-10-23T16:05:44Z
updated: 2026-10-23T16:05:44Z
---
# yarn

Owner: Corvin Duskmantle <ARCHITECT>. Decided 2026-10-23 (decision #33; operator: "create this as bboxy450-crate, name it somehow sexy"). **Timing (operator): useful now or later; the design is documented first, the crate is vendored when a loop needs it or an Executor is free.** **What it is:** the operator's `reactor!` proc macro from `Gonzih/kittens-old` at `293e134526bb2deee96f1197d6268aee64495482` (2026-10-07), vendored into this monorepo as `bbt-yarn` (runtime, `no_std`, `forbid(unsafe_code)`) and `bbt-yarn-macros` (the compiler), published later as `bboxy450-yarn` and `bboxy450-yarn-macros`. The macro replaces a hand-written `tokio::select! { biased; … }` loop with a declared source topology that rustc checks before anything runs: shutdown in the leading prefix, `before`/`last` ordering, starvation protection (a may-remain-ready source above a protected one must yield to it or carry a reasoned waiver), bounded drains, all-guarded liveness, readiness declared per arm; every error is `KTRnnn <consequence>. Repair: <repair>.`

**Why we want it, in one line:** both scale bugs the gates found on 2026-10-22 were arbitration policy hidden in hand-written loops: the ring writer starving on per-file barriers and the message pump's 32-per-tick ceiling with a one-item batch. `yarn` makes that policy a declaration the compiler reads, and `#[starvation(allowed, reason = "…")]` is a decision record in code. It is the invariant model of this repo applied to event loops: undeclared intent compiles weaker, declared intent is a gate.

## 0. The design, recorded so it outlives its source

### What `reactor!` does

A declaration of persistent sources in priority order, each arm carrying relations, which the macro (1) parses into `Reactor { policy, phases, arms }`, (2) validates as a graph before anything runs, (3) expands into one `loop` that runs `initialize` once, `before_poll` each round, snapshots the `#[when]` guards once per arbitration, polls the sources in biased order (direct `poll_next`, or a generated `tokio::select!` in the retained comparison backends), runs the winning handler, optionally drains up to `N` immediate items, and runs `after_event` only after a service window in which every handler returned `Ok(Control::Continue)`; and (4) emits `if false { assert_… }` blocks so trait-level checks fire at the use site with a stable `KTRnnn` or `SRC001` name in rustc output. Every error reads `<CODE> <consequence>. Repair: <repair>.` The boundary is what is declared: remove `#[shutdown]` or `#[last]` and the macro cannot infer it; the program compiles weaker, and two compile-pass negative controls document exactly that.

One arbitration round: `initialize` once; then per round `before_poll`, snapshot the guards, poll sources in declared order, run the winning handler (a shutdown or terminal arm exits; `Control::Stop` exits; `Control::Continue` may drain up to `N` immediate items), and `after_event` only after a whole window of `Continue`.

### Grammar, the only implemented one

```text
yarn::reactor! {
    policy {
        selection: biased;
        required_phases: [initialize, before_poll, after_event];
    }
    initialize { ... Result<(), E> ... }
    before_poll { ... Result<(), E> ... }
    #[source(id)]
    #[readiness(quiescent | may_remain_ready)]
    [relations]
    item = persistent.place => { ... Result<Control<Exit>, E> ... }
    after_event { ... Result<(), E> ... }
}
```

| Relation | Meaning |
|---|---|
| `#[shutdown]` | terminal, unguarded, undrained, protected, in the leading lexical prefix; handler returns `Result<Exit, E>` |
| `#[terminal]` | a successful handler exits with `Result<Exit, E>` |
| `#[before(other)]` | freezes one source-order edge |
| `#[last]` | the source is globally final |
| `#[when(bool_expr)]` | snapshots one ordinary `bool` once per arbitration |
| `#[yields_to(input, when = buffered)]` | disables this higher source while the target adapter reports backlog, including between drained items |
| `#[drain(max = N)]` | handles the selected item plus immediate items, at most the unsuffixed literal `N` in `1..=4096`, no allocation |
| `#[starvation(allowed, reason = "…")]` | explicit waiver of default starvation protection; non-emptiness is checked, truth is not |

Rules of the loop: the source borrow ends before each handler; `after_event` runs once only after a whole service window of `Ok(Control::Continue)` and never after `Stop`, `Err` or panic; priority never preempts a running handler or phase. Sources are persistent path or field expressions in separate storage: a temporary is rejected (KTR015), two ids on one place are rejected (KTR020), a binding is `_` or one immutable identifier.

### What the validator checks

Duplicate ids; unknown references; scheduling cycles through `before`, shutdown and `last` edges, and buffered-yield cycles; lexical order against declared order; shutdown constraints (unguarded, undrained, no yield, not `last`, no waiver, leading prefix); a terminal source cannot drain; exactly one `last` and nothing after it; starvation (a may-remain-ready source above a protected source must yield to it or the victim carries a waiver); all-guarded dead reactors; drain limits; readiness declared on every arm; phases present exactly as `required_phases` says.

| Code | Fires when |
|---|---|
| KTR000 | grammar: missing `policy`, unknown phase, bad binding, attribute with arguments it does not take, unsupported attribute, zero arms |
| KTR001 | duplicate source id |
| KTR002 | relation references an unknown source |
| KTR003 | scheduling cycle |
| KTR004 | a source follows `last`, or two sources claim `last` |
| KTR005 | shutdown arm guarded, drained, `last`, waived, or outside the leading prefix |
| KTR006 | declared readiness does not match the adapter's `Readiness` type |
| KTR007 | a may-remain-ready source above a protected source can starve it |
| KTR008 | terminal source drained, or drain max malformed or out of `1..=4096` |
| KTR009 | `#[drain]` on a source without `DrainableSource` |
| KTR010 | yield target is self, lacks a `BacklogSource` probe, or forms a yield cycle |
| KTR011 | phase block present but not required, or required but missing |
| KTR012 | policy block malformed or missing `selection: biased;` |
| KTR013 | handler or phase result has the wrong shape |
| KTR014 | every arm is `#[when]`-guarded: an all-false snapshot polls nothing and registers no wake |
| KTR015 | source is a temporary expression |
| KTR016 | lexical order contradicts a declared edge, for example shutdown below a firehose |
| KTR017 | arm missing `#[source]` or `#[readiness]`, or an attribute declared twice |
| KTR018 | starvation waiver malformed or reason empty |
| KTR019 | `#[when]` expression is not `bool` |
| KTR020 | one persistent place under two source ids |
| SRC001 | source type does not implement the sealed `ReactorSource` |

### Runtime pieces the generated code consumes

`reactor::Control<T>` (`Continue` or `Stop(T)`); the sealed `source::ReactorSource` (`Item`, `Readiness`, `poll_next`), whose sealing is the admission gate and whose `#[diagnostic::on_unimplemented]` carries the repair text; the sealed readiness markers `MayRemainReady` and `Quiescent` with a blanket `HasReadiness<R>` keyed on the associated type; `DrainableSource` with `TryNext<T>`; `BacklogSource::has_backlog()`; the channel close policies `Dormant` and `Emit` with `ChannelEvent<T>`; the adapters `Latched<T>` and `FixedQueue<T, N>` in core and `Mpsc`, `OptionalMpsc`, `OneShot`, `OptionalOneShot`, `Cancellation`, `OptionalDeadline` behind the tokio feature; and the hidden `__private` anchor functions that give each KTR a stable name in rustc output. The runtime is `no_std` and `forbid(unsafe_code)`.

### Minimal example

```rust
struct Sources { stop: Latched<()>, events: FixedQueue<u8, 8> }

async fn run(sources: &mut Sources) -> Result<u8, Infallible> {
    yarn::reactor! {
        policy { selection: biased; required_phases: []; }

        #[source(stop)] #[readiness(quiescent)] #[shutdown]
        _ = sources.stop => { Ok(0) }

        #[source(events)] #[readiness(may_remain_ready)]
        #[starvation(allowed, reason = "the example has no lower interactive source")]
        #[drain(max = 4)] #[last]
        event = sources.events => {
            if event == 42 { Ok(Control::Stop(event)) } else { Ok(Control::Continue) }
        }
    }
}
```

### Reference

`Gonzih/kittens-old` at `293e134526bb2deee96f1197d6268aee64495482`: `crates/kittens-macros/src/lib.rs` (2290 lines: five proc-macro shims, parser, validator, expander, `ktr` diagnostics, unit tests), `crates/kittens/src/{lib.rs, reactor.rs, source/mod.rs, source/tokio_impl.rs}`, `crates/kittens/tests/ui/` (18 compile-fail fixtures), `tests/ui-pass/` (3 compile-pass controls), `docs/agent-guide.md`. The repo is public, archived and unlicensed; the operator is its author.

Lens: async loops on the bbt-io runtime. Assumes tokio sources behind the sealed `ReactorSource` adapters (`Mpsc`, `OneShot`, `Cancellation`, `OptionalDeadline`, `Latched`, `FixedQueue`). Breaks when the loop is not async: the supervisor's mio loop stays as it is, and logic's Worker loop (sync, `receive(timeout)`) is a later adoption, not a first one. LESS IS MORE: grammar, codes and docs vendored unchanged; only the crate path moves.

## 1. Placement and provenance

- Crates `crates/bbt-yarn` and `crates/bbt-yarn-macros`, no workspace dependencies, so they go **first** in the layering order: `bbt-yarn-macros bbt-yarn bbt-rt …` ([[layering]], same commit). `bbt-rt` may depend on `bbt-yarn` later; nothing below it ever will.
- Provenance recorded in the crate README: source repository and commit, the operator as author (the old identity wrote it; the record states that as a fact, never as an identity to adopt), and the license the operator sets at vendoring. The source has no license file; **MIT OR Apache-2.0** to match the workspace is proposed and is the operator's call, taken in the vendoring commit message.
- Public-crate docs standard ([[open-source]] §2) applies from the start: `#![deny(missing_docs)]`, the agent guide becomes the crate doc's Mental model and Pitfalls; the KTR code table ships in the README.
- Name considered and rejected: `litter` (a litter of kittens, but also trash), `ears` (readiness), `pounce` (selection), `purr`. `yarn`: the sources are strands, the reactor pulls one at a time in declared order, and the validator is a knot check.

## 2. Vendoring (Y1), what changes and what must not

- Paths `kittens` → `bbt_yarn` in the macro's generated code (`kittens_path`, `__private`), tests and fixtures; macro name stays `reactor!`; KTR and SRC codes stay verbatim so the agent guide and the trybuild fixtures carry over unchanged.
- The `tokio` feature on by default in this workspace (we only run on the bbt-io runtime); `no_std` core kept intact.
- Compiles under the repo's gates: edition 2024, `cargo clippy --workspace --all-targets -- -D warnings` ([[clippy-clean]]), the trybuild compile-fail fixtures (18) and compile-pass negative controls (3) run as tests.
- Nothing else: no grammar additions, no new adapters, no renames of relations.

## 3. Adoption, one loop at a time

| Step | Loop | Topology it declares |
|---|---|---|
| Y2 | the app IO task, `crates/bboxy450-term/src/live.rs:412` | `#[shutdown]` first; `UiMsg::Input` and `Resize` as quiescent protected sources above the `gui.sock` reply stream; the reply stream `may_remain_ready` with `#[drain(max = N)]` and `#[yields_to(input, when = buffered)]`, so operator keystrokes can never starve behind a flood of deltas ([[c1-live-seats]] §2 "producers never block, input is never dropped" becomes a compiler check) |
| Y3 | seat socket session and accept loops, `crates/bbt-mux/src/seat_socket.rs:109, 141, 344, 603` | shutdown, incoming lines, the 20 ms tick; drains bounded |
| Y4 | app child supervision, `crates/bboxy450-term/src/cockpit.rs:197, 523, 856` | child exit above everything; restart policy as declared order (decision #11) |
| later | logic's Worker loop and `pump_messages` (`crates/bbt-mux/src/logic.rs:3153`, `:2649`), after W1.2d and W2.6 settle | the supervisor event firehose yields to GUI and MCP commands when they are buffered; the pump is `#[drain(max = 32)]`; the biggest win and the busiest file, so last |
| never | the supervisor's mio loop; `bbt-rt::spawn_command` | not async arbitration, or a single bounded select that gains nothing |

Each adopted file gains the invariant **no raw `tokio::select!` beside a reactor**: a source-scan test in that crate (the shape of [[modules-through-components]]) fails on `select!` in files listed as adopted, so the macro's guarantees cannot be bypassed silently. The list starts empty and grows with Y2, Y3, Y4.

## 4. Acceptance, the spec signs off

1. Y1: both crates build and test in the workspace, 18 compile-fail fixtures fail with their named KTR codes, 3 negative controls compile, `clippy-clean` and `layering` hold with the new order, README carries source commit, author and license, `cargo package` dry-runs clean from each crate directory (open-source boundary rule).
2. Y2: `live.rs` has no `tokio::select!`; the reactor declares the topology in §3; the existing C1i and admission-retry tests pass unchanged; a new test floods deltas while typing and asserts input latency is bounded (the starvation relation, exercised).
3. Each later step: the adopted file's `select!` count is zero, its tests pass unchanged, and the source-scan list includes it.
4. `sh scripts/ci-local.sh` exit 0 in a clean worktree of the handed-off commit; compile-time delta of the macro crates reported in the handoff.

## 5. Out of scope

Grammar changes, new adapters, the supervisor, logic's Worker loop before its churn settles, publishing (O-track, at a plateau).
