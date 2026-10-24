---
id: "01M5MWWRW0XD4AC43Z9362QV98"
type: "decision"
number: 36
title: "Code adapted from Apache-only sources lives in bbt-core, never in a dual crate"
status: "decided"
decided: "2026-10-23"
operator: []
links:
  - { to: "[[2026-10-22-architecture-round-1#36]]", rel: "supersedes-as-index" }
  - { to: "[[licensing]]", rel: "decides" }
created: "2026-10-23T17:28:16Z"
updated: "2026-10-24T00:10:38Z"
---
# Decision 36: Code adapted from Apache-only sources lives in bbt-core, never in a dual crate

**Code adapted from Apache-only sources lives in `bbt-core`, never in a
dual crate (2026-10-23; from LIC1 and K1).** The kitty key encoder is a
port of alacritty `input/keyboard.rs build_sequence` at 5421470
(Apache-2.0 only), so it is `crates/bbt-core/src/keyboard.rs`, with the
upstream header and a "modified by bboxy450" line, and takes a neutral
key model (key, modifiers, text, state) that the app maps winit events
to, keeping bbt-core free of UI toolkit types. The app crate stays
MIT OR Apache-2.0 for its own code only. General rule for [[licensing]]:
an Apache-only adaptation joins the Apache-only terminal crate; a dual
or MIT adaptation may join a dual crate with its notice. Rejected: a
mixed SPDX expression on the app, "(MIT OR Apache-2.0) AND Apache-2.0"
(correct but opaque to every reader and tool); a new one-file Apache
crate (a crate per license is not a design); keeping the encoder in the
app under Apache alone (silently makes the app Apache-only).
