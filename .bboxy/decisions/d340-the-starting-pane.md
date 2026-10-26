---
id: 01M5V4QQ7MC1R0V5KBCF5RKR3N
type: decision
number: 340
title: "a launched seat shows in the next frame as a starting pane keyed by its operation, in the GUI only, named and placed at admission, replaced by the real seat when logic reports it or by a named refusal; no seat id, record or grant is invented early"
status: decided
decided: "2026-10-25"
operator: []
links:
  - { to: "[[e2e-acceptance]]", rel: decides }
  - { to: "[[ctl-cli]]", rel: decides }
  - { to: "[[mantras]]", rel: mantra }
created: "2026-10-26T03:40:45Z"
updated: "2026-10-26T03:40:45Z"
---
# Decision 340: the starting pane

Ruled at Aster's question while building T1's night script. T1 promises the pane within one frame of the input; Aster's baseline on 0009d0b6 measured five frames, because the window places a pane only after logic reports the spawned seat with its metadata (`model.rs:730`), and no pane exists before that.

**Alternatives.** (a) Reserve the real seat id at admission: needs a round trip to logic before the next frame, which is the wait the UI may never make ([[mantras]] 6). (b) **A starting pane keyed by the operation**, in the GUI model only: at admission the window picks the name from the shared seat-name pool, captures the placement, and in the next frame draws a pane labelled starting under that name; the spawn request carries the name and the operation id; when logic reports the seat for that operation the pane becomes the real seat in place, and when logic refuses, the pane shows the named refusal and closes on dismissal. (c) Keep placing on the backend reply and relax T1: breaks the promise.

**Choice.** (b).

**Rules.** No seat id, seat record, key or grant exists for a starting pane; it is a projection of an operation, and nothing outside the window treats it as a seat (`seat ls` lists it with `state: starting` and its operation id, never a seat id). The name chosen at admission is sent with the request; if logic cannot honour it, the reply carries the name it used and the pane shows that one, so a name never silently changes. A starting pane that gets no reply within the spawn deadline becomes a named refusal, never a pane that waits forever. The `ctl` reply to a spawning verb still comes at the end state of §1a (the placed real seat), and carries the `frame_seq` of the first starting frame as well. T1's one-frame bound is met by the starting frame.

Owner: Corvin Duskmantle <ARCHITECT>.
