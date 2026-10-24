---
id: 01M5K2MDGNQX30W9ZB60Z1J20X
number: 145
type: invariant
title: an Oracle broadcast reaches 1000 seats within budget
status: active
slow: true
gate: seats
check: GATE_SEATS=1000 GATE_BUDGET=13299 sh scripts/gates.sh bus
cost: a slower pump means the Oracle's decisions reach the swarm late; the number creeps back toward the 120 s it was
links:
  - { to: "[[d23-recipient-batching]]", rel: decision }
  - { to: "[[c1-live-seats]]", rel: constrains }
  - { to: "[[messaging]]", rel: constrains }
created: 2026-10-23T00:30:04Z
updated: 2026-10-23T00:30:04Z
---
# An Oracle broadcast reaches 1000 seats within budget

**Property.** 1000 synthetic seats, one Oracle and 999 workers, 30 broadcasts at
10/s through the real messaging ledger and fair queue, pumped as logic pumps:
p99 delivery at most **13 299 ms** and every message delivered (decision [[d23-recipient-batching|#23]];
C1 §5 gate 2).

**Check.** `GATE_SEATS=1000 GATE_BUDGET=13299 sh scripts/gates.sh bus`
(`crates/bbt-mux/tests/gates_seats.rs`, prints one `GATE bus value=…` line) —
exit 0 held, non-zero above budget. Slow (about 15 s plus a release build):
runs under the local CI's slow flag, not on every handoff.

**Cost of a violation.** The Oracle's decisions reach the swarm late; a
regression in the pump or the ledger shows here first, and the number creeps
back toward the 120 s it was before per-recipient batching.

**Evidence.** `.bboxy/runs/w2-4c-bus.log`: p99 10 639 / 9 334 / 13 013 ms over
three runs at 22f5956, 29 970 of 29 970 delivered each time (Seraph, 2026-10-22).
Budget = median + 25 %.

Owner: Corvin Duskmantle <ARCHITECT>.
