# bboxy450-yarn

## What it is

`bbt-yarn` is a `no_std` kernel for persistent async sources and compile-checked
reactor topology. Its `reactor!` macro checks declared ordering, starvation
protection, bounded drains and phases before expanding an async loop.

## Mental model

A **source** owns an operation or buffered event across arbitration. A **reactor**
polls persistent sources in lexical priority order; an **arm** declares its
readiness and relations to other arms. A **service window** runs the selected
handler and an optional bounded drain; **phases** run at declared loop boundaries.

Use the lean expression-position grammar only:

```text
bbt_yarn::reactor! {
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

Canonical relations:

- `#[shutdown]`: terminal, unguarded, undrained, protected, and in the leading
  lexical prefix. Its handler returns `Result<Exit, E>`.
- `#[terminal]`: successful handler exits with `Result<Exit, E>`.
- `#[before(other)]`: freezes one source-order edge.
- `#[last]`: makes the source globally final.
- `#[when(bool_expr)]`: snapshots one ordinary bool once per arbitration.
- `#[yields_to(input, when = buffered)]`: disables the higher source while the
  target adapter reports backlog, including between drained items.
- `#[drain(max = N)]`: handles the selected item plus immediate items, at most
  the unsuffixed literal `N` (`1..=4096`), with no allocation.
- `#[starvation(allowed, reason = "...")]`: explicitly weakens the default
  starvation protection. The compiler checks non-emptiness, not truth.


## Invariants

- `no_std` and `forbid(unsafe_code)` remain active in the runtime crate.
- Source admission and readiness are sealed traits; adapters document their
  selection-loss, close and drop behavior.
- Validation checks declared relations. Undeclared intent is not inferred.
- A source borrow ends before its handler. `after_event` runs only after an
  entire service window returns `Ok(Control::Continue)`.
- Arbitration does not preempt a handler or phase and does not create a runtime.

## Entry points

1. Create persistent `source::Latched`, `source::FixedQueue` or Tokio adapters.
2. Declare `#[source]`, `#[readiness]` and relations in `reactor!`.
3. Return `reactor::Control::Continue` or `Stop(value)` from ordinary handlers;
   shutdown and terminal handlers return their exit value directly.
4. Repair diagnostics without deleting the declaration that carries the rule.

## Minimal example

```rust
use std::convert::Infallible;

use bbt_yarn::reactor::Control;
use bbt_yarn::source::{FixedQueue, Latched};

struct Sources {
    stop: Latched<()>,
    events: FixedQueue<u8, 8>,
}

async fn run(sources: &mut Sources) -> Result<u8, Infallible> {
    bbt_yarn::reactor! {
        policy {
            selection: biased;
            required_phases: [];
        }

        #[source(stop)]
        #[readiness(quiescent)]
        #[shutdown]
        _ = sources.stop => {
            Ok(0)
        }

        #[source(events)]
        #[readiness(may_remain_ready)]
        #[starvation(allowed, reason = "the example has no lower interactive source")]
        #[drain(max = 4)]
        #[last]
        event = sources.events => {
            if event == 42 {
                Ok(Control::Stop(event))
            } else {
                Ok(Control::Continue)
            }
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut sources = Sources {
        stop: Latched::new(),
        events: FixedQueue::new(),
    };
    sources.events.push(1).unwrap();
    sources.events.push(42).unwrap();
    assert_eq!(run(&mut sources).await, Ok(42));
}
```

The same executable is `examples/minimal.rs`:
`cargo run -p bboxy450-yarn --example minimal`.

## Recipes

Retain one locally installed shutdown event:

```rust
use bbt_yarn::source::{BacklogSource, Latched};
let mut stop = Latched::new();
stop.arm(7).unwrap();
assert!(stop.has_backlog());
assert_eq!(stop.disarm(), Some(7));
```

