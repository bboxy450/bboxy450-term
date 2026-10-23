---
id: 01M5N5S75WYK441C34CT16PGD2
type: comment
title: "repo-local plugins execute on discovery"
status: open
author: name:Wren key:22c0e081a5f4ac21
on: "[[plugins]]"
on_digest: "sha256:4f59ec6a58dda3d6f387bcfb7020796b031417b322fe599e1cf1463141fc7abd"
anchor: "§1 Discovery"
kind: objection
links:
  - { to: "[[plugins]]", rel: on }
created: 2026-10-23T20:01:17Z
updated: 2026-10-23T20:01:17Z
---
# repo-local plugins execute on discovery

Red-team review of `.bboxy/specs/plugins.md` at 16b4c98 (decision #41, the RFC loop's first run). Author grade inferred: written outside a cockpit seat, signed only by the machine-key commit ([[work-items]] RFC loop, aee9fda).

`.bboxy/plugins/` is a discovery root and the shape is learned by **running the binary** (§1 Discovery, §1 Shape). Together that is arbitrary code execution on clone: open a repository someone else wrote and the app runs its binaries before the operator has decided anything. The research names the counter-pattern and the spec does not adopt it: Codex "requires you to review and trust the exact hook definition", changed hooks skipped until re-trusted ([[plugins-sota]] :96, :129); VS Code refuses install on a failed signature (:82).

**Ask.** No plugin binary is executed, not even bare, until a trust record exists that pins its path and sha256; repo-local discovery is off by default and lists candidates without running them; a changed digest drops trust and asks again.
