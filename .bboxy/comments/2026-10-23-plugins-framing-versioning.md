---
id: 01M5N5S765NDPFCF4K78AD40X4
type: comment
title: "fix the framing and the version rule now"
status: resolved
author: name:Wren key:22c0e081a5f4ac21
on: "[[plugins]]"
on_digest: "sha256:4f59ec6a58dda3d6f387bcfb7020796b031417b322fe599e1cf1463141fc7abd"
anchor: "§1 Protocol"
kind: suggestion
links:
  - { to: "[[plugins]]", rel: on }
created: 2026-10-23T20:01:17Z
updated: 2026-10-23T20:13:06Z
---
# fix the framing and the version rule now

Red-team review of `.bboxy/specs/plugins.md` at 16b4c98 (decision #41, the RFC loop's first run). Author grade inferred: written outside a cockpit seat, signed only by the machine-key commit ([[work-items]] RFC loop, aee9fda).

§1 Protocol says "JSON-RPC over the plugin's stdio" without the framing (MCP is newline-delimited with no embedded newlines, LSP uses a `Content-Length` header, [[plugins-sota]] :48-49), and the shape carries `shape_version` and `contract_version` with no compatibility rule.

**Ask.** Newline-delimited JSON-RPC 2.0 exactly as MCP stdio (the suite already speaks it); versions are MAJOR.MINOR, MAJOR must match, MINOR is additive and the app accepts any minor at or above the one it needs (Terraform :44, Nushell's refuse-on-mismatch :37); a refusal names the plugin, the contract and both versions. These belong in P-0 with `plugin check`.
