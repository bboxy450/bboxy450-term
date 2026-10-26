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
updated: "2026-10-26T16:18:20Z"
---
# Budgets

**Lens.** Interfaces can each be correct while their combined demand cannot be met. A budget splits each shared resource across the subsystems, with a margin, so a cut that spends another subsystem's share is seen before it lands. **Allocated** numbers are decisions; **measured** numbers confirm or refute them; a **gap** is a resource with no allocation yet, which is a defect to close, not a default.

## Frame time at 120 Hz, 8.33 ms, under 967 seats with 50 watched

| Stage | Owner | Allocation | Measured |
|---|---|---|---|
| Event drain and model update | window (`bbt-ui-model`) | 2.0 ms | not yet split |
| Scene projection | window (`bbt-ui`) | 1.5 ms | 4 321 projections for 42 frames before Rook's fix |
| Encode and submit | renderer (`bbt-render`) | 2.5 ms | encode 4.9 ms after the atlas fix (earlier, 33 ms) |
| GPU and present | renderer | 1.0 ms | 8.9 ms GPU before the blur fix, at 75 Hz |
| Margin | | 1.33 ms (16 %) | |
| UI CPU p99 | window | under 4 ms (the invariant) | 292 ms on 0e97dc32 |
| Input to GPU | logic and window | within 2 frame intervals, 16.7 ms (the invariant) | 1 076 ms |

The per-stage allocations are the Architect's first split of the invariant's total; the next partitioned 967 run measures each stage against its row, and a stage over its row is that owner's to fix or to argue for a reallocation, never a silent overrun.

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
