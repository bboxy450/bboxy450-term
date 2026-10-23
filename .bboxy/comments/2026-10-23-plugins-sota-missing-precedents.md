---
id: 01M5N5S76DPZTR2KYJWFBM5V6P
type: comment
title: "three precedents the spec needs are missing"
status: open
author: name:Wren key:22c0e081a5f4ac21
on: "[[plugins-sota]]"
on_digest: "sha256:46e7cfe68bb89f0ebd743e48dfcd4d768a50ce3708261a8aa1d577f893ae3d1e"
anchor: "Remaining gaps"
kind: suggestion
links:
  - { to: "[[plugins-sota]]", rel: on }
created: 2026-10-23T20:01:17Z
updated: 2026-10-23T20:01:17Z
---
# three precedents the spec needs are missing

Red-team review of `.bboxy/research/plugins-sota.md` at 16b4c98 (decision #41, the RFC loop's first run). Author grade inferred: written outside a cockpit seat, signed only by the machine-key commit ([[work-items]] RFC loop, aee9fda).

The research covers protocols well and skips three things the spec depends on. (1) GitHub CLI extensions: `gh-<name>` repositories installed with `gh extension install`, the closest precedent for the operator's naming plus install-from-a-repository, and for how a tool trusts that code. (2) Helm plugins: a manifest plus hooks, another naming and hook precedent. (3) Confining a subprocess on macOS (sandbox profiles, what a non-notarised CLI child may do): the spec's isolation section needs this, and §4 only covers loaded plug-ins (:83).

**Ask.** Add the three, same format, before the operator conversation.
