---
type: decision
title: Keep cooperative1ms units with bounded native effect accounting and rest debt
derived_from:
- '[[tasks/2026-11-05-dream-c1-native-quantum-disposition]]'
- '[[decisions/d93377961364581-dream-c1-enforceable-memory-contract]]'
date: 2026-11-05
id: 01M6PHXGQVMB1PZ6BSHXWHJEKJ
updated: 2026-11-05T19:14:27.889Z
created: 2026-11-05T19:10:37.157Z
number: 93377961365086
created_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
updated_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
authorship:
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  kind: seat
  public_key: L62ggzXO9nS9icBZrd6nPN71BnmasbyzYyfTJE3eu0k=
  process_generation: '1'
provenance:
  digest: sha256:008420f228fcc16173f5bc4057ca7a8b513772853f2436b295de0bfd54eaccc4
  frontmatter_digest: sha256:ebe5268abe2071a50eab8daf11be322028a43073a9bb0c548dc47e958c81bc9f
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  sig: E3Yix5teEpX6DGAfTJNOeym9t7DeMA8/IFTL1hOXqiLyGTiJX57SIt/bqd+iJccc/FnWaC+RGR/nrDTOtifACA==
  log: .bboxy/log/2026-11-05.jsonl#140448
---
# Cooperative C1 quantum and bounded native effects

Date2026-11-05. Governs [[specs/dream-term-c1]], [[interfaces/dream-term-c1]] and inherited Dream read preparation only. Supersedes the interpretation of the CPU row as an absolute preemptive limit on every native operation; it does not change source eligibility, authority, heap, deadline or responsiveness limits. [[decisions/d93377961364581-dream-c1-enforceable-memory-contract]] and [[decisions/d93377961364684-dream-c1-optimized-private-fixture]] otherwise remain.

## Evidence and alternatives

The actual prepared-four run failed: /tmp/dream-c1-prepared-four-real.log:152-175 has12passes/3failures, and runtime-isolated-four-heap.json reports four settled/reaped refusals, zero completed jobs. job0's1,089,208ns is the **whole spawn_prepared call**, not an isolated Command::spawn measurement; whole budget interval1,093,000ns. job1's1,316,083ns is an entire child control tick. job2's configure_existing1,135,292ns combines busy_timeout and two PRAGMAs (whole1,138,750ns). job3's Publication::link1,510,916ns brackets one std::fs::hard_link; its final StorageUnavailable is not proof that no link effect happened. Parent reactor maxima0/175083/109417/58750ns do not certify the failed jobs. Separate parser cleanup failures receive no exception.

Choices: (A) preserve an unimplemented OS-preemption promise and reject ordinary native variance; (B) widen all units or waive native wrappers/SQLite/control/serde; (C) keep cooperative1ms and strict control gates, with a small enumerated native-call lane and full retrospective cost/effect accounting. Choose C. It is a measurable engineering allocation, not an OS maximum or a claim that these failures already pass.

## Exact enforceable law

Application hash/parse/serde/SQLite statements, scans, rendering, allocation-bearing loops and stepped destruction keep **≤1ms measured thread CPU/unit** and existing bounded chunks. Shared UI/mux/async-reactor handlers and established pipe-control polls remain **≤1ms**, with no native launch/SQLite/IO on them. Nice+10/utility and existing owner/count limits remain. An elapsed-time check can refuse further work after an operation; it cannot preempt a syscall already executing.

Only the following fixed operations may use native admission:

| Tag | Exact callable boundary / finite cap |
|---|---|
| process_create | One Command::spawn on an already prepared/validated absolute worker command per admitted run. Frozen argv/environment/three pipes and fixed async-signal-safe priority/group pre_exec only. Command building, validate_runner, taking handles, fcntl/nonblocking, decoding and all surrounding work are ordinary1ms units. Existing capability probe has its separate≤1/account cap, no retry loop. |
| compute_owner_create | At most one Builder::spawn for the existing sole parked compute owner per worker; fixed2MiB stack and bounded entry. Preprovision before Hello/control polling; no source/manifest work until matching Start and authority/foreground admission. No extra pool/thread or product session wake. |
| publish_noreplace | One hard_link or existing no-replace rename for one already staged private artifact, with prechecked identity/path and no overwrite. No create/write/hash/fsync/stat/unlink batch is included. Parent has≤2 such publications/job (manifest and worker-start receipt); producer has≤4096 page publications plus1 descriptor, bounded further by actual row/page/output/deadline admission. Q2 uses its existing tighter selected-output bound, never adds calls. |

