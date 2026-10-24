---
id: 01M5P27M8GFH1S08F14FZN8JBT
type: decision
number: 253
title: "checkpoints bound recovery work, never ingestion: backlog-aware, bounded idle checkpointing"
status: decided
decided: "2026-10-24"
operator: []
links:
  - { to: "[[mux-daemon]]", rel: decides }
  - { to: "[[d21-ring-durability]]", rel: amends }
  - { to: "[[d22-tick-journal]]", rel: builds-on }
  - { to: "[[c1-live-seats]]", rel: constrains }
  - { to: "[[checkpoint-bounded]]", rel: constrains }
  - { to: "[[seats-967-frame]]", rel: unblocks }
created: "2026-10-24T04:20:48Z"
updated: "2026-10-24T04:31:55Z"
---
# Decision 253: checkpoints bound recovery work, never ingestion

Ruled by the Architect for R5g (Rook) on 2026-10-24, from Aster's window 4 run at 967 seats and 50 watched (frozen source 7998996b, report in Rook's R5f record): **72.4 % of logic wall time went to checkpoint saves** (3 256 saves, checkpoint RPC 70.6 %, ReadRing 19.4 %), and 605 of the 608 maintenance checkpoints in the retained tail fired on seats still backlogged, because `logic.rs:2920` resets `changed_at` on every replay page, `:4191-4201` calls a seat idle after 200 ms of parser silence even while it is behind the ring, and `writer.rs:742-745` awaits each strong-committed save serially inside the ingest loop. Replay never stopped; the catch-up budget was eaten. Amends [[d21-ring-durability]] with the checkpoint policy it lacked.

**What a checkpoint is for.** Durability is the ring and the tick journal (#21, #22): a crash loses at most one tick per seat of admitted bytes, a power cut at most 5 s or 16 MiB per machine, and never a torn record. A parser checkpoint loses no bytes when it is late; it only bounds **recovery work**, the bytes replayed from the checkpoint cursor to the ring tail at restart, and so the time to the first live frame after resume (#11). The policy therefore bounds two things, recovery work per seat and checkpoint overhead per machine, and makes ingestion wait for neither.

**The rules.**

1. **Idle means caught up.** A seat is idle only when its parse cursor equals the ring tail **and** no byte has arrived for 200 ms. A backlogged seat is never idle, so the maintenance checkpoint never fires on it. `changed_at` is the arrival time of the **oldest** un-checkpointed byte and is never reset by a replay page.
2. **The recovery bound.** A checkpoint is forced when a seat's un-checkpointed bytes reach **R = 8 MiB**, or its oldest un-checkpointed byte is **60 s** old, whichever first. R is derived from a 1 s replay target at the unloaded replay rate measured today (about 10 MiB/s, from Sable's 20k-line seeks in C1c-3) and is re-derived when that rate is measured again; it lives as data beside the tick budget, not as a literal in the loop. **The bound is checked on every `catch_up` yield** (Rook's source read, 2026-10-24): the live-event guards at `logic.rs:2846-2850` (bytes, 256 sequences, non-output) are never reached during partial replay because `event_inner` returns at `:2779-2780` when `catch_up` yields, and the only replay save today sits at the target boundary (`:2873-2875`, `:2921-2924`); so replay debt, bytes or records since the last **confirmed** checkpoint, and age are both evaluated at each yield, and `checkpoint_sequence` still advances only on confirmed publication (`save_inner:2741-2744`).
3. **Checkpoint work is bounded and off the ingest path.** Saves run on a side task and the ingest loop never awaits one. **Two debts, named apart (Rook, 2026-10-24):** *parsed debt* is parsed cursor minus confirmed checkpoint cursor, the only thing a checkpoint can cover; *backlog* is ring tail minus parsed cursor, which grows with capture whether logic parses or not. This policy bounds **parsed debt**: forced at R, and a seat whose parsed debt reaches **2R** pauses **its own** parsing until its save lands, so parsed debt is bounded at 2R per seat absolutely while every other seat keeps flowing; the pause is counted and shown, never silent, and it grows that seat's backlog, which is accepted. Backlog is the catch-up problem itself: bounded by the settle gate and by #21's backpressure to the PTY when the ring is busy, **never by a new capture or admission bound** from this policy (rejected: throttling a PTY because logic is slow). Recovery work after a crash is parsed debt plus backlog; the first is this policy's number, the second is reported beside it as the catch-up gap. **On the wire:** the supervisor protocol allows one unresolved request per leased stream (`client.rs:263`, the one-frame reply reserve), so in v1 the side task issues saves **one at a time on the leased stream, interleaved between `ReadRing` requests**; **K = 4 is the depth of the forced-save queue**, not wire concurrency; no second connection, which would contend for the logic lease. If the overhead gate (test d) fails at one save on the wire, the next step is a second stream dedicated to checkpoints under the same lease, a protocol minor version, taken then and measured, not now.
4. **What stays forced, unchanged.** Checkpoint at seat close and seal, at a resize (the parser state's dimensions change), at shutdown and supervisor handover, and the restart detection path of #11; the tick commit per tick and the 5 s or 16 MiB flushes of #21 are durability and are not touched by this policy.
5. **Watched seats are not privileged by this ruling.** Window 4 shows the 49 watched seats received 10 % of replay pages for 5 % of the seats and all progressed; no starvation. If Rook's and Aster's partition asks for watched priority or host-local ReadRing batching, that is a second ruling (R5h) with its own measurement, not a rider on this one; note for it now: watched priority needs an explicit GUI Watch, Unwatch and disconnect interest registry (`seat_socket.rs:528-538`), because Snapshot traffic is not a watch registry.

**Tests, and the gate.** (a) A fixture with one caught-up and one backlogged seat (20 MiB behind) records zero maintenance saves on the backlogged seat until it catches up. (b) A seat replaying 100 MiB produces forced saves at intervals of at most R and its debt never exceeds 2R. (c) **Crash recovery:** `SIGKILL` logic mid-replay under the firehose at the gate's seat count, restart, and every seat's **parsed debt** at the kill (replay from its confirmed checkpoint to where parsing stood) is at most 2R, with the backlog at the kill reported beside it, with the time until every seat is live measured and its budget written as median of three runs plus 25 %. (d) **Overhead:** in the 967-seat, 50-watched run the checkpoint RPC share of logic wall time is at most **10 %**, read from the R5 diagnostics that measured 70.6 %. (c) is the check of [[checkpoint-bounded]]; (d) joins the frame gate's diagnostics, since this stall is what blocks [[seats-967-frame]].

**Rejected.** Checkpointing by wall-clock period alone (the measured failure: it ignores backlog). No forced checkpoint under backlog (unbounded recovery work; a crash after an hour of catch-up replays the hour). Dropping strong commit on checkpoints (a torn checkpoint is worse than a late one). A checkpoint thread per seat (#21's reasoning: a thousand threads is not a design). Pausing all replay while any save is in flight (the stall again, with a different name).

Owner: Corvin Duskmantle <ARCHITECT>.
