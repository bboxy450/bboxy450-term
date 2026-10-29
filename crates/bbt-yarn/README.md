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

- `macros` (default): re-export `reactor!` and its retained comparison backends.
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
