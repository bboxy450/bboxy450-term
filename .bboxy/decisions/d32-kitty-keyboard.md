---
id: "01M5MQPQCRYM1HJPTS6VQPVRK5"
type: "decision"
number: 32
title: "The kitty keyboard protocol is a core terminal invariant"
status: "decided"
decided: "2026-10-23"
operator:
  - "also for terminals we should adotp Kitty keyboard protocol!"
  - "document that as a core invariant of our terminal implementation"
links:
  - { to: "[[2026-10-22-architecture-round-1#32]]", rel: "supersedes-as-index" }
  - { to: "[[kitty-keyboard]]", rel: "constrains" }
  - { to: "[[terminal-core]]", rel: "decides" }
created: "2026-10-23T15:57:35Z"
updated: "2026-10-24T00:10:38Z"
---
# Decision 32: The kitty keyboard protocol is a core terminal invariant

**The kitty keyboard protocol is a core terminal invariant (operator,
2026-10-23, verbatim: "also for terminals we should adotp Kitty keyboard
protocol!", "document that as a core invariant of our terminal
implementation").** Recorded as [[kitty-keyboard]]: one app encoder
(`keyboard.rs`, a port of alacritty's `build_sequence`) encodes every
key against the focused seat's live `TermMode` from `Screen.modes`, full
progressive-enhancement stack with push/pop/query, legacy encoding when
no flags are set, no second key path to the PTY. Check: one named unit
test that must exist and pass (a cargo filter matching nothing exits 0,
so the check greps for exactly one passed). K1 (Sable) builds it;
[[terminal-core]] row updated. Rejected: encoding in logic (the mode is
live in the app's model and the PTY write already has one id-deduped
path); a second encoder for injected text (two encoders drift);
enabling the protocol without the invariant (a preference, not a gate).
