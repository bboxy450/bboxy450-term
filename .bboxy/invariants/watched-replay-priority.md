---
id: 01M5Q8ZWTAPN6A0K7APMQDYC7S
type: invariant
number: 276
title: "watched seats get at least half of the replay pages when both lanes are runnable"
status: active
check: cargo test -p bboxy450-mux --locked --test logic watched_lane_gets_at_least_half_of_replay_pages -- --exact 2>&1 | grep -q 'ok. 1 passed; 0 failed'
cost: the panes the operator is looking at starve behind hundreds nobody is watching; the frame gate cannot settle and the cockpit looks frozen while logic is busy elsewhere
links:
  - { to: "[[d253-checkpoint-policy]]", rel: decision }
  - { to: "[[c1-live-seats]]", rel: constrains }
  - { to: "[[seats-967-frame]]", rel: unblocks }
created: "2026-10-24T15:38:09Z"
updated: "2026-10-24T15:38:09Z"
---
# Watched seats get at least half of the replay pages when both lanes are runnable

**Property.** With an explicit watch interest registered for a set of seats
(session, seat and generation) and both replay lanes runnable, the watched lane
receives at least half of the replay pages over any window of eight scheduling
rounds (the design value is 3 : 1 watched to background, data not a literal);
when one lane is empty the other receives every page; a watch that ends
(unwatch or disconnect) moves its seat to the background lane on the next round
([[d253-checkpoint-policy]], R5h). The 7.44 % that motivated this is evidence;
the gate is the page count, because a count does not depend on the machine.

**Check.** `cargo test -p bboxy450-mux --locked --test logic watched_lane_gets_at_least_half_of_replay_pages -- --exact 2>&1 | grep -q 'ok. 1 passed; 0 failed'`
— exit 0 held. R5h supplies this four-PTY check with a fixed replay-page
budget, explicit GUI watch admission and exact serial-replay Screens.

**Cost of a violation.** The panes the operator is looking at starve behind
hundreds nobody is watching; the frame gate cannot settle and the cockpit
looks frozen while logic is busy elsewhere.

**Evidence.** Rook's frame-run analysis, 2026-10-24: watched seats at 7.44 % of
replay pages, `ReadRing` at 73 % of logic wall time between producer stop and
timeout, because GUI Watch never reached ingest.

R5h's named check passes with the watched tail complete inside the page budget
while all three background seats remain backlogged; every seat then reaches
its exact serial-replay Screen. Queue tests also cover the 3:1 quota over every
eight-round window, donation, disconnect refcounts and generation/Exit order.
Commands and before/after results are retained in `.bboxy/runs/r5h-*.log`.

Owner: Corvin Duskmantle <ARCHITECT>.
