# bboxy450-yarn-macros

## What it is

`bbt-yarn-macros` compiles the retained lean `reactor!` grammar. It validates
declared topology and emits direct biased polling over `bbt-yarn` source adapters.

## Model projection

`reactor_model!` accepts the same reactor grammar and shares its parser and
validator. It emits the `bbt_yarn::model` scheduling representation for independent
native Rust and TLA+/TLC checking. Enable Yarn's `model` feature at the call site.
Opaque Rust handlers are not translated. See the runtime crate's verification
guide and `tests/model_projection.rs` for shared-declaration runtime/model checks.

## Mental model

The **parser** builds a reactor from its policy, phases and arms. The **validator**
checks source identities, ordering, readiness declarations, drains and yield
relations. The **expander** emits arbitration and handler code against the runtime
crate, resolving a renamed dependency through `proc-macro-crate`.

The consumer guide and runnable example live in `bbt-yarn`; use its macro
re-export. Internal generated identifiers and the four comparison backends are
retained from upstream.

## Invariants

- Grammar, KTR/SRC codes and scheduling semantics match the pinned source.
- `forbid(unsafe_code)` applies to this compiler crate.
- Validation rejects declared contradictions; it cannot infer missing intent or
  check handler termination, external event order or raw-runtime bypasses.
- This crate depends on compiler utilities only, not on the runtime crate.

## Entry points

Use `bbt_yarn::reactor!` in applications. This crate exports `reactor!` for that
re-export; the hidden comparison macros are experimental implementation controls.

## Minimal example

Importing the compiler entry point compiles without depending on the runtime:

```rust
use bbt_yarn_macros::reactor;
```

An invocation also needs the `bbt-yarn` runtime. Its `examples/minimal.rs` is the
runnable invocation example and its integration suite exercises macro expansion.

## Recipes

A facade may re-export the macro without introducing a runtime dependency:

```rust
pub use bbt_yarn_macros::reactor;
```

For an application, depend on `bbt-yarn` with its default `macros` feature and
follow that crate's source declaration example.

## Pitfalls

Do not delete `#[shutdown]`, `#[last]` or other relations to silence a diagnostic;
that weakens the declaration. Waiver reasons are checked for non-emptiness, not
truth. Unit-test builds omit compiler ABI shims because compiler-owned token
streams are not available there; token-level unit tests and downstream trybuild
fixtures exercise the compiler and invocation boundaries separately.

## Platform support

The proc macro runs on the compiler host and requires `std`; the separate runtime
keeps its `no_std` core. Only the handoff's host was executed for this vendoring.

## Feature flags

No crate feature flags. The runtime's `macros` feature controls this dependency.

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

The Rust library name remains `bbt_yarn_macros`. Use the existing import name with a versioned package alias:

```toml
bbt-yarn-macros = { package = "bboxy450-yarn-macros", version = "0.1.0" }
```

## Respectful slopping

Project-owned contributions use [Respectful Slop License 1.0-Draft](LICENSE).
Retained upstream material keeps its original terms; earlier grants are not
withdrawn. [ATTRIBUTION.md](ATTRIBUTION.md) identifies the scope, source and reuse
examples; [NOTICE](NOTICE) preserves factual upstream notices.
Slop respectfully. Keep the credit attached.
