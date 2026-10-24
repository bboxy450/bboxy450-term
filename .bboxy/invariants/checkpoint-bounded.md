---
id: 01M5P27M8GWEFR5YP5665P8CP3
type: invariant
number: 254
title: "crash recovery work per seat stays under the checkpoint bound"
status: pending
slow: true
gate: pending-r5g
check: R5G_SEATS=4 cargo test -p bbt-mux --locked --test recovery checkpoint_bounded_recovery -- --exact 2>&1 | grep -q 'ok. 1 passed; 0 failed'
cost: a seat that replays an hour of ring after a crash is a seat the operator cannot use for a minute; unbounded checkpoint work is the stall window 4 measured, 72 percent of logic wall time
links:
  - { to: "[[d253-checkpoint-policy]]", rel: decision }
  - { to: "[[mux-daemon]]", rel: constrains }
  - { to: "[[seats-967-frame]]", rel: unblocks }
created: "2026-10-24T04:20:48Z"
updated: "2026-10-24T05:19:20Z"
---
# Crash recovery work per seat stays under the checkpoint bound

**Property.** Under the firehose at the gate's seat count, `SIGKILL` logic
mid-replay and restart (a fixture that starts 20 MiB behind has backlog, not
debt, and is valid): every seat's **parsed debt** at the kill, the replay from its last confirmed
checkpoint cursor to where parsing stood, is at most **2R = 16 MiB**
([[d253-checkpoint-policy]] rules 2 and 3; the unparsed backlog, ring tail
minus parsed cursor, is the catch-up gap and is reported beside it, not
bounded here), no seat received a maintenance checkpoint while backlogged, and the
time until every seat is live again is under a budget written here from the
first three settled runs (median + 25 %). The ingest loop never awaited a save.

**Check, two counts, one test (Rook's proposal, Oracle and Architect, 2026-10-24).** Default: `R5G_SEATS=4 cargo test -p bbt-mux --locked --test recovery checkpoint_bounded_recovery -- --exact 2>&1 | grep -q 'ok. 1 passed; 0 failed'`, exit 0 held, with **4 real PTYs and every assertion** (SIGKILL mid-replay, parsed debt at most 2R, backlog reported, restart to live); this is the line in `check:` and it joins the default set when R5g lands. Full count: `R5G_SEATS=967 cargo test -p bbt-mux --locked --test recovery checkpoint_bounded_recovery -- --exact 2>&1 | grep -q 'ok. 1 passed; 0 failed'`, exit 0 held, run in a gate slot for the three scheduled budget runs and the evidence below; never in the default set, because two default runs at 967 PTYs would contend for the 999-PTY ceiling (decision #19) and could not both pass. The test does not exist yet; R5g creates it and its commit removes `slow` and `gate` from this file. The
overhead half of the policy (checkpoint RPC share at most 10 % of logic wall
time) is read from the R5 diagnostics in the frame gate, not from this test.

**Cost of a violation.** A seat that replays an hour of ring after a crash is a
seat the operator cannot use for a minute; unbounded checkpoint work is the
stall window 4 measured, 72 % of logic wall time.

**Evidence.** Window 4 (Aster, frozen source 7998996b, 2026-10-24): 3 256 saves,
checkpoint RPC 70.6 % of wall, 605 of 608 retained maintenance saves on
backlogged seats; the policy's numbers are derived in the decision.

Owner: Corvin Duskmantle <ARCHITECT>.
