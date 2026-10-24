---
id: "01M5JFXASGXFSWJ18FXKGTFA7W"
type: "decision"
number: 2
title: "C1 binding: app IO thread + double buffer"
status: "decided"
decided: "2026-10-22"
operator: []
links:
  - { to: "[[2026-10-22-architecture-round-1#2]]", rel: "supersedes-as-index" }
  - { to: "[[runtime]]", rel: "decides" }
created: "2026-10-22T19:02:54Z"
updated: "2026-10-24T00:10:38Z"
---
# Decision 2: C1 binding: app IO thread + double buffer

**C1 binding: app IO thread + double buffer.** The IO thread owns the seat
socket and applies snapshot and deltas into a back-buffer model; the UI
thread swaps at frame start, never locks or waits ([[runtime]] §3). Logic
coalesces per seat to the latest state when the app lags, scheduled by
visibility; hidden seats send a state bit. Rejected: logic in-process; pull
per frame.
