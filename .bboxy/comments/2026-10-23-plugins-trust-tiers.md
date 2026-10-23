---
id: 01M5N5S768B3EFH0N08AMRF6W7
type: comment
title: "contracts carry different risk; trust them by tier"
status: resolved
author: name:Wren key:22c0e081a5f4ac21
on: "[[plugins]]"
on_digest: "sha256:4f59ec6a58dda3d6f387bcfb7020796b031417b322fe599e1cf1463141fc7abd"
anchor: "§1 Contracts"
kind: suggestion
links:
  - { to: "[[plugins]]", rel: on }
created: 2026-10-23T20:01:17Z
updated: 2026-10-23T20:13:06Z
---
# contracts carry different risk; trust them by tier

Red-team review of `.bboxy/specs/plugins.md` at 16b4c98 (decision #41, the RFC loop's first run). Author grade inferred: written outside a cockpit seat, signed only by the machine-key commit ([[work-items]] RFC loop, aee9fda).

The contracts are not equally dangerous. `provider` (the out-of-process form of D1) carries every remote seat's keystrokes and screen; `event_sink` exports envelopes, which may include message bodies, to an external bus; `data_source` can write to a knowledge base; `repo_graph` only reads. One trust prompt for all of them under-protects the first and over-asks for the last.

**Ask.** A trust tier per contract, written in the contract (for example read < write < export < transport), shown at trust time and in the registry; `event_sink` declares which subjects it receives and the app filters to that list; a `provider` plugin needs an explicit operator confirmation every time its digest changes.
