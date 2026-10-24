---
id: "5K6NHGAMB85GCTF0DFXF7MT1XZ"
type: "note"
number: 57
title: ".bboxy — bboxy450-term knowledge base"
created: "2026-10-21T18:53:32Z"
updated: "2026-10-24T00:10:38Z"
---
# .bboxy — bboxy450-term knowledge base

Renamed from `.kb/` on 2026-10-21 per [[work-items]]: every repo keeps its tasks and knowledge in `.bboxy/`. Frontmatter migration happens on the next touch of each file.

Repository-local records. Markdown only. No secrets, no raw transcripts.

| Dir | Holds |
|---|---|
| `vision.md` | What we are building and why. Oracle-owned. |
| `graph.md` | **Execution graph: who is working on what, dependencies, state.** Oracle-maintained, updated on every claim/handoff/accept/block. |
| `plan.md` | Workstreams and crate split. |
| `runbook-web-to-native.md` | **The runbook for the web → native loop**: lab, design system, tokens, ui-model, bboxybook, parity script, CI, roles, commands, how to read the report, how to add a component. |
| `ux-sidebar.md`, `ux-pick.md`, `ux-flow.md` | UX constraints, the Oracle's selection history, and the operator's running rule log. |
| `specs/ux-prototype.md` | **The current UX state**, distilled from the lab code at tag `ux-prototype-v1`. Supersedes `ux-pick.md` wherever they differ. |
| `tasks/` | One file per assignment: `YYYY-MM-DD-slug.md`. Scope, allowed files, acceptance checks, evidence. |
| `research/` | Executor research output. `file:line — mechanism` and `command — result`. Pointers, not summaries. |
| `specs/` | Technical specs derived from research. One concern per file. |
| `runs/` | Logs, build output, evidence artifacts referenced from tasks. **Screenshots the operator sends go here** as `ref-<what>.png` and are linked from the doc that interprets them, so every harness can look at them. |

Roles: Oracle + Creativity = the Claude session on `#bboxy450-channel`. Executors = Codex sessions. Executors write `research/` and `specs/`; the Oracle verifies and owns `vision.md` and `tasks/`.
