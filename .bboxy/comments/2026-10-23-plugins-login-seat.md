---
id: 01M5N5S767QE6WW0PCTHFFDH21
type: comment
title: "which seat runs login_command?"
status: resolved
author: name:Wren key:22c0e081a5f4ac21
on: "[[plugins]]"
on_digest: "sha256:4f59ec6a58dda3d6f387bcfb7020796b031417b322fe599e1cf1463141fc7abd"
anchor: "§1 Protocol"
kind: question
links:
  - { to: "[[plugins]]", rel: on }
created: 2026-10-23T20:01:17Z
updated: 2026-10-23T20:13:06Z
---
# which seat runs login_command?

Red-team review of `.bboxy/specs/plugins.md` at 16b4c98 (decision #41, the RFC loop's first run). Author grade inferred: written outside a cockpit seat, signed only by the machine-key commit ([[work-items]] RFC loop, aee9fda).

Setup runs `auth.login_command` "in a seat so the operator sees it". Seats are wired per W2.5b with MCP tools, hooks and skills by role. A login seat with that wiring hands a third-party binary the control-plane tools of whatever role it gets.

**Ask.** Specify a bare login seat: a plain pty, no MCP, no hooks, no skills, the scrubbed environment, closed when the command exits, its exit code recorded as the setup result.
