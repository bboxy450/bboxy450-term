---
id: 01M5P27M8GWEFR5YP5665P8CP3
type: invariant
number: 254
title: "crash recovery work per seat stays under the checkpoint bound"
status: pending
slow: true
gate: pending-r5g
check: cargo test -p bbt-mux --locked --test recovery checkpoint_bounded_recovery -- --exact 2>&1 | grep -q 'ok. 1 passed; 0 failed'
cost: a seat that replays an hour of ring after a crash is a seat the operator cannot use for a minute; unbounded checkpoint work is the stall window 4 measured, 72 percent of logic wall time
links:
  - { to: "[[d253-checkpoint-policy]]", rel: decision }
  - { to: "[[mux-daemon]]", rel: constrains }
  - { to: "[[seats-967-frame]]", rel: unblocks }
created: "2026-10-24T04:20:48Z"
updated: "2026-10-24T04:20:48Z"
---
# Crash recovery work per seat stays under the checkpoint bound

**Property.** Under the firehose at the gate's seat count, `SIGKILL` logic
mid-replay and restart: every seat's replay from its last checkpoint cursor to
the ring tail is at most **2R = 16 MiB** ([[d253-checkpoint-policy]] rules 2
and 3), no seat received a maintenance checkpoint while backlogged, and the
time until every seat is live again is under a budget written here from the
first three settled runs (median + 25 %). The ingest loop never awaited a save.

**Check.** `cargo test -p bbt-mux --locked --test recovery checkpoint_bounded_recovery -- --exact 2>&1 | grep -q 'ok. 1 passed; 0 failed'`
— exit 0 held. The test does not exist yet; R5g creates it and its commit
removes `slow` and `gate` from this file so it joins the default set. The
overhead half of the policy (checkpoint RPC share at most 10 % of logic wall
time) is read from the R5 diagnostics in the frame gate, not from this test.

**Cost of a violation.** A seat that replays an hour of ring after a crash is a
seat the operator cannot use for a minute; unbounded checkpoint work is the
stall window 4 measured, 72 % of logic wall time.

**Evidence.** Window 4 (Aster, frozen source 7998996b, 2026-10-24): 3 256 saves,
checkpoint RPC 70.6 % of wall, 605 of 608 retained maintenance saves on
backlogged seats; the policy's numbers are derived in the decision.

Owner: Corvin Duskmantle <ARCHITECT>.