For a Tokio channel, use `source::mpsc(receiver, source::close::Emit)` to observe
one typed close event, or `close::Dormant` to become dormant silently. Put a
protected interactive source above a firehose, or declare a buffered yield to
it; a drain handles at most the declared number of items per service window.

## Pitfalls

Sources must be persistent path/field expressions in separate storage. Use a
reviewed yarn adapter. For a cancellation-awkward producer, give an ordinary
owned task/thread the operation and expose its output through an admitted mpsc
source. K0 has no hidden spawn, scope helper, or unchecked escape.

The source borrow ends before each handler. `after_event` runs once only after a
whole service window whose handlers all return `Ok(Control::Continue)`. It does
not run after `Stop`, `Err`, or panic. Priority never preempts a handler or
phase.

When repairing an error, preserve the declaration carrying the invariant.
Deleting `shutdown`/`last`, adding a waiver, moving work into an opaque helper,
or using raw selection may compile while weakening the architecture.

Ordinary Rust ownership and source admission are separate checks. The 18 upstream
compile-fail fixtures include the ordinary ownership error `E0382` and the
`SRC001` admission anchor as well as KTR diagnostics. The three compile-pass
negative controls demonstrate raw-runtime bypass, erased constraints and allowed
doc-comment rationale; they are not proofs of application-wide safety.

## Pure finite protocols

`protocol!` is an item macro; its module name is an ASCII identifier starting
with a letter, as required by TLA+ export. One declaration generates a module containing
`State`, `Event`, `Effect`, `RejectReason`, `IgnoreReason`, `Decision` and
`step(&State, Event) -> Decision<State, Effect>`. State fields are private;
read-only getters expose their finite values, and `Default` uses the declared
initial values. The reducer executes no effects.

```rust
bbt_yarn::protocol! {
    pub mod Flag {
        state { enabled: bool = false; }
        events { Set(value: bool); }
        effects { Changed(value: bool); }
        on Set(value) {
            case change when enabled != value => {
                set enabled = value;
                emit Changed(value);
            }
            case same when enabled == value => {
                preserve enabled;
                ignore Unchanged;
            }
        }
    }
}
let Flag::Decision::Accepted { next, effects } =
    Flag::step(&Flag::State::default(), Flag::Event::Set(true)) else { panic!() };
assert!(next.enabled());
assert_eq!(effects, [Flag::Effect::Changed(true)]);
```

Declare finite domains with `domain Id { A, B }`. State, event and effect types
are `bool`, a declared domain, `Option<Domain>` or `Set<Domain>`. Values include
`Id::A`, `Some(id)`, `None`, `Set::empty()` and state/event names. The expression
language is exactly `==`, `!=`, `&&`, `||`, `!`, parentheses, `.is_none()`, and
set `.contains(id)`, `.with(id)`, `.without(id)`. Sets are immutable values;
`with`/`without` return updated sets. Empty sets and `None` require a typed
context. Calls to arbitrary Rust functions, arithmetic, loops and other types
are compile errors with stable `PRO` diagnostics and repair hints.

Each event has exactly one `on Event(bindings)` block with named
`case name when <bool> => { ... }` cases. Event and effect variants use positional
values in generated Rust, such as `Event::Submit(Id::A)` and
`Effect::Fetch(Id::A)`. Each case must `set field = expression;` or
`preserve field, other;` for every field, exactly once. All assignment right-hand
sides **and emitted arguments read the pre-state**, regardless of statement
order. Assignments take effect simultaneously; effects retain declaration order.
`reject Reason;` and `ignore Reason;` require preserving all fields and prohibit
`set` and `emit`. Their decisions carry only a generated reason, so the caller
retains its input state and emits nothing.

