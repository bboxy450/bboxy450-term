---
id: 01M5N5S75ZGDQB87KH44AT9K0X
number: 176
type: comment
title: "a plugin accept bypasses the signed acceptance chain"
status: resolved
author: name:Wren key:22c0e081a5f4ac21
on: "[[plugins]]"
on_digest: "sha256:4f59ec6a58dda3d6f387bcfb7020796b031417b322fe599e1cf1463141fc7abd"
anchor: "§1 Contracts · task_system"
kind: objection
links:
  - { to: "[[plugins]]", rel: on }
created: 2026-10-23T20:01:17Z
updated: 2026-10-23T20:13:06Z
---
# a plugin accept bypasses the signed acceptance chain

Red-team review of `.bboxy/specs/plugins.md` at 16b4c98 (decision [[d41-plugins|#41]], the RFC loop's first run). Author grade inferred: written outside a cockpit seat, signed only by the machine-key commit ([[work-items]] RFC loop, aee9fda).

`task_system` lists `accept` and `reject` with "the same shapes as [[work-items]] and the W2.5d tools". In [[work-items]], acceptance is the app's judgement over evidence: `accepted` requires a recorded `verify_result` whose command equals the task's `verify` with exit 0, and every record is signed by the writer's seat leaf ("The spec signs off, not the agent", decision [[d28-structured-task-verification|#28]]). A Linear or Jira plugin implementing `accept` would mint acceptances with no signature, no verify_result and no seat, and the provenance chain ([[signed-main]] is its first link) would end at a third-party API.

**Ask.** Acceptance logic never crosses the contract. The app evaluates and signs; the plugin stores the signed record opaquely and returns it byte for byte; `accept` and `reject` in the contract carry an app-signed record, never a bare state change.
