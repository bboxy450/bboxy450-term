---
id: 01M5N5S761DE0DN77WS5KC8GBS
number: 166
type: comment
title: "tool names collide and the plugin picks its own role class"
status: resolved
author: name:Wren key:22c0e081a5f4ac21
on: "[[plugins]]"
on_digest: "sha256:4f59ec6a58dda3d6f387bcfb7020796b031417b322fe599e1cf1463141fc7abd"
anchor: "§1 MCP"
kind: objection
links:
  - { to: "[[plugins]]", rel: on }
created: 2026-10-23T20:01:17Z
updated: 2026-10-23T20:13:06Z
---
# tool names collide and the plugin picks its own role class

Red-team review of `.bboxy/specs/plugins.md` at 16b4c98 (decision [[d41-plugins|#41]], the RFC loop's first run). Author grade inferred: written outside a cockpit seat, signed only by the machine-key commit ([[work-items]] RFC loop, aee9fda).

Tools are `plugin_<provider>_<command>` "gated by the same role classes as every other tool". Two problems. (1) Collisions: one provider in two systems (`…-task_system-linear`, `…-data_source-linear`) yields the same `plugin_linear_search`, and the same plugin in both discovery roots shadows silently (Docker keeps `ShadowedPaths`, [[plugins-sota]] :21). (2) Nothing says who assigns a command's class. If it comes from the shape, a plugin can declare its own write as a read, which is self-escalation.

**Ask.** Names include the system (`plugin_<system>_<provider>_<command>`); the **contract** fixes each method's role class and the shape cannot change it; a shadowed name is listed and refused, never resolved by order.
