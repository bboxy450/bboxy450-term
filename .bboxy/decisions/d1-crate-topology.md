---
id: "01M5JFXASG4FY1C4D4N2MYGFVD"
type: "decision"
number: 1
title: "Crate topology: split by process boundary"
status: "decided"
decided: "2026-10-22"
operator: []
links:
  - { to: "[[2026-10-22-architecture-round-1#1]]", rel: "supersedes-as-index" }
created: "2026-10-22T19:02:54Z"
updated: "2026-10-24T00:10:38Z"
---
# Decision 1: Crate topology: split by process boundary

**Crate topology: split by process boundary.** `bbt-supervisor` (mio, PTY
masters, leases, ring writer) leaves `bbt-mux`; everything tokio stays
(logic, store, bus, index, adapters, messaging, MCP). Crate line = process
line = crash domain = runtime. Rejected: one crate until a plateau; split by
domain (churns four in-flight tasks). When: after W2.4 merges, before W2.5.
