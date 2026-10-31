# Mandatory formal verification

Operator decision [365](../.bboxy/decisions/d365-mandatory-formal-verification.md)
makes TLA+/TLC and Yarn verification mandatory for every crate. `make test`
includes `make test-formal`. Scoped presubmit/rechecks and the hosted workflow
also run that target; changed model/tool inputs invalidate reuse.

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

## Project contracts

The project corpus adds 33 contract families and 33 broken controls to the
14 checker/Yarn conformance cases: **80 models** in total. The declarations live
in `crates/bboxy450-term/tests/formal_contracts/models.rs`, as a test-only
integration consumer of the published native checker. No Java or model checker
is added to the terminal runtime.

| Boundary | Property checked |
| --- | --- |
| Landing verification | A clean current commit, inputs, complete checks and current remote base authorize publication |
| Immediate dirty wake | Rechecking after clearing pending prevents a lost wake |
| Notification replacement | A queued replacement wins after slow authorization |
| Notification recovery | Enabled policy checks permission before a question; finite IO failures retry and cancellation eventually withdraws |
| Current notification lifecycle | UI detach retains desktop/current episode; old context and captured clicks cannot target a newer A → B → A incarnation |
| Original-operation recovery | Fresh lookup preserves unknown outcomes, never redispatches and confines results to the original seat/root |
| Seat error diagnostics | Current authority/workspace and own-row filters fence pages; seat reads never acknowledge the operator |
| Workspace reads | Root, revision and authority still match at publication, including A → B → A |
| Plugin inspection | Source digest, approval and activation generation still match |
| Harness readiness | Input ownership and single-use proof remain current; unknown outcomes fence retries |
| Local messaging custody | Delivery progresses without assuming signing custody recovers |
| Message suppression | Events, muted recipients and resolved/superseded receipts never wake |
| Retained process ownership | UI detach preserves owners; explicit stop checks process birth |
| Window persistence | Only the matching durable version releases a fenced reply |
| Focus receipt | A completed frame has the current visible, focused process generation |
| Terminal query replies | Live terminal queries progress through bounded delivery; historical apply/restore cannot emit PTY writes. Saturation and pending-reply owner restart remain explicit implementation gaps. |
| Terminal checkpoint | Restore excludes queued output effects |
| Live-only bus subscription | Anonymous new subscriptions bypass retained storage; replay and durable policies retain IO |
| Seat ingestion | The watched quota permits background progress and pauses stay local |
| Terminal raster publication | Worker-submitted pixels stay immutable across source replacement, including A → B → A |
| Render arena | Live slices are disjoint, uploads precede submit, rewind follows its boundary |
| Read-only selection | Replacing displayed source retires its captured range |
| Sidebar accordions | Independent fold choices preserve focus and survive save/restore |
| Sidebar reveal | Retarget/tick cannot retain frame demand after settlement; clock progress and continuous geometry use private witnesses |
| Render line plans | Font and mask identities remain current; repacked coordinates are resolved later |
| Async renderer setup | Dropping the UI waiter retires a late worker result |
| Import staging | Internal staging spends no record numbers; a fair bounded retry eventually publishes after ordinary contention |
| Historical reconciliation | Operator-root authority archives retired records without replacing accepted verification |

`contracts.toml` records source and implementation-test paths, assumptions and
properties for each family. The gate refuses missing pairs, unmapped contracts
and missing evidence paths. Generated per-model metadata records variable names,
initial values and bounds, action names, invariants, response properties and weak
fairness groups in `evidence.json`. Source and witness hashes invalidate stale
verification receipts. Broken variants must produce the prescribed safety or
liveness violation in **both** engines, with identical full graphs.

`make test-formal-contracts` runs all native project models plus five unattended
real-API witnesses: exhaustive short dirty-bit histories, notification replacement
across an authorization barrier, stale GUI save completion, partial-parser
checkpoint restoration, and superseded focus timeout correlation. `make test`
also runs the mapped existing guard, workspace, plugin, messaging, lifecycle,
ring-ingestion and GPU tests. Witnesses exercise selected implementation behavior;
there is no mechanical refinement proof from arbitrary Rust to these models.

Platform key storage/notification internals, byte parsers, document/diagram layout,
glyph rendering, reference-gallery pixels and load-generator throughput retain
implementation, property, golden, process or performance tests. Their inventory
entries state that scope. Adding a synthetic state machine to a pure layout
algorithm would not establish its correctness. New asynchronous document or UI
protocols still need their own model when introduced.

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


## Formal evidence during landing

Every successful formal run emits a `formal: receipt` JSON line containing the
source commit, evidence-file SHA-256, checkout, corpus size and crate count.
A content-addressed archive under the repository’s private Git state retains
evidence and generated models after checking-slot cleanup. Landing validates the actual evidence file against current source hashes,
coverage and contract inventories, required model verdicts, pinned reference
tools, TLA hashes and bounds metadata. Full-gate reuse requires exactly one
receipt; scoped presubmit records it too. The landing record retains it, and
checks it again immediately before pushing. A disjoint rebase may reuse Rust
checks through the existing scope mechanism, but reruns formal verification for
the new source commit. No cached receipt from an old commit authorizes publication.

Use `make check-formal-evidence` to validate existing evidence without rerunning
the model checker. `make push` also runs this check. The evidence hash records a
script-produced result; it is not a proof of arbitrary Rust refinement or a
replacement for signed commits, actual test execution and review.