State predicates use `#[invariant(id = "RESULT-001")] fn result_is_current() {
visible.is_none() || visible == desired }` inside the declaration. This generates
`result_is_current(&State) -> bool` and the identically named model property ID.
Duplicate IDs, missing field initializers, missing event arms, unaccounted fields
and overlapping/incomplete guard partitions are compile errors. Guard checking
enumerates every declared event-parameter value crossed with every value of
state fields read by any guard for that event, including short-circuit operands.
It does not assume invariants or restrict the universe to reachable states.
Diagnostics identify that universe. Domains have 1..=63 variants; universes over
65,536 valuations fail with a repair hint, never pass by partial exploration.

With Yarn's `model` feature, the same declaration also provides `model()`,
`State::model_values()` and `action_event(action_index)`. Domain values use
zero-based ordinals, `None` is -1, booleans are 0/1, and sets use one Boolean
variable per domain member. Model actions retain rejected/ignored self-loops.
`model().check(limit)` checks all reachable states or returns an error if the
limit is exceeded. Fairness and liveness are present only when explicitly declared.
The feature is selected on the Yarn dependency; consumers need no feature named
`model`. Ordinary reducers work with `--no-default-features --features macros`
and use `core` plus `alloc` for returned effect vectors.

[LatestResult](tests/protocol.rs) is the full submit/complete/cancel example:
21 reachable states satisfy RESULT-001, and all 105 edges replay against the
reducer. Removing the freshness fence yields `Submit(A), Submit(B), Complete(A)`;
replaying that native counterexample violates the generated invariant function.
The same declarations feed the existing TLA+ exporter/Java TLC parity path in
`make test-formal`, comparing all states, labeled edges and verdicts. Results
cover exactly the declared finite state/event universe and atomic pure steps.
**Effect execution, async adapters and domains larger than declared are
unmodeled.** This is not verification of arbitrary Rust or a complete TLA+ frontend.

