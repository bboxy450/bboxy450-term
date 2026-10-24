---
id: "01M5JGX4C0BM45XMS8BCK6MCZ0"
type: "decision"
number: 14
title: "The app reads its own .bboxy: the viewer is cockpit work"
status: "decided"
decided: "2026-10-22"
operator:
  - "I want to use our app to review our architecture and specs and docs."
links:
  - { to: "[[2026-10-22-architecture-round-1#14]]", rel: "supersedes-as-index" }
  - { to: "[[document-viewer]]", rel: "decides" }
  - { to: "[[2026-10-22-c-cockpit]]", rel: "reference" }
created: "2026-10-22T19:20:16Z"
updated: "2026-10-24T00:10:38Z"
---
# Decision 14: The app reads its own .bboxy: the viewer is cockpit work

**The app reads its own `.bboxy`: the viewer is cockpit work (operator).**
"I want to use our app to review our architecture and specs and docs."
[[document-viewer]] (W3.5, `bbt-doc`: Markdown plus our own Mermaid
engine) moves from not-yet-briefed into [[2026-10-22-c-cockpit]] as a
track; the 7.10 DocumentModal is its L7 face. Harnesses write tasks,
epics, specs and decisions as Markdown through the task tools (W2.5) and
everyone, the operator first, reads them in the app. The Mermaid subset to
build first is what `.bboxy` uses today: `flowchart` LR, TB, BT, TD with
subgraphs, `&` fan-out, dotted edges, cylinder and stadium shapes,
`classDef` dashed; `sequenceDiagram` with participants, `loop`, dashed
replies. Wikilinks resolve to documents in the viewer. Rejected: a
browser-based viewer (not our shape); waiting for L9.
