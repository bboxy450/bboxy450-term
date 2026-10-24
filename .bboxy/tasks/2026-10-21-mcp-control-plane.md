---
id: "3WWW0VJXQTF8V7ZT4B9DHJT19S"
type: "task"
number: 85
title: "Task M: MCP control plane — the Oracle's hands"
created: "2026-10-21T19:44:55Z"
updated: "2026-10-24T00:10:38Z"
---
# Task M: MCP control plane — the Oracle's hands

Date: 2026-10-21. Owner: Vesper Greywind <ORACLE>. Assignee: Nyx Copperwing <EXECUTOR>. Status: ASSIGNED.

## Goal

bboxy450-term runs an MCP server. The Oracle seat drives the whole terminal through it, modelled on iTerm2's `it2` binary: list seats, read any seat's screen and scrollback, send text or keystrokes, create seats, split, focus, close, assign roles, set per-seat channel permissions, message. Executors get a minimal set (own status, message within grants). See [[vision]] Roles and control, [[mux-daemon]] (writer lease, input IDs), [[messaging]] (grants, evidence states), [[harness-adapters]] (input guard) once it lands.

## Research

- `it2 --help` tree and subcommand help on this machine (installed with iTerm2 3.7.6; `~/.agents/skills/it2/SKILL.md`), plus the iTerm2 Python API it fronts: enumerate every verb that maps to a seat operation. `command — result`.
- How Claude Code and Codex consume MCP tools: tool naming, JSON schema shape, result size limits, streaming or not, how a tool call shows in the TUI. Cite local `claude --help`, `codex --help`, and the official MCP docs you fetch.
- Local MCP transport choice: stdio child per seat vs one unix-socket server the harness connects to; what each harness supports today.

## Deliverable

`.bboxy/specs/mcp-control-plane.md`: tool catalogue (name, params, result, who may call it by role), how a call is authenticated to a seat and its grants (from [[messaging]] seat grants), how screen reads page through [[mux-daemon]] GetHistory, how input goes through the writer lease and the adapters' input guard, spawn limits per cohort (operator cap), audit log of Oracle actions, what Executors can call. Cohort templates ("Oracle + 2 Executors") as one tool. **Gap:** lines.

## Verification

```sh
ls .bboxy/specs/mcp-control-plane.md && grep -c '^| ' .bboxy/specs/mcp-control-plane.md
```

Report on `#bboxy450-term`.
