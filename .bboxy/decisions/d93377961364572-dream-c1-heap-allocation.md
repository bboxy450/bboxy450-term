---
type: decision
title: C1 fixes parent 32 MiB and child 96 MiB within the combined 128 MiB live heap ceiling
id: 01M6NTAHBPPEQMBJGDENTYG18G
updated: 2026-11-05T12:18:17.928Z
created: 2026-11-05T12:18:17.928Z
number: 93377961364572
created_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
updated_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
authorship:
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  kind: seat
  public_key: L62ggzXO9nS9icBZrd6nPN71BnmasbyzYyfTJE3eu0k=
  process_generation: '1'
provenance:
  digest: sha256:46118636ab3b918b05d9b7fcf1b32d6c2a7b95e2e3ace00a2c69dff32caa1f45
  frontmatter_digest: sha256:828a1fb9dd65c6898fdf85633c0dd1d897f9b1df3f7411bb5576dd058ede1f25
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  sig: GSKYytfPrRxLxc9T2s705RjjncKLyrqImNUaXnV1QVmjVLdLoxQ9fJZGE1JIkTyAQgSUwz+m4Ae7gYgcufEKCA==
  log: .bboxy/log/2026-11-05.jsonl#135334
---
# Fixed C1 live heap split

C1 retains its **128 MiB combined live working-allocation ceiling per job**: consumer/parent **32 MiB (33,554,432 bytes)** plus producer/child **96 MiB (100,663,296 bytes)**. No borrowing, swapping, uncharged helper, new wire field/version, larger input/output cap or increased threshold. This fixed v1 allocation governs [[specs/dream-term-c1]] and [[interfaces/dream-term-c1]] under [[tasks/2026-11-05-dream-c1-heap-allocation]].

| Owner | Work + reserve within its fixed ceiling |
|---|---|
| Consumer parent | 30 MiB work + 2 MiB exclusively for control/status/cancel/error/settlement |
| Producer child | 94 MiB work + 2 MiB exclusively for framing/control/progress/error/settlement |

**What counts.** Concurrent live allocation capacity, not logical lengths: source/read/copy/line buffers; parser/serde trees and strings; transcript/D0 structures; context/manifest/request/record copies; result/page/index/hash/canonicalization buffers; channel queues, thread closures and bookkeeping; C1-attributable native/library buffers including SQLite; allocator rounding/metadata covered by the reservation bound. Every background thread/helper of that job uses the same owner ledger. Charge shared C1 allocations to a documented owner/reserve before use; never omit them or charge them to an unrelated subsystem. Child total includes its startup/runtime heap baseline, not only compute deltas. Parent means job-attributable allocations in the shared host, not all unrelated mux/application heap. Retained job/control data remain charged while live. Stacks, executable/library mappings, kernel buffers, file cache and RSS are separate OS-memory evidence, never mislabeled heap.

**Enforcement.** Before creating any job buffer or starting an allocation-bearing unit, acquire an atomic nonblocking reservation for its conservative **maximum simultaneous live capacity**, including old+new storage during growth/realloc, parser temporaries, clones, serialization and nested/native calls. Held reservations plus the protected control reservation must never exceed the owner's ceiling. Use fallible, explicitly bounded builders/reserve operations; reserve before allocation, keep the charge until actual drop/free, and account for cross-thread transfers. Existing Rust/serde/native calls may be retained only with a source-derived bound covering all their allocations and overlap; otherwise replace that local path with a bounded visitor/builder or refuse it before allocation. An arbitrary bytes-times-factor or post-unit heap sample is not a proof of coverage. No allocation-failure panic/unwind from GlobalAlloc, unbounded process-global allocator switch, or host-wide kill is a resource-limit strategy. Native/library memory without an enforceable bound cannot enter a certified job.

Preallocate the 2 MiB control reserve before expensive work, with fixed bounded frames/status/error storage. Compute/source/output cannot consume it. Failed reservation/checked-size arithmetic/unsupported allocation coverage returns resource_limit before the effect, stops further expensive admissions and uses that reserve for independent cancellation, terminal reporting and actual process/thread settlement. Never abort the shared host or publish a partial completed result. If child startup/control provisioning cannot be established, fail runner admission honestly; no unmetered fallback. A source inside byte/line caps may fail the smaller working allowance; no cap is relaxed to make it fit. The existing at-most-four account jobs/two-per-root and ownership/settlement gates remain; every job and its concurrent control/status helpers must pass its own memory admission.

**Why this split.** Inspected P src/dream/c1.rs:30,114-132,221-241 permits 128 MiB independently and samples process heap around each unit with a nominal 16 MiB margin; its :892 control buffer and :719-746 manifest/log/D0 storage coexist. C crates/bbt-mux/src/logic/dream_c1.rs:37,604-611 permits another 128 MiB, with selected estimates at :762-768,997-1003,1127-1131 and ordinary serde/canonical copies. Streaming 64 KiB IO and bounded frames/pages make a smaller parent feasible through fail-closed local admission; current counters alone do not enforce it. Merely changing constants is insufficient. Prefer this proposed conservative split over widening the combined budget or pretending the two independent maxima coincide.

**Evidence required.** Both authors add/run their existing canonical make test-dream-c1 with a real parent plus actual producer subprocess over multi-MiB input, all relevant threads/control lanes active. Record each owner's attributed actual live heap high-water, enforced reservation high-water, allocator coverage/baseline and cumulative allocated bytes separately; report process RSS separately. Derive synchronized simultaneous parent+child samples if measured, with sampling limits stated. Otherwise report max(parent)+max(child) only as a conservative upper bound, **not** a measured simultaneous peak. The hard reservation bounds establish P(t) ≤32 MiB and C(t) ≤96 MiB, hence P(t)+C(t) ≤128 MiB for all t, including between samples. Sampling does not establish that invariant.

Controlled attempts must exhaust parent work and child work independently (and cover transient realloc/parser/serialization overlap), fail resource_limit before exceeding the cap, preserve unchanged input/immutable prior results, and keep cancel/status/settlement responsive from the protected reserves. Repeat with four admitted jobs so helper work cannot bypass admission. A causal broken control removing reservation/charge coverage must be detected by independent allocation evidence. Preserve existing CPU/duty/latency/source/wire/formal requirements; extend the existing resource model/control only for the two fixed credits and protected control reserve. No new broad audit or implementation assignment.

Snapshot, not measurements: P HEAD 5eec82a33959b764b32cbd7692cac2482fd3e009, c1.rs SHA256 8235462e51025684d2fe1e661f786b42e0626d81985dfabd5185fd242d95f199; C HEAD ad243195daf74153756308346d01a56390c1261c, logic/dream_c1.rs SHA256 11bb0a2ab859298937c0cac850177edfa450ead08ebc39d773329d876238cc8b. Sources were read on 2026-11-05; producer was clean and consumer dirty. Implementation and allocator-coverage evidence remain the respective authors' work.