Inspired by [Verus state_machine!](https://verus-lang.github.io/verus/state_machines/)
(explicit pre-state transitions) and [machine-check](https://github.com/onderjan/machine-check)
(a constrained Rust-facing language). Independently implemented; no source copied.

## Theorems in Lean 4

`#[invariant(id = "RESULT-001", theorem)]` marks an invariant that also owes a proof for
every reachable state. With the `model` feature the declaration generates `lean()`, the
machine for `bboxy450_tlc::lean`: a bool is a proposition, a set is a predicate on its
domain, an option is an `Option`, and each accepting case is a constructor of `Step`. A
domain whose members neither a transition nor a theorem names is an abstract type, so the
theorem holds for a domain of any size, not only the declared members. The theorem is
`<fn name>_statement`; prove it as `<fn name>_proved` in namespace `Yarn<Module>` and
export it with `lean().export(dir, proof, "pass")`, as `examples/yarn_lean.rs` does.
`make test-lean` checks it with the pinned Lean kernel and refuses `sorry`. Effects,
fairness and response properties are not exported, and the proof is about the declared
transitions: the reducer is generated from the same AST, which is tested, not proved.

## Temporal properties and effect lifecycle

State predicates can be named with `#[predicate] fn ready() { ... }`,
`#[terminal] fn done() { ... }` or `#[quiescent] fn idle() { ... }`.
All generate ordinary `fn(&State) -> bool` functions from the same finite
expression language. Invariant functions are also usable as predicates.
Declare a response as a standalone attribute (including its final semicolon):
`#[property(id = "VISIBLE-001", leads_to(ready, done))];`.
This lowers to `Model.responses` and TLA+ `ready ~> done`: every occurrence of
`ready` must eventually reach a state satisfying `done`. IDs are unique across
invariants and temporal properties; unknown predicate names fail compilation.
Terminal/quiescent labels describe state predicates; they do not prune the
graph, stop the reducer, or automatically satisfy a different response target.
Ignored/rejected self-loops and implicit stuttering preserve outstanding obligations.

Attach `#[fair(weak)]` or `#[fair(strong)]` immediately before a named `case`.
The equivalent standalone form is `#[fair(weak, case = Present::current)];`.
Unknown cases and duplicate annotations fail compilation. Each annotation
creates one fair group containing the disjunction of that case's concrete
parameter instances. Weak fairness requires a continuously enabled
**nonstuttering** action group eventually to run; strong fairness also covers
groups enabled infinitely often. This is case fairness, not per-parameter,
per-event, per-effect or global runtime fairness. A fair ignored/rejected case
cannot make a false response predicate true. Both native checking and the TLA+
export use the existing `WF_vars` / `SF_vars` semantics.

Declare effect outcomes with, for example,
`effects { Fetch(id: Id) completes { Succeeded, FailedBeforeEffect, UnknownOutcome, CancelledBeforeStart }; }`.
It generates `FetchAdmitted(id)`, `FetchSucceeded(id)`,
`FetchFailedBeforeEffect(id)`, `FetchUnknownOutcome(id)` and
`FetchCancelledBeforeStart(id)` event variants carrying the intent's declared
parameters. Every generated variant requires an exhaustive `on` block, just
like an explicitly declared event. Outcome names are scoped by the effect's
prefix; `Admitted` is reserved. Legacy effects without `completes` keep their
existing intent-only API.

`emit Fetch(id)` returns an intent, never executes it, admits it, or recursively
processes a completion. The caller supplies admission and completion as later
`step` inputs. Lifecycle state and guards are explicit in the protocol:
accept admission only for queued work, completion only for admitted work,
and cancellation-before-start only before admission. Revalidate the current
generation in the completion/commit guard. The language does not invent a
hidden queue or enforce guards that the declaration omitted. The finite
witnesses check those guards against delayed, reordered, duplicate and stale
inputs. An `UnknownOutcome` may already have performed the external effect;
a retry needs a retained claim or an independently verified idempotent backend.
No effect backend or persistent claim store is supplied by this macro.

[protocol_temporal.rs](tests/protocol_temporal.rs) uses one frame and two
nonreused generations. The live property is ready leads_to visible-or-deselected,
for each generation: **if a successfully fetched request remains selected**,
weak fairness on `Present::current` ensures it eventually becomes visible.
Strong fairness is separately declared on `FetchAdmitted::admit`; successful
fetch completion is not assumed. The correct declaration has exactly 91 states
and 1,365 labeled edges. Removing its generation comparison violates safety
(99/1,485); removing presentation fairness retains 91/1,365 but yields closed
lassos. A one-state/two-edge fair-stuttering control remains quiescent forever
and fails its terminal-target response. The unknown-outcome witness retains
an admission claim (6 states/42 edges); allowing unknown retries past that
claim duplicates a commit (16/112). All edges and counterexamples replay the
generated reducer, and the same declarations enter Java TLC graph/verdict parity.
These results assume atomic pure steps at exactly those finite bounds; they do
not prove external effect execution, arbitrary Rust, wall-clock progress,
restart persistence or general TLC completeness.

## Required claims and structured verification

With `model` and `macros`, make the project contract explicit:

```text
bbt_yarn::requirements! {
    pub ResultContract {
        "RESULT-001" => LatestResult::result_is_current;
    }
}
bbt_yarn::verify! {
    #[test]
    pub fn latest_result_contract() {
        contract: ResultContract;
        protocol: LatestResult;
        exploration: exhaustive;
        require_reachable: [Submit::fresh, Complete::current];
        limits { states: 100_000; on_exhaustion: inconclusive; }
    }
}
```

Required ids must match all protocol invariants and temporal property ids
exactly. A missing claim, an unrequired extra claim, a wrong predicate, or an
unknown case is invalid input. A required case must fire on an actual edge in
the complete reachable graph, including rejected/ignored self-loops. A state
limit produces `inconclusive` and fails the test; partial exploration never
establishes a claim. Only exhaustive exploration and inconclusive exhaustion
are accepted syntax. Qualified `super::` and `crate::` paths retain their scope.

Every generated test writes `target/yarn-verification/<test>.json` before its
verdict assertion, including failed runs. An explicit Cargo target directory
is honored. `verify::VerificationResult::to_json()` separates `outcome`,
`method`, `scope`, `implementation`, `provenance` and `counterexample`.
Scope includes domain members/sizes, encoded bounds, state limit, explicit
fairness assumptions and unmodeled effects/environment. A refuted result
includes event names/parameters and the failed condition; temporal traces also
retain their cycle. Counts are absent when exploration is incomplete.

`generated_core` means the reducer and model share the protocol AST. It does
not claim independent implementation replay. The separate LatestResult edge
witness checks all 21 states/105 edges. Declaration locations and deterministic
FNV-1a hashes identify canonical tokens (comments/whitespace excluded); these
are not cryptographic signatures. Repository signed commits and SHA-256 formal
receipts provide separate custody. Tool versions are recorded. No serialization
or hashing dependency is added to the no_std runtime.

`protocol!` exposes `verification_metadata()` and `action_case(index)` with the
`model` feature. A verification declaration additionally exposes
`latest_result_contract::check()` for tooling, returning the same structured
result without asserting or writing it. The repository's
`protocol_contracts` example consumes that generated configuration directly.

`make check-protocol-contracts` compiles the current configuration and checks
`verification/protocol-contracts.toml`. It prints the contract diff and rejects
removed required ids/cases, shrinking domains/bounds/limits, added assumptions
or fairness, and new waivers. Weakening requires a matching baseline amendment
in the same commit as its source change; a later baseline-only amendment does
not retroactively approve it. Non-`test` attributes, including `ignore` and
`should_panic`, are recorded as potential waivers. The baseline is reviewable
policy, not proof that its assumptions describe reality. The RESULT-001-drop
fixture must fail. This check and a JSON golden comparison run in
`make test-formal`. The committed golden normalizes only expansion locations;
real results retain them. No fairness or larger-domain claim is inferred.

Inspired by [Quint](https://quint.sh/docs/checking-properties) (explicit finite verification scope)
and [TLA-Verus](https://arxiv.org/abs/2610.14923) (statement
preservation). Independently implemented; no source copied. Keep the project
credit and [reuse policy](https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md)
with extracts.

## Generated component cards

With `model`, `card::generate` checks a model and produces deterministic Markdown
from `ProtocolMetadata`, ordered named predicates over encoded states, intent and
recorded assumptions. The first matching predicate names each state's node;
unmatched states, more than twelve reachable nodes, violations and incomplete
exploration fail. Edges come from the checked graph, merge duplicates and omit
self-loops. The card records finite domains, liveness/fairness scope, all declared
cases (including reject/ignore), unsupported waivers and the existing source-token
hash. No diagram or second case-outcome parser is maintained by hand.

## Native verification and TLA+ export

Enable the optional `model` feature for the hosted, pure Rust explicit-state
checker in `model`. `reactor_model!` projects the same validated grammar into a
finite scheduling model. Use a shared declaration for executable/model variants;
`tests/model_projection.rs` demonstrates the link against real source selection.
Arbitrary handlers and OS/runtime effects remain outside this projection.

The native backend explores exact states, checks invariants/deadlocks and
analyzes response properties under explicit weak fairness, including stuttering.
The separate TLA+ exporter feeds upstream TLC. `make test-formal` compares full
state/edge sets and property outcomes on positive and broken controls. Java is a
development reference, never an application/runtime dependency. This is the
initial finite expression/action subset, not a complete TLA+ parser or TLC
replacement. See the repository's `verification/README.md` and operator decision
365 for mandatory coverage and promotion rules.

## Platform support

The core adapters use `core` only. Tokio adapters are enabled on hosted targets
(`target_os != "none"`) and use allocation; the reactor runs inside the caller's
runtime. Vendoring is verified on the host named in the handoff; it does not
claim execution on other platforms or bare-metal targets.

## Feature flags

- `macros` (default): re-export `protocol!`, `reactor!` and its retained comparison backends.
- `tokio` (default): Tokio channel, one-shot, cancellation and deadline adapters.
- `model` (opt-in): hosted native finite-state checking and TLA+ export; requires std.
- `--no-default-features`: the `no_std` core without Tokio or the proc macro.
- `--no-default-features --features macros`: core adapters plus macro expansion.

## Provenance and license

Vendored from [Gonzih/kittens-old](https://github.com/Gonzih/kittens-old) at exactly
`293e134526bb2deee96f1197d6268aee64495482` (2026-10-07). The original author is the
operator, recorded in that commit as Maksim Soltan <gonzih@gmail.com>, under the
old handle Gonzih. This is historical attribution; current maintenance and Git
identity are Boxy <bboxy450@gmail.com> (bboxy450).

The pinned source contains `LICENSE-MIT`, `LICENSE-APACHE`, and the workspace
license expression `MIT OR Apache-2.0`.
The pinned pre-adoption code remains `MIT OR Apache-2.0`, as confirmed by the operator for
vendoring on 2026-10-23 (decision #33). `LICENSE-MIT` and `LICENSE-APACHE` retain
the pinned upstream texts and copyright, including `Copyright (c) 2026 Kittens
contributors`. No publication is part of this change.

Vendoring changes crate paths to `bbt_yarn` / `bbt_yarn_macros`, makes the crate
documentation self-contained, and adds the workspace's documentation gates.
The grammar, scheduling semantics, KTR/SRC diagnostic identifiers and generated
internal names are retained. No existing application loop adopts the macro.

# Diagnostic index

K0 diagnostic identifiers are provisional but semantic anchors are tested.

| Anchor | Meaning | Canonical repair |
|---|---|---|
| `KTR001` | duplicate source ID | rename one source and update relations |
| `KTR003` | `before`/shutdown/last cycle | remove or reverse the conflicting relation |
| `KTR004` | conflicting or non-final `last` | move one complete last arm to the end |
| `KTR005` | invalid shutdown contract | make it unguarded, undrained, protected, and leading |
| `KTR006` | declared readiness differs from adapter | state the adapter's conservative marker |
| `KTR007` | firehose can starve protected source | reorder, or add a direct buffered yield |
| `KTR008` | invalid or terminal drain | use a supported positive literal or remove it |
| `KTR009` | source is not drainable | use a stable drainable adapter or remove drain |
| `KTR010` | invalid yield/capability/cycle | use an observational backlog target and acyclic edge |
| `KTR011` | phase list/block mismatch | restore the block or deliberately remove both |
| `KTR014` | every arm carries `#[when]`; an all-false snapshot polls no source and registers no source wake | keep an unguarded wake-capable control source or encode enablement in a wake-registering adapter; a permanently dormant source is insufficient |
| `KTR015` | temporary source expression | construct a persistent adapter or isolate producer |
| `KTR016` | lexical order violates relation | move the complete arm |
| `KTR019` | guard is not bool | return one synchronous bool |
| `KTR020` | same exact place has two IDs | keep one source identity |
| `SRC001` helper | source is not admitted | retained/latching adapter or owned signal/channel isolation |

Compile-fail fixtures under `crates/bbt-yarn/tests/ui` are the executable repair
reference. Compile-pass negative controls live beside them under `ui-pass`.

## Dependency name

The Rust library name remains `bbt_yarn`. Use the existing import name with a versioned package alias:

```toml
bbt-yarn = { package = "bboxy450-yarn", version = "0.1.0" }
```

## Respectful slopping

Project-owned contributions use [Respectful Slop License 1.0-Draft](LICENSE).
Retained upstream material keeps its original terms; earlier grants are not
withdrawn. [ATTRIBUTION.md](ATTRIBUTION.md) identifies the scope, source and reuse
examples; [NOTICE](NOTICE) preserves factual upstream notices.
Slop respectfully. Keep the credit attached.
