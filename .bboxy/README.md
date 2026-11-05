---
authorship:
  kind: seat
  process_generation: '1'
  public_key: fewHFqX1/aT03A8Ik46eNy0fXOwtv5Rt1gZmJu1Aids=
  signer: seat:01M649MS05FAD5CY9CS0Q07H76
created: 2026-10-21T18:53:32Z
created_by: seat:01M649MS05FAD5CY9CS0Q07H76
id: 5K6NHGAMB85GCTF0DFXF7MT1XZ
number: 57
provenance:
  digest: sha256:032ee1e4f1cd371cf844439772b0fc0a525595d4ee872783003bf99d2b2d78b9
  frontmatter_digest: sha256:beecdb0b4720902788a4244a863505c0ad8d0cd0fa955c88eaa906a6b080bff7
  log: .bboxy/log/2026-10-29.jsonl#1396
  sig: 9FGnh3qOpZNTn0ovQOmh8A/6iDgVcVBr74T2kJ/c0mxJ1Cd/W2UudtCf4wFaNs9KPRRQoMvkWdjLkOx0jfNVCA==
  signer: seat:01M649MS05FAD5CY9CS0Q07H76
title: .bboxy — bboxy450-term knowledge base
type: note
updated: 2026-10-29T18:57:56.821Z
updated_by: seat:01M649MS05FAD5CY9CS0Q07H76
---
# .bboxy — bboxy450-term knowledge base

Renamed from `.kb/` on 2026-10-21 per [[work-items]]: every repo keeps its tasks and knowledge in `.bboxy/`. Frontmatter migration happens on the next touch of each file.

Repository-local records. No secrets or raw transcripts.

Start with [[implementation-journey|the implementation journey]] for the dated path from terminal primitives to a usable swarm, recent landed changes, issue-by-issue gaps and the next verification. [[2026-10-29-zero-issues-cohort]] records the current repair cohort. Historical task boards and graph snapshots describe their dates; native assigned tasks and verified evidence determine current state.

| Dir | Holds |
|---|---|
| `journey/` | The implementation journey and issue disposition by layer; links preserve original record identities. |
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
