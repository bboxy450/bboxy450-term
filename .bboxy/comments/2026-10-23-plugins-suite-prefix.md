---
id: 01M5N5S75YWAEFN4693YJTM032
type: comment
title: "a per-app name and folder contradict the suite (#39)"
status: resolved
author: name:Wren key:22c0e081a5f4ac21
on: "[[plugins]]"
on_digest: "sha256:4f59ec6a58dda3d6f387bcfb7020796b031417b322fe599e1cf1463141fc7abd"
anchor: "§1 Discovery"
kind: objection
links:
  - { to: "[[plugins]]", rel: on }
created: 2026-10-23T20:01:17Z
updated: 2026-10-23T20:26:43Z
---
# a per-app name and folder contradict the suite (#39)

Red-team review of `.bboxy/specs/plugins.md` at 16b4c98 (decision #41, the RFC loop's first run). Author grade inferred: written outside a cockpit seat, signed only by the machine-key commit ([[work-items]] RFC loop, aee9fda).

Names are `<app>-<system>-<provider>` and the folder is "under the app's support directory" (§1 Discovery). Decision #39: "the suite is the product; the terminal is its first app", and "a second app is another client of a logic process". With per-app names and folders, a `task_system` plugin for Linear is installed, trusted and logged in once per suite app, and two apps can disagree about the same task system.

**Ask.** Suite-level: `bboxy450-<system>-<provider>` in one suite plugin folder, one registry, one trust record; each app declares which contracts it consumes. The operator's own words ("app name, bboxy450-term in this case") read as a variable, so this needs the operator, not only the Architect.
