---
id: 01M649KN623JXTGPKG9JCRWV04
type: decision
number: 365
title: "TLA+/TLC and Yarn verification are mandatory; native Rust checker runs beside TLC"
status: decided
decided: "2026-10-29"
operator:
  - "make TLA+ and TLC part of verification stack for our codebases and all our crates!"
  - "TLA+/TLC + yarn verification are mandatory in our codebase!"
  - "implement TLC runtime in yarn crate in pure rust"
  - "for a while we can run java + our rust implementation side by side"
  - "until we provee to ourselves that our implementation is complete"
links:
  - { to: "[[yarn]]", rel: amends }
  - { to: "[[d33-yarn-macro]]", rel: builds-on }
created: "2026-10-29T16:59:05Z"
updated: "2026-10-29T16:59:05Z"
---
# Decision 365: mandatory formal verification

The operator's direct instructions on 2026-10-29 authorize this decision and its
implementation. **TLA+/TLC plus Yarn verification are mandatory throughout our
codebases and every crate.** This augments unit, property, process, smoke and e2e
checks; a checked design never substitutes for exercising its implementation.

**Yarn is a constrained frontend.** Its existing Rust reactor grammar projects
into a finite state-machine representation. The same representation feeds two
independent algorithms: the standalone `bboxy450-tlc` native Rust explicit-state checker, re-exported by Yarn, and a
TLA+ exporter checked by upstream Java TLC. Export TLA+ from the model's actions,
never from the native checker's discovered graph. Reuse one reactor declaration
for runtime/model where possible; arbitrary Rust handlers require explicit
behavioral contracts and remain outside an automatic scheduling projection.

**Publication:** the native runtime is `bboxy450-tlc`, a dependency-free standalone crate with public source at https://github.com/bboxy450/bboxy450-tlc, documentation, examples and the project's Respectful Slop License/attribution policy. The operator explicitly authorized crates.io and GitHub publication as bboxy450; publication still verifies that identity and signs tags and packaged artifacts.

**Native Rust is the direction; Java TLC is the reference during development.**
Both run unattended, side by side. The mandatory gate compares the complete
reachable state sets and transition relations as well as safety, deadlock and
liveness outcomes, including deliberately broken controls. A disagreement,
timeout, unsupported input, truncated search, missing tool, missing crate or
missing negative control fails. No environment flag silently disables reference
checking. No Java process is introduced into the application runtime or ordinary
reactor execution. Tooling uses pinned, checksum-verified local downloads with
private worktree outputs, no global install, GUI, credentials or operator prompt.

**Scope is explicit.** The initial native subset is finite integer/Boolean
state expressions, guarded simultaneous assignments, safety/deadlock checks,
response properties, weak-fair action groups and stuttering. It is not a full
TLA+ parser, module evaluator, TLAPS prover or complete TLC replacement. Opaque
handlers, real executor wakes, unbounded collections, crash durability and wall
clock performance are not established by the initial reactor projection.
`verification/coverage.toml` inventories every workspace crate and distinguishes
bounded contract models from behavior still covered only by implementation
tests. Those entries are coverage gaps, not a claim that all crates are formally
proved. Changes to concurrency, ownership, lifecycle, persistence, retries and
arbitration must update their models and negative controls; new crates must have
an explicit coverage entry and cannot bypass this verification stack by omission.

**Retiring Java requires a subsequent explicit decision.** A green bounded
corpus proves agreement on that corpus only. Before promoting the Rust backend,
record a language/semantic compatibility matrix, upstream example conformance,
independent generated/adversarial differential tests, parser/evaluator/error
compatibility, safety/deadlock/fairness/liveness evidence and reproducible resource
limits. Establish which TLA+ features and TLC modes are supported. Do not call a
restricted frontend complete TLC, and do not remove the reference on confidence
or an agent's report.

**Executable gate:** `make test-formal`, included in `make test`, scoped presubmit/rechecks and manual hosted
CI. It runs native Rust tests/strict lint, negative infrastructure controls, and
native/TLC graph and verdict comparison. Sources, model hashes, bounds, tool
versions, actual exit statuses and coverage are recorded under
`target/formal/evidence.json`; full per-model logs and native counterexamples
stay under `target/formal/models`. See `verification/README.md`.

Rejected: reimplementing all of TLA+ before delivering a checked subset;
replacing TLC on a handful of green tests; running Java inside a proc macro or
the product; declaring arbitrary Rust correct from scheduling metadata; hiding
unmodeled crates; treating elapsed-time budgets as established by a finite model.
