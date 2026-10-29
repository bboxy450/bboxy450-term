# bboxy450-tlc

An experimental, dependency-free **pure Rust finite-state model checker**. It
checks safety, deadlocks and response properties under explicit weak fairness,
and exports TLA+ for independent comparison with upstream Java TLC.

Source: https://github.com/bboxy450/bboxy450-tlc

**This is an initial supported subset, not a complete TLA+ parser or TLC
replacement.** Java TLC remains the development reference. Native/reference
verification compares complete reachable state and action-labeled transition sets and property
outcomes, including deliberately broken models. No Java is required to use the
Rust library. No background service, network access or global runtime is started
by the library.

## Mental model

A `Model` contains finite integer variables, guarded atomic `Action`s, safety
`Invariant`s and temporal `Response`s. Every assignment reads the same pre-state.
`check` explores the exact reachable graph with BFS. Fair response checking uses
strongly connected components and explicit TLA stuttering. Every returned
counterexample has a prefix; a liveness violation also includes a fair closed
walk. State/resource limits, overflow and invalid expressions fail, never return
an incomplete green result.

```rust
use bboxy450_tlc::{Action, Expr, Invariant, Model, Variable};

let mut model = Model::new("Counter", vec![Variable::new("count", 0, 2, 0)]);
model.actions.push(Action::new(
    "increment",
    Expr::Var(0).lt(Expr::Int(2)),
    vec![(0, Expr::Var(0).plus(Expr::Int(1)))],
));
model.check_deadlock = false; // Reaching count=2 intentionally terminates.
model.invariants.push(Invariant {
    name: "bounded".into(),
    condition: Expr::Var(0).le(Expr::Int(2)),
});
let report = model.check(100)?;
assert_eq!(report.states.len(), 3);
assert!(report.violations.is_empty());
let tla = model.tla()?;
assert!(tla.contains("MODULE Counter"));
# Ok::<(), String>(())
```

## Entry points

- `Model::check(max_states)` returns a complete `Report`, or a named error.
- `Model::tla()` translates the expression/action representation to TLA+,
  independently of the graph discovered by the native checker.
- `Model::config(graph_only)` emits a TLC configuration. Graph-only mode disables
  semantic property/deadlock checks specifically to compare full negative-control
  graphs; normal verification must use `false`.
- Yarn's optional `model` feature re-exports this API and projects the same
  `reactor!` declaration through `reactor_model!`.

## Supported semantics and limits

The current subset includes finite signed-integer state vectors; integer/Boolean
literals; equality, integer comparison/add/subtract, Boolean operations; guarded
simultaneous assignments; invariants; explicit-action deadlocks; `request ~>
response`; and weak fairness over named action groups. An enabled self-loop does
not count as nonstuttering progress for fairness. The native graph stores exact
states, not fingerprints. Native arithmetic is checked i64; parity with TLC
outside the tested numeric ranges is not claimed.

Not yet implemented: reading general `.tla` source, modules/instantiation,
sets/functions/sequences, quantifiers, arbitrary temporal formulas, strong
fairness, symmetry/distributed checking, arbitrary Rust translation or a
refinement proof connecting an application to its model. Domain bounds are
checked as TypeOK, not used to prune inconvenient states. No elapsed-time,
filesystem, OS, cryptographic or GPU property follows from a finite model alone.
The library currently needs std and does not make a no_std claim.

## Verification and contribution

In the development workspace, `make test-formal` runs native tests/strict lint,
shared-declaration Yarn/runtime checks and pinned Java TLC side by side. The
public source mirror includes the native/reference corpus and its canonical
`make test` gate. Do not promote this backend to complete TLC on a small green
corpus. New semantic features need independent positive and broken controls,
exact graph parity, error behavior and documented scope before acceptance.

When contributing, preserve origin credit, add a minimal failing model for a
semantic fix, run the mandatory gate, and document assumptions and bounds.
Generated or reused source must retain license/credit beside the implementation.
Never ship credentials, raw private process traces or downloaded runtime binaries.

## License and respectful slopping

Copyright 2026 Boxy (bboxy450) and contributors. Project-owned code uses
[Respectful Slop License 1.0-Draft](LICENSE), a custom license (not an SPDX/OSI
approval claim). Keep attribution attached: accurately mark protected reuse as
“Copied from”, “Adapted from” or “Translated from”; independent conceptual
inspiration credit remains a request. See [ATTRIBUTION.md](ATTRIBUTION.md) and
[NOTICE](NOTICE). Retain upstream notices and prior grants where applicable.

The independently implemented checker uses state-machine and fairness concepts
from [Leslie Lamport's TLA+](https://lamport.azurewebsites.net/tla/high-level-view.html).
No upstream TLC implementation code was copied. Upstream TLC and Temurin retain
their own licenses; neither is shipped in this Rust crate.

Original integration source and policy:
https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md.
Slop respectfully. Keep the credit attached.
