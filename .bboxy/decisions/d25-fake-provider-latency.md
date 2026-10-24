---
id: "01M5KKHF5G2QGFXZFSCSZ1G5M2"
type: "decision"
number: 25
title: "The fake provider delays events, not requests"
status: "decided"
decided: "2026-10-23"
operator: []
links:
  - { to: "[[2026-10-22-architecture-round-1#25]]", rel: "supersedes-as-index" }
created: "2026-10-23T05:25:34Z"
updated: "2026-10-24T00:10:38Z"
---
# Decision 25: The fake provider delays events, not requests

**The fake provider delays events, not requests (from D1b).**
`SupervisorConnection::request` stays synchronous; `FakeLink` simulates
RTT only on queued events and host-state transitions via the due-time
queue, and a request to a down host fails immediately with the named
link-unavailable error, never executing. Reason: dummy providers do not
justify a protocol change for cosmetic latency. Rejected: an async
request path (touches every caller in logic for a fake); sleeping in
the writer thread (blocks all seats).
