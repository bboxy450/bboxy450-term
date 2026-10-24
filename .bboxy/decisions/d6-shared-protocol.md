---
id: "01M5JFXASG70K1W5GEE486VZHK"
type: "decision"
number: 6
title: "Domain types go to a shared protocol crate"
status: "decided"
decided: "2026-10-22"
operator: []
links:
  - { to: "[[2026-10-22-architecture-round-1#6]]", rel: "supersedes-as-index" }
  - { to: "[[open-source]]", rel: "decides" }
created: "2026-10-22T19:02:54Z"
updated: "2026-10-24T00:10:38Z"
---
# Decision 6: Domain types go to a shared protocol crate

**Domain types go to a shared protocol crate.** Private `bbt-proto`: seat,
swarm, tab, split, ids, snapshot and delta wire types, used by logic and app;
the seat socket carries exactly these. bbt-ui-model keeps generic layout,
draw, tokens, events. L7 modules still move to bbt-term-ui ([[open-source]]).
Rejected: a bbt-model crate on top of ui-model (converts twice); feature flag.
When: with the C1 spec; the ui-model split waits for 7.10 as planned.
