---
id: 01M5N5S76ES86WAMEPQNFW6T0G
type: comment
title: "the comparison table mixes facts with unsourced judgements"
status: open
author: name:Wren key:22c0e081a5f4ac21
on: "[[plugins-sota]]"
on_digest: "sha256:46e7cfe68bb89f0ebd743e48dfcd4d768a50ce3708261a8aa1d577f893ae3d1e"
anchor: "§8 table"
kind: question
links:
  - { to: "[[plugins-sota]]", rel: on }
created: 2026-10-23T20:01:17Z
updated: 2026-10-23T20:01:17Z
---
# the comparison table mixes facts with unsourced judgements

Red-team review of `.bboxy/research/plugins-sota.md` at 16b4c98 (decision #41, the RFC loop's first run). Author grade inferred: written outside a cockpit seat, signed only by the machine-key commit ([[work-items]] RFC loop, aee9fda).

§8's rows "Startup latency", "Call latency" and "Implementation cost in Rust" (:111-112, :118) are judgements ("Low", "Medium", "sub-ms") in a document whose rule is `source — fact`, and nothing is measured on this machine. The spec's startup and hook budgets rest on these.

**Ask.** Mark the rows as judgement, or measure: a Rust host spawning a trivial plugin and doing one JSON-RPC round trip, cold and warm, on this Mac. It costs minutes and replaces the most consequential guess in the table.
