---
id: 01M5NVBXE9PQN1A7CE0VTBGDHB
number: 229
type: comment
title: "what would block P-0 tomorrow"
status: resolved
author: name:Seraph key:eec764b7d2f4b9b8
on: "[[plugins]]"
on_digest: "sha256:a3b7a2b411f819d0fbaa532d587847fe91891bf9a4b9b14e0c2419c9c65d9212"
anchor: "§3 Order · P-0"
kind: suggestion
links:
  - { to: "[[plugins]]", rel: on }
created: 2026-10-24T02:20:49Z
updated: "2026-10-24T02:24:35Z"
---
# what would block P-0 tomorrow

Review of `.bboxy/specs/plugins.md` revision 3 at main 9c2022a7 (digest pinned above), fresh eyes from an idle Executor at the operator's request ("review plugin draft and send message to you with feedback!"). Author grade inferred: written outside a cockpit seat, signed only by the machine-key commit. Not repeated here: the 21 comments of 2026-10-23, all resolved.

If the operator says go tomorrow, P-0 cannot be briefed as written: "Acceptance is written when the order is agreed", and these are open (each has its own comment):

- the tier table, one row per contract (tier-table);
- where the schemas live relative to `bbt-proto` (bbt-plugin-layering);
- the env allowlist and cwd (spawn-env-cwd);
- the lifecycle, MCP's or the plugin-as-MCP-server (mcp-lifecycle);
- the ignore-unknown rule and the streaming form (unknown-fields-streaming);
- the hook ceiling as data (hook-budget-ceiling).

**Suggested P-0 acceptance.** `bbt-plugin` holds the shape, seven contract schemas and the trust-record type. `bboxy450-term plugin check <binary>` passes a reference trivial plugin and refuses, by name, an unknown contract, a MAJOR mismatch, a stdout write that is not protocol, and an over-ceiling budget. The round-trip measurement records cold and warm p50/p99 over n ≥ 100 with the load average.
