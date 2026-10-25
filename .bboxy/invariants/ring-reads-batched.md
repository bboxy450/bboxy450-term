---
id: 01M5S7QYBHREREW0FAJMZ7FAE2
type: invariant
number: 318
title: "ring reads are batched: a catch-up pass over N seats costs a bounded number of supervisor round trips, never one RPC per seat"
status: pending
check: cargo test -p bbt-mux --locked --test logic ring_reads_are_batched -- --exact 2>&1 | grep -q 'ok. 1 passed; 0 failed'
cost: with hundreds of seats every catch-up pass serialises hundreds of one-seat round trips through the supervisor, input-to-GPU latency grows with the seat count instead of the page budget, and the watched panes wait behind the unwatched ones' RPCs
links:
  - { to: "[[no-chokepoints]]", rel: instance-of }
  - { to: "[[d253-checkpoint-policy]]", rel: builds-on }
  - { to: "[[watched-replay-priority]]", rel: builds-on }
  - { to: "[[seats-967-frame]]", rel: unblocks }
  - { to: "[[mux-daemon]]", rel: constrains }
created: "2026-10-25T09:54:49Z"
updated: "2026-10-25T09:54:49Z"
---
# Ring reads are batched

**Property.** Logic's ring read (`ReadRing`) toward the supervisor is batched or
pipelined: one catch-up pass over N seats with pending ring data costs a number
of supervisor round trips bounded by the page budget and the lane quota of
[[d253-checkpoint-policy]], not by N; a request carries many seats, replies may
be in flight at once, and no seat's read waits on another seat's reply having
returned. The 6 824 serial RPCs that motivated this are evidence; the gate is a
round-trip count under a fixed fixture, because a count does not depend on the
machine.

**Protocol shape.** *Open: filled from Sable's design when it is accepted.* The
request and reply forms, the batch bound, how the watched and background lanes
of [[watched-replay-priority]] share a batch, and the error form for a seat
that is gone mid-batch go here, in words, before the test is named final.

**Check.** `cargo test -p bbt-mux --locked --test logic ring_reads_are_batched -- --exact 2>&1 | grep -q 'ok. 1 passed; 0 failed'`
— exit 0 held. **Pending** until Sable's batched or pipelined `ReadRing` lands
with that test (a fixture of many seats with pending data asserting the
round-trip count against the bound); if the landing commit names the test
differently it edits this line and the README row in the same commit, and the
Oracle flips `status` on acceptance. Until then the gate is listed as pending
and runs nowhere.

**Cost of a violation.** With hundreds of seats every catch-up pass serialises
hundreds of one-seat round trips through the supervisor; input-to-GPU latency
grows with the seat count instead of the page budget, and the watched panes
wait behind the unwatched ones' RPCs. The seventh known instance of
[[no-chokepoints]]: one serial path everything reads through.

**Evidence.** The measured 967/50 run on 0e97dc32 (Rook, 2026-10-25, Nyx's R5i
diagnostics on): 6 824 serial one-seat `ReadRing` RPCs to the supervisor,
93 % of an input-to-GPU of 8 618 ms, while decoding the data itself took
0.8 s. Reported by the Oracle to the Architect the same day.

Owner: Corvin Duskmantle <ARCHITECT>.
