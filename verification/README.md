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

`protocol!` adds a closed application-state language. One declaration generates
private finite state, events/effects, a pure simultaneous reducer, invariant
functions and symbolic model expressions. `tests/protocol.rs` supplies the exact
LatestResult declarations to both reducer tests and the Yarn exporter: 21 states,
105 enabled edges, and a freshness-broken control replayed as
`Submit(A), Submit(B), Complete(A)`. Every reachable state and edge is compared
with generated Rust, including rejected/ignored self-loops. ProtocolLanguage
adds exhaustive language-value/edge witnesses. These models enter the same Java
TLC graph/verdict gate as the scheduling corpus. Effect execution, async adapters
and domains larger than declared remain outside the checked scope. Guard
completeness enumerates all declared parameter and guard-read state values,
without invariant or reachability pruning, and refuses universes over 65,536.
See `crates/bbt-yarn/README.md` for the grammar and limits.

### What protocol liveness assumes

`crates/bbt-yarn/tests/protocol_temporal.rs` supplies the temporal/effect
contracts to both runtime witnesses and the existing exporter. Named state
predicates feed `#[property(id = "...", leads_to(P, Q))];`; named cases alone
carry `#[fair(weak)]` / `#[fair(strong)]`. A case's concrete parameter instances
form one disjunction, not independently fair instances. Only nonstuttering
transitions count toward fairness. Terminal/quiescent predicates are explicit
Boolean labels, never permission to drop a pending response or prune the graph.
The one-state BrokenProtocolProgress control has both WF and SF annotations but
still violates its quiescent-to-terminal response through ignore/reject loops.

PresentGeneration has Frame={F}, Generation={First,Second}, one owner and atomic
steps. Issued generations cannot be reused. Emission, admission, completion and
presentation occur in separate steps; all generated completion outcomes require
exhaustive handling. Each successfully fetched generation must eventually be
visible **or cease to be selected**. Thus continued selection implies eventual
visibility under weak fairness of `Present::current`; strong fairness of
`FetchAdmitted::admit` is separately explicit. No assumption forces successful
fetches, every event's delivery, a globally fair runtime or a wall-clock bound.
The generation guard is rechecked at presentation. The correct 91-state/1,365-edge
model passes; removing generation revalidation refutes safety (99/1,485), and
removing only presentation fairness refutes liveness with closed lassos at the
original graph bounds. RetryUnknown (6/42) retains its admission claim across an
uncertain possible commit; relaxing only the unknown retry guard produces a
duplicate commit (16/112). Known pre-effect failure may release the claim.

Every explicit edge in all eight declarations is replayed through generated
`step`; the three requested broken controls replay their actual counterexamples.
The exporter emits expressions, not precomputed transition tables; Java TLC
independently compares full states, labeled transitions and verdicts. Real
external effects, persistent dedup services, process restart and larger identity
domains remain implementation-only gaps. See the PresentGeneration and
RetryUnknown entries in `contracts.toml` for bounds and assumptions.

The native checker accepts an explicit expression/action representation: finite
integer domains, integer/Boolean literals, equality, comparisons, arithmetic,
Boolean operations, `IF-THEN-ELSE`, bounded `\A`/`\E` over integer ranges
(`Forall`/`Exists` with de Bruijn `Bound`), arrays of consecutive variables
read by a dynamic index (`Pick`, a `CASE` in TLA+ that fails outside its
domain, as TLC does), several initial values per variable
(`Variable::choice`, `v \in {…}`), guarded simultaneous assignments,
invariants, deadlocks, response properties and named weak-fair and
strong-fair (`SF_vars`) action groups. Strong fairness is checked by
refining a candidate component: an SF group enabled in it but never taken
inside leaves only the sub-components that avoid its enabled states. Each of
these constructs has corpus entries with Java TLC parity and a broken
control (`IteMax`, `QuantifiedScan`, `StrongFairFlicker`). It explores exact states
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

Each project contract pairs a model family with its broken control. The checked
totals come from `contracts.toml` and `corpus.toml`. The declarations live
in `crates/bboxy450-term/tests/formal_contracts/models.rs`, as a test-only
integration consumer of the published native checker. No Java or model checker
is added to the terminal runtime.

