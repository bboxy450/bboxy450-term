# .kb — bboxy450-term knowledge base

Repository-local records. Markdown only. No secrets, no raw transcripts.

| Dir | Holds |
|---|---|
| `vision.md` | What we are building and why. Oracle-owned. |
| `tasks/` | One file per assignment: `YYYY-MM-DD-slug.md`. Scope, allowed files, acceptance checks, evidence. |
| `research/` | Executor research output. `file:line — mechanism` and `command — result`. Pointers, not summaries. |
| `specs/` | Technical specs derived from research. One concern per file. |
| `runs/` | Logs, build output, evidence artifacts referenced from tasks. **Screenshots the operator sends go here** as `ref-<what>.png` and are linked from the doc that interprets them, so every harness can look at them. |

Roles: Oracle + Creativity = the Claude session on `#bboxy450-channel`. Executors = Codex sessions. Executors write `research/` and `specs/`; the Oracle verifies and owns `vision.md` and `tasks/`.
