---
id: "01M5JVKHD0GE6B1SM2SC3SQSXN"
type: "decision"
number: 21
title: "Ring durability has two tiers"
status: "decided"
decided: "2026-10-22"
operator: []
links:
  - { to: "[[2026-10-22-architecture-round-1#21]]", rel: "supersedes-as-index" }
  - { to: "[[mux-daemon]]", rel: "decides" }
created: "2026-10-22T22:27:16Z"
updated: "2026-10-24T00:10:38Z"
---
# Decision 21: Ring durability has two tiers

**Ring durability has two tiers (from the C1d measurement: logic dies at
~45 seats).** Tick commits are ordered with a barrier, not flushed;
full flushes happen at checkpoint, seal, shutdown and every 5 s or
16 MiB per machine. Loss bounds stated in [[mux-daemon]] Durability
tiers: crash ≤ one tick per seat of admitted-minus-committed bytes, the
pre-capture PTY tail reported separately (ruled 2026-10-22 when Rook's
check showed the first wording counted kernel-buffer bytes the ring
never had); power cut ≤ 5 s or 16 MiB by an admission rule that flushes
at 16 MiB undurable; never a torn record. Busy is backpressure to the PTY, never fatal. Rejected:
full flush per tick (the measured failure); no flush except shutdown
(unbounded power-cut loss); a writer thread per seat (1000 threads is
not a design). The numbers bind when W1.2d's gate measures them.
