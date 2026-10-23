---
id: 01M5N5S75X3BS2QFP0YQPQ8WDR
type: comment
title: "bare run as the shape contradicts decision #12"
status: open
author: name:Wren key:22c0e081a5f4ac21
on: "[[plugins]]"
on_digest: "sha256:4f59ec6a58dda3d6f387bcfb7020796b031417b322fe599e1cf1463141fc7abd"
anchor: "§1 Shape"
kind: objection
links:
  - { to: "[[plugins]]", rel: on }
created: 2026-10-23T20:01:17Z
updated: 2026-10-23T20:01:17Z
---
# bare run as the shape contradicts decision #12

Red-team review of `.bboxy/specs/plugins.md` at 16b4c98 (decision #41, the RFC loop's first run). Author grade inferred: written outside a cockpit seat, signed only by the machine-key commit ([[work-items]] RFC loop, aee9fda).

§1 Shape: "Running the binary with no arguments prints one JSON document". Decision #12 recorded `binary --shape` for exactly this. Precedent goes the same way: Docker uses a dedicated metadata subcommand, and go-plugin's bare run deliberately refuses ([[plugins-sota]] :19, :43). A bare-run shape also forecloses a plugin that is a usable CLI in its own right (a Linear or NATS tool that does something when run bare), and makes an accidental run (double-click, a `$PATH` collision) silently do protocol work.

**Ask.** Keep #12: `--shape` (or one reserved subcommand); bare-run behaviour unspecified and never relied on. If the operator's verbatim "if we run just binary it outputs json shape" is meant literally, record that #41 supersedes #12 on this point.
