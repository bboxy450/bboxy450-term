---
id: 01M5Q8ZWTAPN6A0K7APMQDYC7S
type: invariant
number: 276
title: "watched seats get at least half of runnable ingestion turns"
status: active
check: make test-watched-replay-priority
cost: the panes the operator is looking at starve behind hundreds nobody is watching; the frame gate cannot settle and the cockpit looks frozen while logic is busy elsewhere
links:
  - { to: "[[d253-checkpoint-policy]]", rel: decision }
  - { to: "[[c1-live-seats]]", rel: constrains }
  - { to: "[[seats-967-frame]]", rel: unblocks }
created: "2026-10-24T15:38:09Z"
updated: "2026-10-31T20:30:00Z"
---
# Watched seats get at least half of runnable ingestion turns

**Operator amendment, #47, 2026-10-30.** Independent read/parser completions
replace blocking replay turns. The existing 3:1 scheduler quota is unchanged;
the gate now measures its actual scheduling turns. A seat awaiting its bounded
read/parser completion cannot admit another page. Background work continues
while it waits, as the direct independent-progress brief requires. Completed
RPCs no longer stand for runnable scheduling turns or their eight-turn windows.

**Property.** With explicit current watch interest and both lanes runnable, the
watched lane gets at least half of every eight scheduling turns (design 3:1).
Empty capacity is donated, background service remains bounded, and unwatch or
disconnect demotes the seat on the next turn. This is a scheduling property,
not a ratio of independently completed reads, CPU deadlines or read throughput.

**Check.** `make test-watched-replay-priority` must exit 0. The exact native
`watched_quota_is_three_to_one_and_each_lane_rotates` witness checks every
sliding eight-turn window; companion queue tests check donation, disconnect,
generation and parked hosts. SeatFairness and SeatIngestion cover the finite
fairness/independence projection. The real four-PTY witness requires the watched
tail to publish within the fixed `2 * watched_source_pages + 16` read budget,
while background seats remain backlogged, then all seats match serial Screens.
Already admitted parsing may finish with further reads held at that budget.

**Evidence.** R5h originally counted blocking RPC pages as turns. The #47 gate
found a three-of-eight RPC-completion window while the watched seat still
finished all 32 replay reads inside its 82-read budget, with exact serial output.
That proxy is invalid after independent parsing; restoring a global completion
barrier to satisfy it would violate #47. The queue quota remains a hard check,
and the real bounded-tail/screen witness remains a separate hard check. The
signed #47 commit and `.bboxy/runs/ingest-47.log` record this decision and exits.

**Cost.** Visible panes must retain priority without blocking healthy background
seats on an unrelated pending completion. Quiet latency acceptance remains #47.

Owner: Corvin Duskmantle <ARCHITECT>; amended under the direct operator brief.
