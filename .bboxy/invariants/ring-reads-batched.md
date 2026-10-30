---
id: 01M5S7QYBHREREW0FAJMZ7FAE2
type: invariant
number: 318
title: "ring reads are batched: a catch-up pass over N seats costs a bounded number of supervisor round trips, never one RPC per seat"
status: active
check: cargo test -p bboxy450-term-mux --locked --lib logic::ring_batch_tests::ring_reads_are_batched -- --exact 2>&1 | grep -q 'ok. 1 passed; 0 failed'
cost: with hundreds of seats every catch-up pass serialises hundreds of one-seat round trips through the supervisor, input-to-GPU latency grows with the seat count instead of the page budget, and the watched panes wait behind the unwatched ones' RPCs
links:
  - { to: "[[no-chokepoints]]", rel: instance-of }
  - { to: "[[d253-checkpoint-policy]]", rel: builds-on }
  - { to: "[[watched-replay-priority]]", rel: builds-on }
  - { to: "[[seats-967-frame]]", rel: unblocks }
  - { to: "[[mux-daemon]]", rel: constrains }
created: "2026-10-25T09:54:49Z"
updated: "2026-10-25T14:36:14Z"
---
# Ring reads are batched

**Property.** Logic's ring read toward the supervisor is batched: one catch-up
pass over N seats with pending ring data costs at most `ceil(N / 8)` supervisor
round trips, never N (967 seats: at most 121 requests, zero singleton calls
from the hot sweep); a request carries up to eight seats, every member is
admitted before any waiting, a refused member fails alone, never preventing the
others' admission, reply or application and never advancing its own cursor,
and the final cursors and Screens are exactly the ones the singleton path
produced. Across hosts the bound is the sum of `ceil(N_host / 8)`. **One
aggregate reply per batch is the design**, not a compromise: the group returns
after its last member, so a slow member delays at most seven others by one
read, a coupling bounded by the member count and `max_bytes`; the property is
about round trips, failure independence and exactness, not about a completed
member being visible before the slowest finishes. Streamed replies would be a
new ruling with its own reply-reserve rule, taken only if a measurement shows
the aggregate wait exceeding the page budget (Sable's question, 2026-10-25). The 6 824 serial RPCs that motivated this are evidence; the gate is a
round-trip count under a fixed fixture, because a count does not depend on the
machine.

**Protocol shape (Sable's design, the Oracle's ruling, 2026-10-25).** One new
request and one new reply beside the existing singleton: `ReadRingBatch {
reads: [(attachment, from, to)], max_bytes }` to the supervisor and
`RingHistoryBatch { results: [(seat, generation, from, to, Result<History,
Failure>)] }` back. A batch has at most **8 members** and at most **768 KiB
encoded**; every member is admitted before any waiting begins; each member
fails on its own with `Gone`, `Stale`, `Busy` or `Limit`, so a batch never
carries an oversize frame and never advances a refused member's cursor; a batch
is charged against the existing 128 pending-IO slots, not a new pool. The
reply is one aggregate per batch, coupling bounded by the 8; streamed replies
only if a measurement asks for them. Logic prefetches one bounded group per
host in the 3 : 1 lane order of [[watched-replay-priority]] and still applies
one page per turn. The singleton `ReadRing` stays for explicit history and
recovery reads and leaves the hot sweep.

**Check.** `cargo test -p bboxy450-term-mux --locked --lib logic::ring_batch_tests::ring_reads_are_batched -- --exact 2>&1 | grep -q 'ok. 1 passed; 0 failed'`
— exit 0 held. **Active (Sable, landed 00fb087c, 2026-10-25).** The batched read landed
with its test at `crates/bbt-mux/src/logic/ring_batch_tests.rs:274`, and a
per-host companion beside it: 967 seats cost 121 batched requests and zero
singleton hot-sweep reads, the bound of the property met exactly. The landing
commit renamed the check to the test's real path and removed the skip flags;
the Architect flipped the status on the Oracle's landing note, since the
landing left it pending without flags, which `scripts/pending-flags.sh`
refuses.

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
