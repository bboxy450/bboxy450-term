---
id: "01M5JFXASGX45SR4W0GPG6ASNA"
type: "decision"
number: 3
title: "MCP server inside logic, stdio bridge per seat"
status: "decided"
decided: "2026-10-22"
operator: []
links:
  - { to: "[[2026-10-22-architecture-round-1#3]]", rel: "supersedes-as-index" }
created: "2026-10-22T19:02:54Z"
updated: "2026-10-24T00:10:38Z"
---
# Decision 3: MCP server inside logic, stdio bridge per seat

**MCP server inside logic, stdio bridge per seat.** A seat launches with the
private entry point `bboxy450-term mcp` as its stdio MCP server, carrying a
per-seat capability token; logic resolves token → seat, role, grants and
enforces may-send-to, read scope and spawn caps in one place. One shipped
binary (specs/README S5). Rejected: own daemon; process per seat.