Each tagged invocation has an **observed acceptance ceiling2ms thread CPU and25ms wall**. These are deliberately narrow regression/admission allocations, not guaranteed native response times. The2ms allocation has margin over the one directly bracketed1.511ms link sample; it is not measured final-source conformance. The25ms allocation leaves margin inside the unchanged100ms pause/control requirement. Missing clocks, over-limit calls or wrapper application CPU>1ms fail; no clock disabling, sampling subtraction, retry-until-lucky or threshold escalation. Native/kernel CPU charged by the calling-thread clock remains in total CPU; retain spawned-owner/process user+system CPU and startup separately. Do not count descendant CPU twice or describe calling-thread accounting as every descendant's CPU.

A work span contains≤1 tagged call and≤1ms application CPU outside it; inclusive observed span therefore≤3ms. Record both subspans and the inclusive interval. This does **not** let arbitrary3ms application units pass. No other operation gains a native tag by its name or an opaque wrapper. In particular configure_existing must split busy_timeout, cache_size PRAGMA and mmap_size PRAGMA with existing guards/rest; each stays1ms. An individual future overrun needs evidence/disposition, not automatic exception.

## Debt, controls and real effects

After every expensive span with inclusive CPU C, set **rest_debt=9*max(1ms,C)**. The next expensive span, on either parent or child for that run, starts only after that much monotonic wall rest **following the span's end**. No saved idle/IO time, borrowing, parallel parent+child units or debt reset on error/restart/manual duplicate. For an ordinary≤1ms unit this is the existing≥9ms rest; a2ms native+0.5ms application span owes≥22.5ms rest. Before handing Start to a child, pay/transfer remaining parent debt; bootstrap debt is paid before Hello/first work. No concurrent native call per job. Four/root/account and probe caps remain self-service; this is per-run accounting, not a global lock/queue.

This preserves the≤10% amortized allocation for admitted expensive spans after debt is paid (four such allocations≤40%); it does not claim every short window or whole-process/control/startup CPU is≤10%. Retain full owner/process CPU/wall and separate control totals so overhead is visible. Controls/status/cancel never wait on work debt, unavailable IO or the native lane, and do not spend the protected work reserve. Foreground/context/deadline prevents new native/application admission; an in-flight call cannot be recalled. Pause is not falsely reported as settled while an effect is in flight. Existing≤100ms pause, p99≤100/max≤250ms input/message/status/cancel, second-root≤2s, ≥240 frames p99CPU≤4ms/input-to-GPU≤16.7ms,30s job deadline,32/96MiB and2s+2s settlement/escalation gates are unchanged.

A call may produce a process/link/thread even if its timing verdict fails. Retain the typed result/handle and original operation/nonce in owned paced storage **before** evaluating budgets. Keep intent, known/unknown effect, exact PID/start/group or inode/path and settlement evidence. Over-budget is resource failure, never proof of absent effect; do not overwrite it with an unrelated StorageUnavailable or retry the effect. Complete only after immutable result validation, paid debt, current context and actual exit. Cancel/error may perform only bounded required settlement/cleanup, still measured; uncertain effect or a native call that does not return stays settling/quarantined with slot/custody retained. No unsafe thread kill/shared-host abort or deletion of uncertain outputs.

## Producer classification and frozen wire

The accepted macOS producer calls run directly before the async runtime (c1.rs:55-87); its run control tick:1805-1999 includes framing, allocation, compute-thread creation:1854 and join bookkeeping. The1.316ms observation cannot be attributed to thread creation from current data. Instrument subspans, move/preprovision the one compute creation out of the established control tick, keep control application ticks≤1ms and show the cause in real logs. If another application segment exceeds1ms, split/fix it; this decision grants no whole-tick exception. The optional async cli/serve wrapper is not evidence of the actual macOS path.

