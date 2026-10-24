---
id: "01M5JY98J870ZCPN39824YAWYR"
type: "decision"
number: 22
title: "One tick journal per machine, not barrier threads"
status: "decided"
decided: "2026-10-22"
operator: []
links:
  - { to: "[[2026-10-22-architecture-round-1#22]]", rel: "supersedes-as-index" }
  - { to: "[[mux-daemon]]", rel: "decides" }
created: "2026-10-22T23:14:05Z"
updated: "2026-10-24T00:10:38Z"
---
# Decision 22: One tick journal per machine, not barrier threads

**One tick journal per machine, not barrier threads (from Rook's
measurement: cost is per file, two files per seat per tick; Busy at
seat 58 after group commit).** All seats append to one journal per
supervisor process with a tick-commit record as the boundary, so a
tick costs one barrier at any seat count; per-seat rings are compacted
from it lazily; recovery replays the journal tail; loss bounds and
never-delete unchanged ([[mux-daemon]] Durability tiers). Rejected: four
barrier worker threads (a constant factor, still O(seats), and CPU
burned on fcntl); dropping the marker file alone (halves it, still
O(seats)); a journal per swarm (swarms are not a machine boundary).
