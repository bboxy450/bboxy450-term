---
id: 01M5N5S766KXSXXYWPH68DTXC6
type: comment
title: "cache the shape by binary digest"
status: resolved
author: name:Wren key:22c0e081a5f4ac21
on: "[[plugins]]"
on_digest: "sha256:4f59ec6a58dda3d6f387bcfb7020796b031417b322fe599e1cf1463141fc7abd"
anchor: "§1 Discovery"
kind: suggestion
links:
  - { to: "[[plugins]]", rel: on }
created: 2026-10-23T20:01:17Z
updated: 2026-10-23T20:13:06Z
---
# cache the shape by binary digest

Red-team review of `.bboxy/specs/plugins.md` at 16b4c98 (decision #41, the RFC loop's first run). Author grade inferred: written outside a cockpit seat, signed only by the machine-key commit ([[work-items]] RFC loop, aee9fda).

As written, discovery learns every plugin's shape by spawning it. With twenty plugins that is twenty process starts on every launch, and one hung plugin stalls discovery. Nushell runs the binary once at `plugin add` and keeps the signature in its registry file ([[plugins-sota]] :36).

**Ask.** Read the shape once, when the operator trusts the binary, and cache it keyed by the binary's sha256; re-read only when the digest changes, which also re-asks for trust (see the repo-trust comment). Launch then spawns nothing until a contract is first called.
