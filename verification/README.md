# Mandatory formal verification

Operator decision [365](../.bboxy/decisions/d365-mandatory-formal-verification.md)
makes TLA+/TLC and Yarn verification mandatory for every crate. `make test`
includes `make test-formal`. The hosted workflow also runs that target.

`make test-yarn-model` runs the native checker, existing reactor runtime and
compile-fail fixtures, the shared-declaration projection/runtime comparison, and
strict Clippy. `make test-formal` additionally bootstraps pinned tooling and
compares the native backend to Java TLC. Outputs live in the selected worktree's
`TARGET_DIR/formal`, never the application's runtime, operator home or a peer's
build directory. First use downloads public checksum-pinned TLC and Temurin
archives without system installation. Cached use is offline. Supported bootstrap
hosts are macOS/Linux on ARM64/x86-64. Upstream licenses remain in the runtime
archive; no tool binaries enter source or release packages.

## What is implemented

Yarn's opt-in `model` feature uses the dependency-free, pure Rust explicit-state
checker in `bboxy450-tlc`. The normal no_std reactor remains separate. `reactor_model!` uses the
same parser, validator and declaration grammar as `reactor!`; it projects source
priority, buffered yields, opaque guard snapshots, terminal arms and bounded
drains into a finite model. See `crates/bbt-yarn/tests/model_projection.rs` for
one declaration checked against actual executable selection for all seven
nonempty readiness masks. The projection assumes terminating handlers. It does
not translate arbitrary Rust, prove cancellation/waker correctness or turn an
arbitration-count bound into a wall-clock latency claim.

The native checker accepts an explicit expression/action representation: finite
integer domains, integer/Boolean literals, equality, comparisons, arithmetic,
Boolean operations, guarded simultaneous assignments, invariants, deadlocks,
response properties and named weak-fair action groups. It explores exact states
with BFS and analyzes strongly connected components for fair response violations,
including implicit stuttering. Counterexamples include a prefix and, for
liveness, a fair closed walk. State limits, overflow, invalid types/references and
out-of-domain states fail rather than truncate or silently prune the graph.
Native integers use checked i64 arithmetic; TLC compatibility outside the tested
numeric ranges is not claimed. This is not a general .tla parser or a complete
TLC replacement.

The TLA+ exporter translates expressions and simultaneous actions; it never
exports the graph discovered by Rust as the model. Java TLC independently
computes that graph. The gate compares exact reachable states and action-labeled edges,
then independently compares verification outcomes. Neither matching exit codes
alone nor matching state counts alone suffices. One TLC worker, a 256 MiB heap,
and a 60-second timeout per invocation bound the reference run. The native
corpus has a 100,000-state bound. Timeouts and tool/parser failures cannot satisfy
an expected negative control.

`corpus.toml` pins independent expected outcomes. The initial corpus covers
priority/draining/yields/guards, arithmetic, simultaneous updates, weak fairness,
fair stuttering, stale-generation checkpoint confirmation and uncertain input
retries. Broken safety, deadlock, starvation, checkpoint and duplicate-input
variants must actually fail. `coverage.toml` inventories every workspace crate,
its actual source path and its modeled scope or remaining implementation-only
coverage. Missing, duplicate and stale crate entries fail the gate. A contract
model is a design check, not a proof that its mapped Rust implements it.

## Changing a crate

For a change to a stateful boundary, update its finite model and add the smallest
counterexample to the previous behavior as a negative control. Keep assumptions,
atomicity, channel capacities, failures and fairness explicit. Model selected
small instances exhaustively; keep performance, OS effects and unmodeled byte
semantics in implementation/process tests. Run `make test-formal`, then the
normal exact-commit `make test` before handoff. New crates must be inventoried;
existing implementation-only entries are visible coverage gaps, not an exemption
from the mandatory verification stack.

`evidence.json` records the source revision/dirty state, input/model hashes,
reference artifact identities, actual reference exits, states/edges and per-crate
scope. Detailed logs, DOT graphs, native states/edges and counterexamples remain
beside it. No results, credentials or raw process logs are checked into Git.

## Native replacement criteria

Java remains the reference until a later explicit decision records conformance.
Expand the supported-language/semantic matrix, import relevant upstream examples,
add independently generated adversarial models, compare parser/evaluator errors
and resource limits, and validate safety, deadlock, stuttering, weak/strong
fairness and the supported temporal operators. Refinement/trace validation is
needed to connect broader models to real implementations. Corpus parity alone
never establishes full TLA+ or TLC completeness.

The language/algorithm concepts are inspired by [Leslie Lamport's TLA+ state
machine model](https://lamport.azurewebsites.net/tla/high-level-view.html).
[TLC](https://github.com/tlaplus/tlaplus) is the independent reference. No upstream
TLC implementation source was copied. Project source and credit policy:
https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md.
