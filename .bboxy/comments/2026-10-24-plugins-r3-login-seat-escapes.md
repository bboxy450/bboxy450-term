---
id: 01M5NVBXDAW3G9F9MEJJWR3Z3S
number: 217
type: comment
title: "the bare login seat still parses OSC 7501 and OSC 52"
status: resolved
author: name:Seraph key:eec764b7d2f4b9b8
on: "[[plugins]]"
on_digest: "sha256:a3b7a2b411f819d0fbaa532d587847fe91891bf9a4b9b14e0c2419c9c65d9212"
anchor: "§1 Protocol · bare login seat"
kind: objection
links:
  - { to: "[[plugins]]", rel: on }
created: 2026-10-24T02:20:49Z
updated: "2026-10-24T02:24:35Z"
---
# the bare login seat still parses OSC 7501 and OSC 52

Review of `.bboxy/specs/plugins.md` revision 3 at main 9c2022a7 (digest pinned above), fresh eyes from an idle Executor at the operator's request ("review plugin draft and send message to you with feedback!"). Author grade inferred: written outside a cockpit seat, signed only by the machine-key commit. Not repeated here: the 21 comments of 2026-10-23, all resolved.

The bare login seat is "a plain PTY with no MCP, no hooks, no skills" (§1 Protocol). But a plain PTY in this app still interprets what the program writes. OSC 7501 is parsed from output as `reported`-grade seat state, and it wins over hooks (`crates/bbt-mux/src/adapters/observe.rs:221`; listed for every adapter, including the default for unknown programs, `crates/bbt-mux/src/adapters/mod.rs:196`; [[harness-adapters]] R1). OSC 52 writes the operator's clipboard (`crates/bbt-core/src/vendor/alacritty/term/mod.rs:359`). An untrusted login command could set a seat to `waiting` (and, with attention, trigger the swell and a desktop notification) or put a crafted command on the clipboard.

**Ask.** The bare login seat has no adapter, so no 7501, 7502 or 21337 observation. OSC 52 writes are disabled, and so are title and notification escapes. Its row is visibly marked as a plugin login, not a harness seat.
