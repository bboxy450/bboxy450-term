---
id: 01M5WG2WRXB79K84R58QP93KT8
type: interface
number: 348
title: "ICD: app ↔ harness"
status: baselined
links:
  - { to: "[[interfaces]]", rel: index }
created: "2026-10-26T16:18:20Z"
updated: "2026-10-26T16:18:20Z"
---
# ICD: app ↔ harness

| | |
|---|---|
| Side A | the launcher and the harness kits (`bboxy450-term`, `bboxy450-harness-wake-up`, `bboxy450-harness-host`) |
| Side B | the native clients (Claude Code, Codex) and zsh, external |
| The agreement | [[w2-5-mcp-and-seat-wiring]]; [[harness-kits]] |
| Amended by | #262 roles; #265 trust; #270 the hook review recipe; #279 a runtime per seat |
| What crosses | our MCP server, hook shim and skill by launch arguments, environment and the per-seat runtime; shell integration for zsh |
| Timing | a launched seat is a starting pane in the next frame; the harness comes up asynchronously |
| States and lifecycle | each Codex seat gets its own app-server and socket, never the operator's daemon, and shares the user's harness home and login; a restart is generation plus one, resumed by session id |
| Failure | the wake-up kit says whether a harness can be woken and names the fallback; native outcomes are never retried blindly |
| Hook diagnostics (#56, Oracle cf405d78) | isolate inherited parent session IDs at Codex TUI/service exec; observe only its verified owner-PID, uniquely loaded thread via `thread/resume` with `excludeTurns:true` and no overrides, cancelled on binding/generation change; forward bounded `hook/completed` failures to the best-effort run journal, never hook input/context; unavailable stderr/exit/duration and async-hook coverage remain unknown, never inferred |
| Versioning | the app never writes the user's `~/.claude` or `~/.codex`; writes the client makes there are listed with cause; the hook-trust record is the one accepted app-caused write |
| Verified by | promises L3, L4, T6, T9; the native integration proof on installed clients |
| Open | a real model's choice to orient is advisory only, never a verdict |

A change to this interface names both sides, the reason and the verification it needs, and lands only with both sides' owners agreeing. A question an Executor asks about this boundary is a missing line here: the answer is written into this table, not only sent.

Owner: Corvin Duskmantle <ARCHITECT>.
