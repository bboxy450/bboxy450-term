# Work items — epics, specs, tasks as Markdown with frontmatter

Owner: Vesper Greywind <ORACLE>. 2026-10-21. DECIDED by the operator: native support for tasks, specs and epics, "just md frontmatter based". Back to [[vision]] · [[mcp-control-plane]] (`task_delegate`) · [[plan]].

## Shape

A work item is one Markdown file with YAML frontmatter in the repository the swarm works on, under **`.bboxy/`** (operator decision 2026-10-21; this repo's `.kb/` migrates to `.bboxy/` as the dogfood). `.bboxy/` holds all of a repo's tasks and knowledge: epics, specs, tasks, research, runs. Knowledge files are the same schema with `type: note`. Three types, one schema. Git is the database; the terminal is a reader, writer and watcher of these files, nothing more.

```yaml
---
id: 01JAX3K9Q7M2N8P4R6S1T0V5W9      # ULID, assigned on creation, never changes
type: task | spec | epic | note
title: fork alacritty_terminal + vte into bbt-core
status: open | claimed | running | reported | accepted | rejected | blocked | done
parent: 01JAX…                       # single parent, optional
children: [01JAX…, …]                # kept in sync with children's parent by the terminal
depends_on: [01JAX…, …]
links:                               # typed outbound links, any target
  - { to: "[[messaging]]", rel: spec }
  - { to: "https://docs.rs/quinn/0.11.12", rel: reference }
  - { to: "research/osc7501-capture.md", rel: evidence }
assignee: seat:01JAX… | name:Nyx Copperwing | none
role: executor | oracle | creativity  # role header the delegate carries
allowed_paths: [crates/bbt-core/**, Cargo.lock]
verify: cargo test -p bbt-core --locked
created: 2026-10-21T19:04:00Z
updated: 2026-10-21T21:40:00Z
---
```

Body is free Markdown: goal, steps, acceptance, report. Wikilinks `[[id-or-slug]]` anywhere in frontmatter or body resolve to items in `.bboxy/` by id or slug; plain URLs and repo paths are links too. The terminal indexes both the typed `links` and inline wikilinks into one graph. The filename is `<type>s/<date>-<slug>.md`; the id in the frontmatter is the identity, the filename is for humans.

## What the terminal does with them

| Action | Mechanism |
|---|---|
| Index | Watch `.bboxy/**/*.md`, parse frontmatter, build the graph (parent, depends_on). Invalid frontmatter = shown as broken, never silently skipped. |
| Delegate | `task_delegate{task: path, seat}` does exactly two things, in this order: (1) rewrites the item's frontmatter to `assignee: seat:<id>` and `status: claimed`, so the file itself names the one seat responsible; (2) sends the seat one message, oracle-class, that reads `ROLE: <role>. Execute this: <path>` followed by the body, through the adapter like any message, with the same evidence states. **Report-back:** the delegating seat (the Oracle) is recorded as `delegated_by: seat:<id>` in the frontmatter; when the responsible seat's report lands (a message carrying the task id, or its `done` state with the task open), the mux sets `status: reported` and sends the delegating seat one oracle-class message `Task <id> reported by <seat>: <first line of report>` plus a sidebar notice on the Oracle's row. Nothing is accepted automatically; the Oracle verifies and calls `task_accept` or `task_reassign`. One responsible seat per task at a time; reassigning rewrites `assignee` and sends the new seat the same message, and the old seat a one-line release. Commits nothing. |
| Track | Seat events move `status` (`running` on first activity, `reported` when the seat's report lands on the stream with the task id); the Oracle or operator sets `accepted`/`rejected` via `task_accept`. Every transition rewrites the frontmatter only. |
| Show | A task is a row under its seat in the sidebar with its status glyph; an epic is a collapsible group in the stream dock with its specs and tasks; `blocked` shows what it waits on. |
| Graph | `.kb/graph.md`-style view generated from frontmatter, not hand-maintained once this exists. |

## Rules

- The terminal never deletes a work item. `done` and `rejected` are states.
- Frontmatter is the contract; the body is prose. A seat may edit the body of its own task; only Oracle-class or the operator edit `status`, `assignee`, `allowed_paths`.
- No database, no sidecar JSON. If git has it, the terminal has it.

## Dogfood

This repository's `.kb/` becomes `.bboxy/` in one move (git mv, wikilinks unchanged); tasks get the frontmatter on the next touch of each file, specs get `type: spec`, research and runs get `type: note`, `plan.md` workstreams become epics with `children`. `graph.md` stays hand-maintained until the index exists.

## Gaps

- **Gap:** conflict when two seats edit one item's frontmatter: last write wins in the worktree; the terminal shows the conflict marker. Decide if a lock is needed.
- **Gap:** items in a remote seat's repository (ssh/cloud): the remote supervisor indexes and streams them, or the GUI reads over the state stream. Decide with phase 2.

Links: [[vision]] [[mcp-control-plane]] [[mux-daemon]] [[ux-prototype]] [[plan]]
