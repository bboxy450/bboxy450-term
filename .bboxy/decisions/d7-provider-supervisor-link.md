---
id: "01M5JFXASGF86TA6AHDA5NW9FE"
type: "decision"
number: 7
title: "Provider yields a supervisor link"
status: "decided"
decided: "2026-10-22"
operator: []
links:
  - { to: "[[2026-10-22-architecture-round-1#7]]", rel: "supersedes-as-index" }
created: "2026-10-22T19:02:54Z"
updated: "2026-10-24T00:10:38Z"
---
# Decision 7: Provider yields a supervisor link

**Provider yields a supervisor link.** `Provider` produces a SupervisorLink
per host: LocalProvider returns the unix socket; FakeProvider wraps the local
supervisor in a scripted link (connect, latency, drop, reconnect from
fixtures); SshProvider later bootstraps a supervisor over ssh and tunnels the
same protocol. Seats, rings and adapters never know where they run.
Rejected: provider spawns seats (duplicates the protocol); data only.
