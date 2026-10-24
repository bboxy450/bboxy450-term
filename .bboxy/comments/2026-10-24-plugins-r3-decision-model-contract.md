---
id: 01M5NVBXD3N9V5CMNGYYDA9MD5
number: 214
type: comment
title: "decision_model: which call sites, what timeout, what gets recorded?"
status: resolved
author: name:Seraph key:eec764b7d2f4b9b8
on: "[[plugins]]"
on_digest: "sha256:a3b7a2b411f819d0fbaa532d587847fe91891bf9a4b9b14e0c2419c9c65d9212"
anchor: "§0 Jev · §1 Contracts · decision_model"
kind: question
links:
  - { to: "[[plugins]]", rel: on }
created: 2026-10-24T02:20:49Z
updated: "2026-10-24T02:24:35Z"
---
# decision_model: which call sites, what timeout, what gets recorded?

Review of `.bboxy/specs/plugins.md` revision 3 at main 9c2022a7 (digest pinned above), fresh eyes from an idle Executor at the operator's request ("review plugin draft and send message to you with feedback!"). Author grade inferred: written outside a cockpit seat, signed only by the machine-key commit. Not repeated here: the 21 comments of 2026-10-23, all resolved.

The operator wants Jev "if it makes sense or we don't have to use it", and the design is "basically its deterministic way of dealing with plugins". §1 says "the places where the app picks among named options may ask the plugin" without naming one, and a hosted call has its own latency and failure modes.

**Questions.** Which call sites, first and exactly (launcher harness pick? reviewer selection?), and what the built-in rule is at each, so behaviour with and without the plugin is defined per site. A per-call timeout after which the built-in rule decides, recorded as such. Who owns a confidence gate (accept the plugin's choice only above a threshold, else the built-in), the contract or the call site. And provenance: the app records `choice`, `confidence`, `probabilities` and the provider's model id (`jev-1.13.0`, which kittens pins on purpose) with each decision, so a later replay explains why a route was taken.
