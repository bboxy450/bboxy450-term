---
id: "6RYXQP3XZR6TFR2GT5N9MWTWH7"
type: "task"
number: 62
title: "Task D: harness touch points — research (redundant pass)"
created: "2026-10-21T18:53:32Z"
updated: "2026-10-24T00:10:38Z"
---
# Task D: harness touch points — research (redundant pass)

Date: 2026-10-21. Owner: Vesper Greywind <ORACLE>. Assignees: one Oracle subagent (writes `research/harness-touchpoints.md`) and one Executor (writes `research/harness-touchpoints-exec.md`). Status: SUPERSEDED (8fd15659). Previous brief wording: OPEN for Executor claim. Converged by the Oracle into `specs/harness-adapters.md`.

## Goal

Enumerate every way bboxy450-term, which owns each harness's PTY, can inject into and observe Claude Code, Codex CLI, pi and omp. See [[vision]].

## Questions

1. Inject: launch flags, system-prompt injection (role prompts), hooks files and events, MCP server config, skills directories, cross-session sockets, typed input as fallback.
2. Observe: turn start/end, waiting-on-input, permission prompt pending, error, window title. Which hook or event proves each.
3. Read the `icq` source (`ls ~/.cargo/registry/src/*/ | grep icq`) for its Claude, Codex, pi and omp adapters. `file:line` for how each is attached and woken.
4. PTY-only detection for a harness with no hooks: termios flags, output quiescence, cursor queries, OSC title/cwd, bracketed paste, synchronized output. State what each signal proves and does not.
5. Interactive-input guard: before the terminal types into a PTY as fallback, how does it know the harness is at a prompt and not mid-render.

## Deliverable

`.bboxy/research/harness-touchpoints-exec.md`. One section per harness, PTY-detection section, matrix harness × touch point. `file:line — mechanism` or `command — result`. `**Gap:**` for unknowns. Links: [[vision]] [[harness-touchpoints]] [[agent-messaging]].

## Verification

```sh
ls .bboxy/research/harness-touchpoints-exec.md && grep -c 'Gap:' .bboxy/research/harness-touchpoints-exec.md
```

Report on `#bboxy450-term`: file, commands + exit status, what came from local source vs memory.
