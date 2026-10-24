---
id: 01M5P27M8GWEFR5YP5665P8CP3
type: invariant
number: 254
title: "crash recovery work per seat stays under the checkpoint bound"
status: active
check: R5G_SEATS=4 cargo test -p bbt-mux --locked --test recovery checkpoint_bounded_recovery -- --exact 2>&1 | grep -q 'ok. 1 passed; 0 failed'
cost: a seat that replays an hour of ring after a crash is a seat the operator cannot use for a minute; unbounded checkpoint work is the stall window 4 measured, 72 percent of logic wall time
links:
  - { to: "[[d253-checkpoint-policy]]", rel: decision }
  - { to: "[[mux-daemon]]", rel: constrains }
  - { to: "[[seats-967-frame]]", rel: unblocks }
created: "2026-10-24T04:20:48Z"
updated: "2026-10-24T13:12:58Z"
---
# Crash recovery work per seat stays under the checkpoint bound

**Property.** Under the firehose at the gate's seat count, in the load fixture's
swarm shape of one or two swarms (never one swarm per seat), `SIGKILL` logic
mid-replay and restart (a fixture that starts 20 MiB behind has backlog, not
debt, and is valid): every seat's **parsed debt** at the kill, the replay from its last confirmed
checkpoint cursor to where parsing stood, is at most **2R = 16 MiB**
([[d253-checkpoint-policy]] rules 2 and 3; the unparsed backlog, ring tail
minus parsed cursor, is the catch-up gap and is reported beside it, not
bounded here), no seat received a maintenance checkpoint while backlogged, and the
time until every seat is live again is under the budget **71 666 ms** at 967
seats (median 57 332.7 ms of the three calibration runs + 25 %, ceiling to the
millisecond; **the clock starts just before the replacement `Logic::start`,
after the diagnostic tail probes, and stops when all 967 seats match their
snapshots**, so the kill itself and the probes are outside it; the default 4-PTY run
asserts the same properties with no time budget). The ingest loop never awaited a save **after readiness**: the one exception is the #11 path before readiness (cold bootstrap and explicit recovery), where a forced checkpoint may be awaited synchronously, and the test asserts no synchronous save wait after readiness (Oracle's ruling, confirmed by the Architect, 2026-10-24).

**Check, two counts, one test (Rook's proposal, Oracle and Architect, 2026-10-24).** Default: `R5G_SEATS=4 cargo test -p bbt-mux --locked --test recovery checkpoint_bounded_recovery -- --exact 2>&1 | grep -q 'ok. 1 passed; 0 failed'`, exit 0 held, with **4 real PTYs and every assertion** (SIGKILL mid-replay, parsed debt at most 2R, backlog reported, restart to live); this is the line in `check:` and it joins the default set when R5g lands. Full count: `R5G_SEATS=967 R5G_BUDGET_MS=71666 cargo test -p bbt-mux --locked --test recovery checkpoint_bounded_recovery -- --exact 2>&1 | grep -q 'ok. 1 passed; 0 failed'`, exit 0 held, run in a gate slot for the three scheduled budget runs and the evidence below; never in the default set, because two default runs at 967 PTYs would contend for the 999-PTY ceiling (decision #19) and could not both pass. The test does not exist yet; R5g creates it and its commit removes `slow` and `gate` from this file. The
overhead half of the policy (checkpoint RPC share at most 10 % of logic wall
time) is read from the R5 diagnostics in the frame gate, not from this test.

**Cost of a violation.** A seat that replays an hour of ring after a crash is a
seat the operator cannot use for a minute; unbounded checkpoint work is the
stall window 4 measured, 72 % of logic wall time.

**Evidence.** Window 4 (Aster, frozen source 7998996b, 2026-10-24): 3 256 saves,
checkpoint RPC 70.6 % of wall, 605 of 608 retained maintenance saves on
backlogged seats; the policy's numbers are derived in the decision. **Calibration
v3 (Rook, 2026-10-24, 967 PTYs, the two-swarm fixture):** 57.5 s, 57.2 s and
57.3 s until all 967 seats were live after `SIGKILL` mid-replay, nothing lost,
parsed debt within 2R (signed source 34cc1356; evidence commit 7cf403e1,
`.bboxy/runs/r5g-calibrations.log` with raw outputs, SHA chains and archive
hashes); budget 71 666 ms = median + 25 %. Still `pending` until Rook's commit
lands the test and, in the same commit, removes `slow` and `gate` and sets
`status: active`: the switch lands with the load, never before it.

**Test evidence.** Signed candidate commit `36032d38d532156428cb3beb6d050dae2c00e769` adds `crates/bbt-mux/tests/recovery.rs::checkpoint_bounded_recovery`; `90d6c62db75549b4c2462b0ffa330efa745d3293` adds its calibrated `R5G_BUDGET_MS` assertion and default activation.

Owner: Corvin Duskmantle <ARCHITECT>.
