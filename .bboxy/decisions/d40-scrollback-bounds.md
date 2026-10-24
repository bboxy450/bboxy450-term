---
id: "01M5N2PN3R12VSRH2SEE0V4Z5E"
type: "decision"
number: 40
title: "Scrollback replay bounds"
status: "decided"
decided: "2026-10-23"
operator: []
links:
  - { to: "[[2026-10-22-architecture-round-1#40]]", rel: "supersedes-as-index" }
created: "2026-10-23T19:09:47Z"
updated: "2026-10-24T00:10:38Z"
---
# Decision 40: Scrollback replay bounds

**Scrollback replay bounds (2026-10-23, C1c-3, Sable's two questions).**
(1) The resident parsed window rolls; a seek outside it re-replays from
the nearest ring checkpoint before the target, so any depth is
reachable and "top is line 1" is by re-replay, not residency. (2) The
4 MB cap is total heap of the history Playback per watched seat with a
dimension-aware line cap, floor three viewports, ceiling 10k; the
vision's "10k lines or 4 MB" is the raw ring's retention, a different
thing. A narrow bounded-storage and scroll-row API goes into bbt-core
(its own code, Apache-only crate) because every app with scrollback
needs it ([[d39-suite-is-the-product|#39]]), no wire or renderer change. (c) The replay anchor is
the capture start at current dimensions, the wire having no checkpoint
or resize payload; checkpoints on the wire only if the 20k-line seek
cost measures bad. (d) The 4 MB hard cap beats the three-viewport
floor: an unfit dimension returns `history_viewport_exceeds_budget`,
live unaffected (Oracle, same day). Rejected: an
oldest-accessible limit at 10k lines (the ring has the bytes, refusing
to show them is a product bug); counting raw bytes only (the grid is
the heap that matters); the API in the app (it is terminal model, not
glue).
(e) Architect's read of Sable's C1c-3 at 6c6d06d (2026-10-23): the
cap is 4 000 000 bytes (`scrollback.rs:14`), 10k lines (`:15`), the
three-viewport floor refuses by name (`:219-221`), restart replays
from capture start at the pane's template dimensions (`:283-297`),
pages release on live (`:293, 352, 457`), `bbt-core` carries the
bounded API (`lib.rs:111, 129`, `Term::new_with_spare_rows`), the 20k
fixture joins live exactly (`tests/scrollback.rs:115`); the measured
20k seek (54.9 / 131.7 ms) keeps checkpoints off the wire. One
divergence: a 512-column / 256-row bound refused under the budget
name (`:202-208`); ruled: a refusal name never lies, so the bound is
removed or renamed `history_dimensions_unsupported` with its reason
in the spec; Sable removed it the same evening, heap accounting is the
only admission rule, and `history_dimensions_unsupported` names only
`bbt-core`'s own minima (`lib.rs:67-75`). The 50-seat memory acceptance is met by `Playback`
accounting (53.76 MB conservative of 200 MB); RSS and timing at 50
watched belong to gate 4.
