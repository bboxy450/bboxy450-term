---
id: 01M5WG2WS221JWTJ6EQ04S78N0
type: interface
number: 350
title: "ICD: plugin ↔ host"
status: baselined
links:
  - { to: "[[interfaces]]", rel: index }
created: "2026-10-26T16:18:20Z"
updated: "2026-10-26T16:18:20Z"
---
# ICD: plugin ↔ host

| | |
|---|---|
| Side A | the contracts and the host (`bbt-plugin`, the knowledge host in `bbt-mux`) |
| Side B | plugin providers, bundled and external |
| The agreement | [[plugins]]; [[knowledge-base]] |
| Amended by | #249 a plugin is an MCP server; #255 visualization is ours; #278 P-0 rulings |
| What crosses | JSON Schema contracts with wire names; data only, never pixels |
| Timing | hook ceilings as data per contract |
| States and lifecycle | trust is a content-addressed copy; nothing executes before trust; plugins start disabled; one provider instance per root, the root bound at start |
| Failure | a refusal names its reason in the app; external writes reply `signed_writer_unavailable` until the signed-writer bridge; authorship is always the host's |
| Versioning | contracts are versioned in `bbt-plugin`; a provider declares the version it implements |
| Verified by | `plugin check` on its fixture store; contract conformance fixtures; promises G3, G4, D1 and up, N1 and up |
| Open | the signed-writer bridge for external providers |

A change to this interface names both sides, the reason and the verification it needs, and lands only with both sides' owners agreeing. A question an Executor asks about this boundary is a missing line here: the answer is written into this table, not only sent.

Owner: Corvin Duskmantle <ARCHITECT>.
