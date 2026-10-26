---
id: 01M5WG2WSGF22CM3MKX7VC8P3W
type: spec
number: 355
title: "budgets: frame time, memory, disk, build and run time and inference, allocated across subsystems with margins; measurement confirms an allocation, it never replaces one"
status: baselined
links:
  - { to: "[[interfaces]]", rel: builds-on }
  - { to: "[[resource-ledger]]", rel: builds-on }
  - { to: "[[seats-967-frame]]", rel: constrains }
created: "2026-10-26T16:18:20Z"
updated: "2026-10-26T18:13:24Z"
---
# Budgets

**Lens.** Interfaces can each be correct while their combined demand cannot be met. A budget splits each shared resource across the subsystems, with a margin, so a cut that spends another subsystem's share is seen before it lands. **Allocated** numbers are decisions; **measured** numbers confirm or refute them; a **gap** is a resource with no allocation yet, which is a defect to close, not a default.

## Frame time at 120 Hz, 8.33 ms, under 967 seats with 50 watched

Two clocks run in parallel and are never added together: the **CPU path** of the window, capped by the frame invariant at **4 ms p99**, and the **GPU's own execution**, which overlaps the next frame's CPU work and must also fit the 8.33 ms frame. Corrected 2026-10-26 at Rook's questions (twice: the endpoints, then draw versus encode, checked against the meter's own total): a first version summed CPU stages to 6 ms against the invariant's 4, gave GPU a CPU-style row, and named no endpoints.

| Stage | Endpoints in the code (`bboxy450-term`, source 2dc967db) | Allocation p99 | Measured |
|---|---|---|---|
| Drain | frame start to the call of `project_scene` (`main.rs:5482`), the event drain and model update only; the current `phases.drain` (`main.rs:5492`) includes `project_scene` and is split by timing it separately | 1.5 ms | not yet split |
| Projection | `project_scene` plus `phases.layout` plus `phases.draw` (scene paint and foreground construction, `main.rs:5550-5583`) | 1.0 ms | 4 321 projections for 42 frames before Rook's fix |
| Encode and submit | `phases.encode`: the actual command encoding and `queue.submit` (`gpu.rs:413-511`, returned at `main.rs:5596`) | 1.0 ms | encode 4.9 ms after the atlas fix |
| CPU margin | | 0.5 ms | |
| **CPU path total** | the three above, which sum exactly to the meter's CPU total (`bbt-rt/src/meter.rs:16`, drain plus layout plus draw plus encode) | **under 4 ms** (the invariant) | 292 ms on 0e97dc32 |
| GPU execution | Metal's own GPU execution duration for the frame's command buffer (`gpu.rs:495-498`) | 6.0 ms | 8.9 ms before the blur fix, at 75 Hz |
| GPU margin | | 2.33 ms | |
| Present wait | `phases.present_wait`, the host's wait; reported, not allocated, expected near zero, since the UI thread never waits | report only | |
| Input to GPU | end to end from input admission to the GPU completion of the frame that shows it | within 2 frame intervals, 16.7 ms (the invariant) | 1 076 ms |

**How samples are kept.** The partitioned run records bounded raw per-frame samples of each phase and of the GPU duration, without changing thresholds or scheduling (Rook's proposal); a frame whose GPU duration is unavailable counts as unavailable, never as zero; the verdict per row is its p99 over available samples with the unavailable count beside it. A stage over its row is its owner's to fix or to argue for a reallocation, never a silent overrun.

## Logic catch-up and history

| Item | Allocation |
|---|---|
| Ring read round trips per pass | at most ceil(N/8) |
| Parsed debt at a logic kill | at most 16 MiB per seat |
| Replay until live after a logic kill | under 71.7 s |
| Watched share of replay pages | at least one half |

## Memory

| Item | Allocation | Measured |
|---|---|---|
| 400 idle seats, supervisor plus logic | 420 MiB | 333.7 MiB median, about 825 KiB per seat |
| Split between supervisor and logic | **gap** | the ledger's per-process RSS supplies it |
| Window and renderer | **gap** | |

## Disk on the shared machine

| Item | Allocation |
|---|---|
| Free space floor | 10 GiB; slots drop to two under 25 GiB |
| Private build target per agent | about 5 GiB, own targets only |
| Parity render store | last green sets only |
| Committed evidence | hash and summary per run, never raw logs (#341) |

## Build and run time per landing

| Item | Allocation |
|---|---|
| Presubmit run | 15 minutes over tests and invariants |
| Presubmit build | 30 minutes, an infrastructure cap |
| A presubmit test binary | 60 s, measured alone in the night |

## Messaging and inference

| Item | Allocation |
|---|---|
| Bus p99 at 1 000 seats | 13.3 s |
| Digest window | at most 10 minutes or 50 messages |
| Events waking a model under defaults | zero |
| Inference per landed cut, per agent | **gap**: the ledger measures it first, then an allocation is set |

Owner: Corvin Duskmantle <ARCHITECT>.
