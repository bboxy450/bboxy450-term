---
id: "01M5KWRHHRV8TP65ST6CGTZDM6"
type: "decision"
number: 26
title: "Terminals own every plain key"
status: "decided"
decided: "2026-10-23"
operator: []
links:
  - { to: "[[2026-10-22-architecture-round-1#26]]", rel: "supersedes-as-index" }
  - { to: "[[gui-components]]", rel: "decides" }
created: "2026-10-23T08:06:43Z"
updated: "2026-10-24T00:10:38Z"
---
# Decision 26: Terminals own every plain key

**Terminals own every plain key (2026-10-23, Oracle finding in
[[gui-components]] §4.4).** With a terminal pane focused, Tab, arrows
and every other plain key go to the PTY; pane navigation is ⌘⌥ arrows
(spatial) and ⌘⌥ Tab (tree order); plain Tab and arrows navigate only
in the launcher, overlays, text fields and app chrome. Rejected: plain
Tab/arrows for pane navigation (steals them from shells and TUIs);
a modal "navigation mode" (one more state the operator must notice).