Plugin-system models ship with the crates that own each contract, behind each
crate's `model` feature and built with `bboxy450_tlc::build`: PluginHook and
PluginHostApi in `bboxy450-plugin`, PluginRuntime (the SDK's own admission,
cancellation and host-callback contract, for plugin authors) in
`bboxy450-term-plugin-sdk`, and PluginActivation, PluginConnection,
PluginHookGate and PluginSetup in `bboxy450-term-mux`. Their exported
`.tla`/`.cfg` files, broken controls included, live in `crates/<crate>/models/`
and are part of each package (`cargo package --list`). `formal_contracts/plugin_models.rs`
re-exports them into the gate; `make test-formal` fails when a shipped file
differs from its Rust builder (`BBOXY_UPDATE_MODELS=1 cargo test -p
bboxy450-term --test formal_contracts shipped` rewrites them).

| Boundary | Property checked |
| --- | --- |
| Landing verification | A clean current commit, inputs, complete checks and current remote base authorize publication |
| Crate publication | Verified identity and indexed dependencies precede publication; finite rate limits retry to completion |
| Immediate dirty wake | Rechecking after clearing pending prevents a lost wake |
| Notification replacement | A queued replacement wins after slow authorization |
| Notification recovery | Enabled policy checks permission before a question; finite IO failures retry and cancellation eventually withdraws |
| Current notification lifecycle | UI detach retains desktop/current episode; old context and captured clicks cannot target a newer A → B → A incarnation |
| Original-operation recovery | Fresh lookup preserves unknown outcomes, never redispatches and confines results to the original seat/root |
| Seat error diagnostics | Current authority/workspace and own-row filters fence pages; seat reads never acknowledge the operator |
| Private error journals | Explicit test accounts and private backend bootstraps own diagnostics before inherited run selection; provider and ctl failures stay private |
| Workspace reads | Root, revision and authority still match at publication, including A → B → A |
| Plugin inspection | Source digest, approval and activation generation still match |
| Plugin connections | A call result is delivered only from the current activation generation; disable closes the connection |
| Harness readiness | Input ownership and single-use proof remain current; unknown outcomes fence retries |
| Supervisor input receipts | Oldest terminal receipts retire within a bounded retry window; pending writes survive and fresh inputs progress beyond capacity; removing eviction fails liveness |
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
| Native surface resize | Configure precedes worker sharing, Metal layer resize avoids global GPU idleness, and acquire rejects lagging drawables |
| Native frame observation | Private source/composition/drawable copies are read only after submission and retain their frame identity while the UI advances; real pixels, layer sizes and injected strips use the native night witness |
| Render arena | Live slices are disjoint, uploads precede submit, rewind follows its boundary |
| Read-only selection | Replacing displayed source retires its captured range |
| Sidebar accordions | Independent fold choices preserve focus and survive save/restore |
| Sidebar reveal | Retarget/tick cannot retain frame demand after settlement; clock progress and continuous geometry use private witnesses |
| Render line plans | Font and mask identities remain current; repacked coordinates are resolved later |
| Async renderer setup | Dropping the UI waiter retires a late worker result |
| Import staging | Internal staging spends no record numbers; a fair bounded retry eventually publishes after ordinary contention |
| Live task amendments | Exact delegator, nonterminal state and current etag permit brief changes; protected verify revisions force fresh reports after A → B → A |
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
the model checker. `make land-full` also runs this check. The evidence hash records a
script-produced result; it is not a proof of arbitrary Rust refinement or a
replacement for signed commits, actual test execution and review.

### d5002 presubmit work and verdict reuse

Presubmit still compiles and exports every model. It requests
`make test-formal FORMAL_REUSE=1`; full ci-local explicitly forces
`FORMAL_REUSE=0`, including when a caller inherited a reuse override.
The formal driver may reuse a model only when its TLA/configuration bytes,
fresh native graph bytes, independent expectation, contract metadata and full
tool-pin map and checker_sha256 match a complete green archived model.
The checker digest covers tracked working-tree bytes of scripts/formal.py,
scripts/test-formal.py, crates/bboxy450-tlc and crates/bbt-yarn; any checker
change invalidates every model verdict. A per-key atomic pointer
under the private Git evidence store avoids a global archive scan. Missing,
corrupt, legacy or mismatched entries re-check; an index write failure does
not invalidate newly checked archived evidence. Each reused row names its
`reused_from` evidence hash. The new receipt always binds the current candidate,
current complete inventory and current source hashes. This memoizes the same
deterministic NativeAndTlcAgree evidence predicate in LandingVerification;
it adds no publication permission or assumption that changed models still pass.

