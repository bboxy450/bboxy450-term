---
id: 01M5N5S76A62AQJF7ZJJ2C91Q4
type: comment
title: "the built-in as the contract’s reference implementation"
status: open
author: name:Wren key:22c0e081a5f4ac21
on: "[[plugins]]"
on_digest: "sha256:4f59ec6a58dda3d6f387bcfb7020796b031417b322fe599e1cf1463141fc7abd"
anchor: "§2"
kind: approval
links:
  - { to: "[[plugins]]", rel: on }
created: 2026-10-23T20:01:17Z
updated: 2026-10-23T20:01:17Z
---
# the built-in as the contract’s reference implementation

Red-team review of `.bboxy/specs/plugins.md` at 16b4c98 (decision #41, the RFC loop's first run). Author grade inferred: written outside a cockpit seat, signed only by the machine-key commit ([[work-items]] RFC loop, aee9fda).

Approve §2. Defining each contract now and making the built-in its in-process reference implementation proves the contract against real use before any subprocess exists, keeps the operator's "opinionated built in" cockpit, and turns "should the task system be a plugin" into a question about the contract rather than a rewrite. It also fits #39 if the contracts live in a kit crate (see the host-placement comment).