No C1 JSON/serde/schema/IR/algorithm/control-version change. ResultDescriptor.max_poll_cpu_ns is the honest inclusive maximum at its exact **field-capture cutoff, before final descriptor serialization/hash/write/publication**. Include tagged native intervals observed before that cutoff; never clamp/subtract native time or claim that this immutable field includes its own future publication or later cleanup/control/settlement. Under this documented profile it can exceed1ms only with matching native evidence, and is≤3ms for a completed run; the field alone cannot establish compliance. Terminal disaggregated and inclusive metrics must cover final descriptor serialization/hash/write/publication plus later cleanup/control/settlement, with explicit capture cutoffs and actual exit/reap evidence. Validate captured max_poll_cpu_ns≤terminal inclusive maximum; post-capture violations fail even when a descriptor file exists. No self-referential resealing or wire bump. Retain post-reap full process usage to cover the complete child lifetime and label any final telemetry cutoff honestly. Terminal metrics also retain startup/native totals/maxima/counts, wall, effect and debt/rest observations; application/control/cleanup maxima still≤1ms and native maxima≤2ms/25ms. A legacy consumer's1ms-only check may refuse safely; do not silently teach it that every≤3ms result is valid. Bind both sides to the exact approved executable/source/compiler/features and private metrics profile in their existing local runner receipts; missing required metrics/profile is unproven/incompatible. Raw aggregate and disaggregated metrics remain retained, not new authority. The workspace ordinary-release worker SHA16a746d6d115f75c33402ac61ded2f9ba6ac2272930acd0bd99febb7c9ce4edf is distinct from earlier standalone artifacts despite pinned source47182320.

## Minimal formal and real gates

Finite model C1QuantumDebt: two jobs/owners, two calls, epochs A1/B2/A3; app CPU0..2, native CPU0..3, debt0..27 (abstract ms), foreground/cancel/deadline, native kind/count/in_flight, effect absent/present/unknown, exited/settled/presented. Initially debt0, no child/effect, sources open. Actions Close/AdmitApp/AdmitNative/ReturnNative/PayRest/Foreground/ChangeContext/Cancel/ObserveExit/Present. Atomic admission guards; native execution/return and effects interleave. Invariants: no hot-owner native, no admission with debt/foreground/stale/open source; no cap bypass; full cost creates debt; no repeat of an unresolved effect; no completion before current validation+real exit; control remains enabled during debt/native. Weak fairness applies to internal control/rest/settlement, not OS return. A stuck external call reaches honest quarantine, not assumed IO liveness.

Causal controls remove rest repayment, label2ms parser as native, admit repeated spawn/link under one op, release owner before effect/exit, hide native CPU from total, or exempt a whole child tick/SQLite wrapper. They must violate the intended guard at model/runtime, not fail compilation.

Existing make test-dream-c1 on both repositories and their canonical formal targets add: exact-call CPU+wall profiling; real four-job successful completion plus independent control/heap/foreground/native frames on the actual ordinary-release worker; a repeated-native/debt bypass negative; >2ms and >25ms native outcomes retaining a real staged effect/handle; clock failure; cancel/ABA between effect and verdict; withheld native completion with independent controls and honest quarantine; strict2ms parser/control/cleanup negatives. Use fixed scoped test hooks/private identities, no operator. Preserve the two parser cleanup failures as failures until fixed. No reduced corpus or replaced fake positive. Keep prior failed logs; final success requires fresh source-bound joined evidence, not reinterpreting this failed sample.

## Implementation ownership and primary references

Consumer: logic/dream_c1.rs Budget/native_dispatch_finish/publish_paced/checkpoint_stage; wake-up dream_c1.rs::Worker::spawn_prepared separates validated command preparation from spawn and post-spawn setup; registry Journal::configure_existing supplies split statements and durable effect/settlement bookkeeping. If any launch is on a shared reactor, move it to the existing bounded isolated owner. Producer: fresh assignment for c1.rs bootstrap/compute creation, control subspan instrumentation, debt/metrics and unchanged wire conformance. Accepted C1 task stays frozen. Oracle amends the active Q2 brief to inherit this law; no out-of-scope author edits are requested here.

Official [Rust Command::spawn](https://doc.rust-lang.org/std/process/struct.Command.html#method.spawn) and [thread Builder](https://doc.rust-lang.org/std/thread/struct.Builder.html#method.spawn) describe process/thread creation and returned ownership, not a documented1ms CPU deadline; caller-side non-interleavability is an engineering boundary, not proof of a single kernel syscall. [Apple link(2)](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/link.2.html) documents atomic hard-link publication and result semantics, supporting effect retention after a timing verdict. Fetched2026-11-05; Apple page is archival. POSIX clock pages were403/unavailable; timing semantics are grounded in inspected CLOCK_THREAD_CPUTIME_ID usage, with no new scheduler/OS-preemption claim. Source/classification findings, not a fresh benchmark, choose this correction.
