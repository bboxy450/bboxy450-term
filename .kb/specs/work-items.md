# Work items — epics, specs, tasks as Markdown with frontmatter

Owner: Vesper Greywind <ORACLE>. 2026-10-21. DECIDED by the operator: native support for tasks, specs and epics, "just md frontmatter based". Back to [[vision]] · [[mcp-control-plane]] (`task_delegate`) · [[plan]].

## Shape

A work item is one Markdown file with YAML frontmatter in the repository the swarm works on, under `.kb/` by default (configurable per swarm). Three types, one schema. Git is the database; the terminal is a reader, writer and watcher of these files, nothing more.

```yaml
---
id: 01JAX3K9Q7M2N8P4R6S1T0V5W9      # ULID, assigned on creation, never changes
type: task | spec | epic
title: fork alacritty_terminal + vte into bbt-core
status: open | claimed | running | reported | accepted | rejected | blocked | done
parent: 01JAX…                       # epic for a spec/task, spec for a task (optional)
depends_on: [01JAX…, …]
assignee: seat:01JAX… | name:Nyx Copperwing | none
role: executor | oracle | creativity  # role header the delegate carries
allowed_paths: [crates/bbt-core/**, Cargo.lock]
verify: cargo test -p bbt-core --locked
created: 2026-10-21T19:04:00Z
updated: 2026-10-21T21:40:00Z
---
```

Body is free Markdown: goal, steps, acceptance, report. Wikilinks `[[id-or-slug]]` between items. The filename is `<type>s/<date>-<slug>.md`; the id in the frontmatter is the identity, the filename is for humans.

## What the terminal does with them

| Action | Mechanism |
|---|---|
| Index | Watch `.kb/**/*.md`, parse frontmatter, build the graph (parent, depends_on). Invalid frontmatter = shown as broken, never silently skipped. |
| Delegate | `task_delegate{task: path}` reads the file, sets `assignee`, `status: claimed`, `updated`, commits nothing (the file change is the operator's or the seat's to commit), injects the role header plus the body into the seat. |
| Track | Seat events move `status` (`running` on first activity, `reported` when the seat's report lands on the stream with the task id); the Oracle or operator sets `accepted`/`rejected` via `task_accept`. Every transition rewrites the frontmatter only. |
| Show | A task is a row under its seat in the sidebar with its status glyph; an epic is a collapsible group in the stream dock with its specs and tasks; `blocked` shows what it waits on. |
| Graph | `.kb/graph.md`-style view generated from frontmatter, not hand-maintained once this exists. |

## Rules

- The terminal never deletes a work item. `done` and `rejected` are states.
- Frontmatter is the contract; the body is prose. A seat may edit the body of its own task; only Oracle-class or the operator edit `status`, `assignee`, `allowed_paths`.
- No database, no sidecar JSON. If git has it, the terminal has it.

## Dogfood

This repository's `.kb/tasks/*.md` are converted to this frontmatter on the next touch of each file; `.kb/specs/*.md` get `type: spec`; `.kb/plan.md` workstreams become epics. `.kb/graph.md` stays hand-maintained until the index exists.

## Gaps

- **Gap:** conflict when two seats edit one item's frontmatter: last write wins in the worktree; the terminal shows the conflict marker. Decide if a lock is needed.
- **Gap:** items in a remote seat's repository (ssh/cloud): the remote supervisor indexes and streams them, or the GUI reads over the state stream. Decide with phase 2.

Links: [[vision]] [[mcp-control-plane]] [[mux-daemon]] [[ux-prototype]] [[plan]]