`formal-stage.py` frames every Make formal sub-step. The existing presubmit
two-clock observer excludes compilation from `stage_seconds` and records
`formal_step_seconds`, start/end one-minute load and advisory findings. The
allocations are formal290/tests400/fixtures60/invariants_misc60 plus90 margin.
Missing phase markers above15s and stage overruns are findings; the unchanged
900s run cap still blocks. Landing fixtures run only for the d5002 machinery
scope; full ci-local still runs them and resource-ledger fixtures every time.
Quarantine receipts discharge only exact observed passing invariant tests in
the same attempt/source/target. Filtered, ignored, ambiguous and differently
configured checks still run. `test-landing-budget-controls.py` removes these
guards individually and requires named red then restored green results.


`SeatIngestion` projects the #47 independent read/parse boundary: two seats,
two records each, one pending slot per seat, fair healthy-seat admission/read/
publication, and an indefinitely held peer/history read. `BrokenSeatIngestion`
restores a shared head queue and must fail healthy-seat liveness. Native socket
reordered completion, held seat/history, exact 967-seat serial replay and stale
owner fixtures map it to implementation. Neither this projection nor corpus
parity proves timing deadlines, Rust refinement or disk-writer independence.

## Pinned harness integration

`crates/bboxy450-harness` is an independent Git submodule. Clone with
`--recurse-submodules`; `make init-submodules` and both CI/landing entry points
initialize exactly the recorded gitlink without prompts or following a branch.
Missing or unapproved submodules fail loudly.

The same initializer checks Cargo's resolved parent feature graph. Neither
`bboxy450-harness/acp` nor `serde_json/preserve_order` may be enabled there;
the real ACP-enabled metadata graph is a rejecting control. Parent workspace
build, test, Clippy and formal commands use default harness features. ACP is
tested only by the submodule's own `make test`, which explicitly canonicalizes
schema and golden JSON without changing the parent's map-ordering contract.

`make test-formal` imports every native harness model and broken-control export and their independent
expected verdicts into this repository's Java TLC graph/verdict comparison.
The harness's own coverage and multi-control contracts validate its complete
source inventory; parent coverage lists every imported model. The receipt hashes
the imported implementation, tests, tools, inventories and workspace resolution.
`HarnessEmbedLifecycle` also maps to the real private MCP seat-spawn witness in
`make test-harness-native`; it is not duplicated. Terminal bytes, elapsed close
time and `kill(-pgid, 0) == ESRCH` remain real process assertions.

The upstream descriptor-inheritance fixture intentionally clears CLOEXEC. Full
CI runs it separately in `make test-harness-native`, after the workspace suites,
so concurrent forking tests cannot inherit its exposed descriptor. Every case
still runs, with the normal test-thread configuration and the original assertions.

`make test-harness-standalone` runs the unchanged child `make test` with a private
Cargo target. It retains the child's generated evidence under the parent target
archives generated formal evidence and restores only four upstream test-owned run artifacts, preserving source.

R5 full-log reuse compares C and its rebased candidate with Git's complete tree
diff (including names, modes and gitlinks with submodule ignoring disabled)
excluding only `.bboxy`. Changes under `.bboxy/invariants/` always refuse reuse:
invariants.sh discovers these records by glob and executes their check fields.
Every other changed record is checked for literal code references, including
removed paths from renames.
Records checks run in the existing private checking checkout, so untracked
records in the author's tree cannot satisfy them. Only a signed candidate with
passing records gates can rebind unchanged formal
inputs to its current SHA. The archive keeps the original evidence digest and
records provenance; the default receipt validator remains exact-SHA. This
equivalent-input predicate supplies the existing LandingVerification checks,
with eligibility covered by real Git fixtures and deliberately broken guards.

R2 reads the literal FIXTURE_SCRIPTS tuple used by test-landing.py's copy loop,
without importing or executing the fixture module. Fixed records/invariant CI
entries and resource-ledger files complete the trigger; adding a copied script
requires no separate trigger edit. Canonical ci-local runs the mutation controls
in a disposable checkout of its committed HEAD, restoring and removing it before
continuing, so the author checkout stays intact.

The archive in git-common-dir is shared across all local branches and is unsigned.
Content hashes detect mismatched inputs or accidental corruption; they do not
authenticate the writer or prove that an archive was executed. Presubmit reuse
therefore trusts this local evidence store. Full postsubmit never reuses its
verdicts and independently checks every model, which bounds this residual risk.
