---
id: 01M5N5S764JKVWF5SHMBGZYBGE
type: comment
title: "a flame graph of an opaque subprocess has no data source"
status: open
author: name:Wren key:22c0e081a5f4ac21
on: "[[plugins]]"
on_digest: "sha256:4f59ec6a58dda3d6f387bcfb7020796b031417b322fe599e1cf1463141fc7abd"
anchor: "§1 Registry"
kind: objection
links:
  - { to: "[[plugins]]", rel: on }
created: 2026-10-23T20:01:17Z
updated: 2026-10-23T20:01:17Z
---
# a flame graph of an opaque subprocess has no data source

Red-team review of `.bboxy/specs/plugins.md` at 16b4c98 (decision #41, the RFC loop's first run). Author grade inferred: written outside a cockpit seat, signed only by the machine-key commit ([[work-items]] RFC loop, aee9fda).

The registry promises "per-call and per-hook timings and a flame graph". The research is explicit: the host sees wall time per call and the internals are opaque (:116), and "No plugin registry found that renders per-hook flame graphs. Gap: no data" (:103). Without a span channel in the protocol, a "flame graph" can only show the host's own nesting of calls and hooks, which is a timeline, not a profile of the plugin.

**Ask.** Either add an optional `trace` capability to the shape (the plugin emits spans with ids and parent ids over the protocol and the host merges them under its call span), or name the view honestly as a call and hook timeline and drop "flame graph".
