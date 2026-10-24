---
id: 01M5N5S763QHW7VP0PRFWRWT7Y
number: 163
type: comment
title: "no process or crate owns the plugin host"
status: resolved
author: name:Wren key:22c0e081a5f4ac21
on: "[[plugins]]"
on_digest: "sha256:4f59ec6a58dda3d6f387bcfb7020796b031417b322fe599e1cf1463141fc7abd"
anchor: "§1 Shape of the system"
kind: objection
links:
  - { to: "[[plugins]]", rel: on }
created: 2026-10-23T20:01:17Z
updated: 2026-10-23T20:13:06Z
---
# no process or crate owns the plugin host

Red-team review of `.bboxy/specs/plugins.md` at 16b4c98 (decision [[d41-plugins|#41]], the RFC loop's first run). Author grade inferred: written outside a cockpit seat, signed only by the machine-key commit ([[work-items]] RFC loop, aee9fda).

The diagram has "logic" calling plugins and "Settings › Plugins" showing them; the spec never says which process spawns them or which crate holds the registry and the contracts. Three rules care. [[ui-never-waits]] forbids any process or socket call in the `src/` of the UI crates and the app, so the app cannot spawn or talk to plugins. [[layering]] requires a new crate to take its place in the order in the commit that adds it. Decision [[d39-suite-is-the-product|#39]] makes the app glue and the kit reusable.

**Ask.** State it: the host (discovery, spawn, protocol, registry, timings) lives in the logic process; the contract and shape schemas live in a kit crate a second suite app can depend on, named with its layering position; Settings reads the registry over the bus like every other view.
