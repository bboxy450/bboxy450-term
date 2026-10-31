---
id: 01M5S7QYBHREREW0FAJMZ7FAE2
type: invariant
number: 318
title: "seat ring reads progress independently without a shared completion barrier"
status: active
check: cargo test -p bboxy450-term-mux --locked --lib logic::ring_batch_tests::held_seat_does_not_block_other_seat -- --exact 2>&1 | grep -q 'ok. 1 passed; 0 failed'
cost: with hundreds of seats every catch-up pass serialises hundreds of one-seat round trips through the supervisor, input-to-GPU latency grows with the seat count instead of the page budget, and the watched panes wait behind the unwatched ones' RPCs
links:
  - { to: "[[no-chokepoints]]", rel: instance-of }
  - { to: "[[d253-checkpoint-policy]]", rel: builds-on }
  - { to: "[[watched-replay-priority]]", rel: builds-on }
  - { to: "[[seats-967-frame]]", rel: unblocks }
  - { to: "[[mux-daemon]]", rel: constrains }
created: "2026-10-25T09:54:49Z"
updated: "2026-10-31T18:29:00Z"
---
# Seat ring reads progress independently

**Operator amendment, #47, 2026-10-30.** A single shared ingestion path is
forbidden. This supersedes the original aggregate-batch request-count property
of #318: a slow aggregate member used to delay up to seven other seats. Keeping
that coupling would violate the new direct brief. The existing bounded batched
wire API remains compatible; hot reads now use independently correlated members.

**Property.** Each seat owns at most one pending ring read and one parser job;
a held or flooded seat cannot hold another seat's completion or advance its
cursor. Successful publication consumes exactly the next ordered capture prefix
under the same attachment and owner incarnation. Ring-backed coalescing preserves
the unapplied tail. GUI/MCP history has its own one-page-per-seat completion lane.
The existing 3:1 fair admission and checkpoint/debt limits remain authoritative.
No read, history page or checkpoint completion is awaited under the link mutex.

**Check.** The frontmatter's native held-seat regression must exit 0. Companion
`make test-seat-ingestion` checks held history and its admission bound, independent
socket completion order, stale parser ownership and exact serial Screens/cursors
for 967 seats on one and two hosts. `make test-formal` checks SeatIngestion and
the deliberately shared-queue liveness failure in both native and pinned Java.
Model fairness and implementation-only bounds are in `verification/contracts.toml`.

**Limits.** This is per-seat read/parse isolation, not a CPU deadline, arbitrary
Rust refinement or independent-filesystem proof. Cross-seat metadata/publication
stays on logic; storage service and the OS remain shared dependencies. Quiet
`make profile-ingestion` retains the unchanged latency verdict; #47 stays open
until that measurement passes.

**Prior evidence.** #318 at 00fb087c reduced 6,824 serial round trips to 121
aggregate requests for 967 seats. That count proved the former batching
property, not independent completion. The #47 fresh-main medians on c03d9cb8
were 1075.230208 ms Written→LogicOutputRead and 1224.975042 ms input→GPU, with
load1 below4 at each start and no functional fixture failures. Measurements and
source/report hashes are recorded in `.bboxy/runs/ingest-47.log`.

Owner: Corvin Duskmantle <ARCHITECT>; amendment implemented under the direct
operator Executor brief, with decisions in the signed implementation commit.
