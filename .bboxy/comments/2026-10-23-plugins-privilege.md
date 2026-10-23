---
id: 01M5N5S760T02M4V8TAM6V8DS3
type: comment
title: "isolation covers crashes, not privilege"
status: resolved
author: name:Wren key:22c0e081a5f4ac21
on: "[[plugins]]"
on_digest: "sha256:4f59ec6a58dda3d6f387bcfb7020796b031417b322fe599e1cf1463141fc7abd"
anchor: "§1 Isolation"
kind: objection
links:
  - { to: "[[plugins]]", rel: on }
created: 2026-10-23T20:01:17Z
updated: 2026-10-23T20:13:06Z
---
# isolation covers crashes, not privilege

Red-team review of `.bboxy/specs/plugins.md` at 16b4c98 (decision #41, the RFC loop's first run). Author grade inferred: written outside a cockpit seat, signed only by the machine-key commit ([[work-items]] RFC loop, aee9fda).

§1 Isolation promises a plugin's "crash is its own" and nothing about what it may read or do. A subprocess runs with the operator's full privileges ([[plugins-sota]] :117: "OS process with the user's privileges"): it can read the ICQ identity keys, the machine signing key, `~/.agents`, other seats' sockets and environment. "The app never reads the plugin's files, config or credentials" (§1 Protocol) is the one direction that does not matter for safety.

**Ask.** State the trust model in the spec, the guide and the registry row: a plugin is code you run as yourself. Spawn with a scrubbed environment (an allowlist; go-plugin `SkipHostEnv`, :42), never the seat's tokens. Research the macOS options for confining a subprocess before P-1 (the research has no data on it; see the missing-precedents comment on [[plugins-sota]]).
