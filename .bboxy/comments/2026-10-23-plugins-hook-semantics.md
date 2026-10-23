---
id: 01M5N5S762KWNGFJZ8H8N88BXM
type: comment
title: "what may a before hook do?"
status: open
author: name:Wren key:22c0e081a5f4ac21
on: "[[plugins]]"
on_digest: "sha256:4f59ec6a58dda3d6f387bcfb7020796b031417b322fe599e1cf1463141fc7abd"
anchor: "§1 Hooks"
kind: question
links:
  - { to: "[[plugins]]", rel: on }
created: 2026-10-23T20:01:17Z
updated: 2026-10-23T20:01:17Z
---
# what may a before hook do?

Red-team review of `.bboxy/specs/plugins.md` at 16b4c98 (decision #41, the RFC loop's first run). Author grade inferred: written outside a cockpit seat, signed only by the machine-key commit ([[work-items]] RFC loop, aee9fda).

§1 Hooks gives `before`/`after` and a `budget_ms`, then "skipped after repeated overruns". Undefined: can a `before` hook block the action (Claude Code exit 2, [[plugins-sota]] :95) or change its input? Is it fail-open or fail-closed on error and on timeout? With several plugins on one event, serial or parallel, in what order, and does one plugin's block stop the rest? "Repeated" is how many, over what window, and how is a skipped hook re-enabled? Who measures the budget, the app's clock or the plugin's report?

**Ask.** A table per phase: allowed effects (observe, block with a reason, mutate), failure policy, ordering, skip and reset rule. The default should be observe-only and fail-open, with blocking as an operator-granted capability.
