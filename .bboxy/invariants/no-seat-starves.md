---
id: 01M5QXJV03J5WH1X4DT2P227ND
type: invariant
number: 314
title: "no seat starves under a full retained-notification queue: every seat with a pending retained notification is admitted within one rotation of the retry sweep"
status: active
check: cargo test -p bbt-supervisor --locked --lib supervisor::capture_read_tests::no_seat_starves_under_full_queue -- --exact 2>&1 | grep -q 'ok. 1 passed; 0 failed'
cost: the seats at the end of the sweep order never get their retained notifications admitted while the hot first seats take every credit; their panes go stale, their watchers see nothing, and with hundreds of seats the tail is silent for the life of the run
links:
  - { to: "[[no-chokepoints]]", rel: instance-of }
  - { to: "[[d253-checkpoint-policy]]", rel: builds-on }
  - { to: "[[seats-967-frame]]", rel: unblocks }
  - { to: "[[mux-daemon]]", rel: constrains }
created: "2026-10-24T21:38:02Z"
updated: "2026-10-24T21:50:28Z"
---
# No seat starves under a full retained-notification queue

**Property.** With the supervisor's retained-notification queue full (128
slots, the bounded queue and its admission rules unchanged) and N seats each
holding a pending retained notification, every one of the N seats receives an
admission within N consecutive retry passes, whatever its position in seat-id
order: each pass resumes the sweep where the previous pass stopped instead of
restarting from the first key. Over any window of N passes no seat's admission
count is zero while another seat's is above zero. The 12,409 attempts that
motivated this are evidence; the gate is the admission count per seat under a
deterministic full queue, because a count does not depend on the machine.

**Alternatives.** (a) A larger queue: hides the order, keeps the starvation,
just later. (b) A queue per seat: more state and a different bounded-queue
rule for one sweep. (c) Fair rotation over the same bounded queue and the same
rules. **Choice** (c), Oracle, 2026-10-24, with a deterministic counterexample
test as the gate.

**Check.** `cargo test -p bbt-supervisor --locked --lib supervisor::capture_read_tests::no_seat_starves_under_full_queue -- --exact 2>&1 | grep -q 'ok. 1 passed; 0 failed'`
— exit 0 held. The deterministic fixture uses two pending seats, a one-slot
channel and exactly one returned credit per pass. The first seat stays hot;
the second must receive the next credit, and the hot seat's sequences remain
FIFO. The same test failed on the fixed-order sweep before the rotation fix.
The fully qualified unit-test name and one-passed assertion prevent a zero-test
run from satisfying the check. The production channel remains 128 slots.

**Cost of a violation.** The seats at the end of the sweep order never get
their retained notifications admitted while the hot first seats take every
credit; their panes go stale, their watchers see nothing, and with hundreds of
seats the tail is silent for the life of the run. The fifth known instance of
[[no-chokepoints]]: one sweep order serialising every seat behind the first.

**Evidence.** Rook's 967/50 analysis, 2026-10-24: the retry sweep walks the
`BTreeMap` from the same start every pass (`crates/bbt-supervisor/src/supervisor.rs:977-982`, `:2117`),
so three watched seats took zero admissions in 12,409 attempts while the hot
first seats held every credit of the 128-slot queue. Reported by the Oracle
to the Architect the same day; the fair-rotation fix was approved by the Oracle
on the spot.

Fair rotation and the executable counterexample: signed source
`f85912a03db8dba69abd8393c789236d4b3b84c7`; the cursor advances only after
successful admission, and the next sweep starts after that seat.

Owner: Corvin Duskmantle <ARCHITECT>.
