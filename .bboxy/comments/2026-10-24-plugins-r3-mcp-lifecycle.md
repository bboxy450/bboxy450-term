---
id: 01M5NVBXDJFRMT9QV3R2BQYRPR
number: 220
type: comment
title: "framing is MCP; take the lifecycle too, or make a plugin an MCP server"
status: resolved
author: name:Seraph key:eec764b7d2f4b9b8
on: "[[plugins]]"
on_digest: "sha256:a3b7a2b411f819d0fbaa532d587847fe91891bf9a4b9b14e0c2419c9c65d9212"
anchor: "§1 Protocol"
kind: suggestion
links:
  - { to: "[[plugins]]", rel: on }
created: 2026-10-24T02:20:49Z
updated: "2026-10-24T02:24:35Z"
---
# framing is MCP; take the lifecycle too, or make a plugin an MCP server

Review of `.bboxy/specs/plugins.md` revision 3 at main 9c2022a7 (digest pinned above), fresh eyes from an idle Executor at the operator's request ("review plugin draft and send message to you with feedback!"). Author grade inferred: written outside a cockpit seat, signed only by the machine-key commit. Not repeated here: the 21 comments of 2026-10-23, all resolved.

Framing is "newline-delimited JSON-RPC 2.0, exactly as MCP stdio", but the lifecycle is not specified. There is no runtime handshake carrying the versions and capabilities (`concurrent`, `trace`), no cancellation message (MCP `notifications/cancelled`, [[plugins-sota]] :52), and no shutdown rule (MCP: close stdin, then `SIGTERM`, then `SIGKILL`, :51). §1 Isolation lists "child cleanup on exit" as an obligation without a mechanism.

**Ask (two options for the Architect).** (a) Adopt MCP's lifecycle as is: `initialize` carrying `shape_version`, the contract versions and capabilities, then `notifications/cancelled`, then MCP's shutdown sequence. (b) Go one step further: a plugin *is* an MCP server whose tools are its contract methods. `tools/call` with `structuredContent` maps onto the two-level error taxonomy §1 Isolation asks for (JSON-RPC error = protocol error, `isError` = command error, :53). Every language's MCP SDK becomes a plugin SDK, and the guide shrinks to "an MCP server with these tools, plus `--shape`". Contract MAJOR.MINOR can travel in `serverInfo`/`capabilities.experimental` without fighting MCP's date-string `protocolVersion`.
