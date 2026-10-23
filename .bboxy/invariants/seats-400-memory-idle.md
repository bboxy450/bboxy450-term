---
id: 01M5K2MDGNTPHBVVW0NSZK885B
type: invariant
title: 400 idle seats fit in 420 MiB
status: active
slow: true
check: GATE_SEATS=400 GATE_RATE=idle GATE_SECONDS=20 GATE_BUDGET=420 sh scripts/gates.sh memory
cost: per-seat memory creeping up is the one thing that makes 1000 seats impossible again
links:
  - { to: "[[c1-live-seats]]", rel: constrains }
  - { to: "[[vision]]", rel: constrains }
created: 2026-10-23T00:30:04Z
updated: 2026-10-23T00:30:04Z
---
# 400 idle seats fit in 420 MiB

**Property.** 400 idle seats (real PTYs, no output) cost at most **420 MiB** RSS
across supervisor and logic (C1 §5 gate 1, idle floor). Measured 2026-10-22:
295.0 / 333.7 / 353.6 MiB, median 333.7, about 825 KiB per seat over an 11.4 MiB
baseline; budget = median + 25 %. The loaded number under the firehose is a
separate invariant once W1.2d lands.

**Check.** `GATE_SEATS=400 GATE_RATE=idle GATE_SECONDS=20 GATE_BUDGET=420 sh scripts/gates.sh memory`
— exit 0 held, non-zero above budget. Slow (about 30 s plus a release build):
runs under the local CI's slow flag.

**Cost of a violation.** Per-seat memory creeping up is the one thing that makes
1000 seats impossible again; [[vision]] rejected 15 to 20 MB per seat on day one.

**Evidence.** `.bboxy/runs/c1d-memory-floor.log` (Seraph, 2026-10-22).

Owner: Corvin Duskmantle <ARCHITECT>.
