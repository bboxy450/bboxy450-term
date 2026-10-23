# Work items — epics, specs, tasks as Markdown with frontmatter

Owner: Vesper Greywind <ORACLE>. 2026-10-21. DECIDED by the operator: native support for tasks, specs and epics, "just md frontmatter based". Back to [[vision]] · [[mcp-control-plane]] (`task_delegate`) · [[plan]].

## Shape

A work item is one Markdown file with YAML frontmatter in the repository the swarm works on, under **`.bboxy/`** (operator decision 2026-10-21; this repo's `.bboxy/` migrates to `.bboxy/` as the dogfood). `.bboxy/` holds all of a repo's tasks and knowledge: epics, specs, tasks, research, runs. Knowledge files are the same schema with `type: note`. Seven types, one schema (operator, 2026-10-21): **epic**, **spec**, **task**, **bug**, **note**, **invariant**, **artifact**. Every item is a **markdown** (the operator's word; "slug" is retired). Git is the database; the terminal is a reader, writer and watcher of these files, nothing more.

```yaml
---
id: 01JAX3K9Q7M2N8P4R6S1T0V5W9      # ULID, assigned on creation, never changes
type: epic | spec | task | bug | note | invariant | artifact
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
verify: cargo test -p bbt-core --locked   # task: the hard gate; exit 0 is the only acceptance
on: "[[plugins]]"                        # comment: the document reviewed
on_digest: sha256:…                      # comment: the exact version reviewed
anchor: "§1 Shape"                       # comment: section or line
kind: objection                          # comment: question | objection | suggestion | approval
review: boundary fidelity against [[gui-components]] §2c   # task, optional: judgement, advises, never blocks
check: sh scripts/layering.sh            # invariant: the command; exit 0 means held
cost: an upward edge welds a library to the product   # invariant: one line
created: 2026-10-21T19:04:00Z
updated: 2026-10-23T23:41:02Z
---
```

Body is free Markdown: goal, steps, acceptance, report. Wikilinks `[[id-or-slug]]` anywhere in frontmatter or body resolve to items in `.bboxy/` by id or slug; plain URLs and repo paths are links too. The terminal indexes both the typed `links` and inline wikilinks into one graph. The filename is `<type>s/<date>-<slug>.md`; the id in the frontmatter is the identity, the filename is for humans.

## Store (operator, 2026-10-23, [[d47-decision-nodes]])

A record is one file and the file is the object: one decision, one comment, one research record, one spec, one task. The git tree under `.bboxy/` is the store, the first and for now the only backend. The schema never assumes a path: identity is the ULID `id`, addressing is the slug or the id, a version is the content digest, a relation is a link by slug. A binary or remote store can hold the same objects later behind the same reads and writes; nothing pluggable is built now ("KISS for now so we can keep developing our system").

## Types

| Type | What it is | Status set | Typical links |
|---|---|---|---|
| epic | an outcome spanning specs and tasks | open, running, done | `children` specs and tasks |
| spec | a decided design for one concern | draft, decided, superseded | `parent` epic; `[[…\|evidence]]` to research notes |
| task | one Executor assignment | open, claimed, running, reported, accepted, rejected, blocked, done | `parent` spec or epic; `depends`; `assignee` |
| bug | a defect with a reproduction | open, claimed, running, reported, fixed, wontfix, duplicate | `[[task\|blocks]]`, `[[spec\|violates]]`, `[[invariant\|violates]]` |
| note | knowledge: runs, runbooks, evidence | none (always current until superseded) | anything |
| invariant | a property that holds before and after every change, **with the command that checks it**; the thing bugs are checked against and specs may not contradict. No command, no invariant: then it is a preference and lives in a note | active, retired | `[[spec\|constrains]]`, `[[bug\|violated-by]]` |
| decision | a numbered record of alternatives, choice, reason and date, written when the decision is made, **one file per decision** `d<N>-<slug>.md` with `number`, `status: decided \| amended \| superseded`, `decided`, `operator` (a list of verbatim words), and links `decides`, `constrains`, `evidence`, `answers`, `amends`, `supersedes` ([[d47-decision-nodes]]; [[2026-10-22-architecture-round-1]] is the index of the first 46) | proposed, decided, superseded | `[[spec\|changes]]`, `[[invariant\|creates]]` |
| proposal | an RFC: motivation, design, alternatives, open questions; reviewed by red-team comments, closed by a decision that creates or changes a spec (operator, 2026-10-23: "proposal should also be a document type, then we can do entire RFC") | draft, review, accepted, rejected, withdrawn | `[[spec\|changes]]`, `[[decision\|decided-by]]`, `[[research\|evidence]]` |
| comment | one review remark on one document version: `on: [[doc]]`, `on_digest` (the reviewed version's digest), `anchor` (section or line), `kind: question \| objection \| suggestion \| approval`; written by a seat and signed by its leaf like every record (operator: "comment should be a document type as well") | open, resolved, withdrawn | `[[doc\|on]]`, `[[comment\|resolves]]` |
| research | truth compression: `file:line — mechanism` and `source — fact` pointers with sources, never prose summaries; what a spec cites as its evidence (operator, 2026-10-23: "research should be a document type as well"; the files in `research/` take `type: research` on their next touch) | none (always current until superseded) | `[[spec\|evidence-for]]` |

**The RFC loop (decision #41).** A `proposal` is written; the Oracle delegates a **red-team review** to a seat (a Claude Executor by the operator's choice, decision #18), which writes `comment` records against the proposal's digest; the author answers with comments or a revision; a `decision` closes it and creates or changes the `spec`. Every record in the loop is written through the task tools and signed by the writer's seat leaf (W2.5d), each comment pins the digest it reviewed, and the decision links the proposal, so provenance holds the whole argument: who proposed, who objected to which version, what was decided and why. The viewer shows comments beside their anchor (later, [[document-viewer]]). **Outside a running cockpit** (today: the agents are ICQ sessions, not seats), a comment or proposal is a plain Markdown file under `.bboxy/comments/` or `.bboxy/proposals/` with `author: name:<Name> key:<icq key prefix>` in its frontmatter, landed in a commit signed by the machine key; the commit signature attests the machine and the operator's account, not the seat, so the record's author grade is **inferred** (the [[provenance]] grades: reported beats inferred, nothing inferred is shown as certain) and it carries no `provenance` block. When the same agent runs as a cockpit seat, the task tools write the record and the seat leaf's DSSE signature joins; a later `artifact_claim` by that seat upgrades an inferred record to reported. Acceptable until agents run as seats, which is the cockpit's own goal; no second signing path is built for the interim.

Invariants are first-class so the graph can answer "what does this bug break" and "which spec does this invariant bind". An invariant carries four things (Architect, 2026-10-22): the **property**, the **check** command, the exit status that means **held**, and the **cost** of a violation; `check` and `cost` are frontmatter fields (schema above). Recorded in `.bboxy/invariants/` with its index: every crate depends only on crates below it (`scripts/layering.sh`); the UI thread never waits on IO (`scripts/ui-lint.sh`); L7 modules draw only through components; every commit on main is signed. Still preferences until they get a command: one meaning per signal; rows never reorder; the app never holds the primary secret; unknown is never shown as failure; no LGPL in the tree.

## Wikilinks and weights (operator, 2026-10-21: "wikilinks is the way")

Wikilinks are the link primitive everywhere: frontmatter, body, messages. Syntax, three forms:

| Form | Meaning |
|---|---|
| `[[slug]]`, `[[slug\|display]]` | link, relation `ref`, weight 1; exactly Obsidian's forms |
| `[[slug\|rel]]` | typed link: `spec`, `evidence`, `parent`, `child`, `depends`, `blocks`, `see` |
| `[[slug\|rel\|w]]` | typed and weighted, `w` a positive number; higher = stronger |

`slug` (the link target text; the item itself is a **markdown**) resolves to a markdown id or filename slug in `.bboxy/`, to an artifact id, or to a seat (`seat:nyx-copperwing`), a swarm, or a URL in angle brackets. The `links:` frontmatter list is the same thing in YAML for tools that prefer it; the terminal merges both. Weight defaults: `parent`/`child` 3, `depends`/`blocks` 2, everything else 1, unless written.

## The graphs the terminal builds

All from the same index; none hand-maintained once the index exists. The code graph is built with tree-sitter (grammars bundled for Rust, TypeScript, Python, Go, Zig to start) by the `.bboxy` indexer's sibling, the code indexer, incrementally on file events; LSP is not required and is an optional enrichment later. It lives in the same in-memory index and is persisted in the index object ([[storage]]).

| Graph | Nodes | Edges and weights | Used for |
|---|---|---|---|
| **markdown graph** (was "slug graph"; operator rename 2026-10-21) | every markdown in `.bboxy/` | every wikilink, weighted as above | navigation, "related", search ranking, the launcher's Go to |
| **artifact graph** | artifacts: markdowns, PRs, binaries, files, images, releases | `produced_by` seat, `for` task, `derived_from` artifact; weights by link type | "who is responsible for this", release manifests, the provenance graph's leaves |
| **task graph** | epics, specs, tasks | `parent`/`child`, `depends`/`blocks` | ordering, what is blocked, what is claimable |
| **execution graph** | seats × tasks | `assignee`, `delegated_by`, status, live seat state | the sidebar's task rows, the Oracle's view of who is on what, this repo's `graph.md` generated |
| **code graph** (operator, 2026-10-21: "codegraphs!") | symbols (functions, types, modules, files) in the seat's worktree | defines, references, calls, imports/depends; weight = reference count | "what calls this", "what breaks if this changes", impact of a task's `allowed_paths`, wikilinks from markdowns to code symbols (`[[code:crate::module::fn]]`) |
| **channel graph** | swarm, subchannels, subgroups, seats | membership and may-send-to (allowed), traffic weight (actual) | where attention goes; the Oracle reshapes it |
| **messaging graph** | seats, swarms, channels | who messaged whom; weight = count decayed by recency; role class on the edge | stream filters, "who is talking to whom", starvation and silence detection |

## What the terminal does with them

| Action | Mechanism |
|---|---|
| Index | Watch `.bboxy/**/*.md`, parse frontmatter, build the graph (parent, depends_on). Invalid frontmatter = shown as broken, never silently skipped. |
| Delegate | `task_delegate{task: path, seat}` does exactly two things, in this order: (1) rewrites the item's frontmatter to `assignee: seat:<id>` and `status: claimed`, so the file itself names the one seat responsible; (2) sends the seat one message, oracle-class, that reads `ROLE: <role>. Execute this: <path>` followed by the body, through the adapter like any message, with the same evidence states. **Report-back:** the delegating seat (the Oracle) is recorded as `delegated_by: seat:<id>` in the frontmatter; when the responsible seat's report lands (a message carrying the task id, or its `done` state with the task open), the mux sets `status: reported` and sends the delegating seat one oracle-class message `Task <id> reported by <seat>: <first line of report>` plus a sidebar notice on the Oracle's row. Nothing is accepted automatically; the Oracle verifies and calls `task_accept` or `task_reassign`. One responsible seat per task at a time; reassigning rewrites `assignee` and sends the new seat the same message, and the old seat a one-line release. Commits nothing. |
| Track | Seat events move `status` (`running` on first activity, `reported` when the seat's report lands on the stream with the task id); the Oracle or operator sets `accepted`/`rejected` via `task_accept`. Every transition rewrites the frontmatter only. |
| Show | A task is a row under its seat in the sidebar with its status glyph; an epic is a collapsible group in the stream dock with its specs and tasks; `blocked` shows what it waits on. |
| Graph | `.bboxy/graph.md`-style view generated from frontmatter, not hand-maintained once this exists. |

## Artifacts (operator, 2026-10-21)

Harnesses produce **artifacts**. A markdown in `.bboxy/` is an artifact. A PR is an artifact. A binary, a release, a file, an image, a rendered PNG from the lab are artifacts. An artifact has an id (ULID), a `kind` (markdown, pr, binary, file, image, release, other), a `ref` (path, URL, digest, PR number), a `digest` where content-addressable, and a **responsible seat**.

A harness claims responsibility over MCP: `artifact_claim{op_id, kind, ref, digest?, task?}` = "this is my artifact, I am responsible for it". The mux records the artifact in `.bboxy/artifacts/<id>.md` (frontmatter `type: artifact`, `kind`, `ref`, `digest`, `produced_by: seat:<id>`, `for: [[task]]`, `derived_from: [[…]]`) and links it from the task. `artifact_release{op_id, artifact_id, to: seat}` hands responsibility over; `artifact_list` and `artifact_get` read. Responsibility is one seat at a time, like tasks.

**Detection, not only declaration (operator, 2026-10-21).** The terminal detects artifact creation and modification itself and keeps the history, whether or not the harness claimed anything:

| Source | What is detected | Attribution |
|---|---|---|
| Filesystem watch on the seat's cwd and worktree (FSEvents on macOS, inotify on Linux) | file created, modified, deleted, renamed; digest before/after | the seat whose PTY was active in that worktree in the window; `inferred` grade |
| Harness tool events (hooks `PostToolUse` Write/Edit, Codex `turn/*` with file diffs, OSC 7502) | exact path and the tool that wrote it | the seat, `reported` grade |
| git in the worktree | commits, branches, tags, their trailers | the seat named in `Bboxy-Seat`, else the seat active at commit time |
| `gh` / provider APIs (later) | PR opened, updated, merged, release published | the seat that ran the command, else `artifact_claim` |

Each detection appends an **artifact event** `{artifact_id, seat, kind: created|modified|deleted|renamed|committed|published, digest, ts, grade, source}` to the artifact's markdown as a `history:` list in frontmatter (bounded, newest N inline, the rest in `.bboxy/log/`). An unclaimed detected artifact gets an auto-created record with `produced_by` = the inferred seat and `claimed: false`; `artifact_claim` later flips it to `claimed: true` and upgrades the grade. The sidebar shows "N artifacts touched" on a seat row and the Graph tab shows the artifact's history. Grade rules are the same as seat state: reported beats inferred; nothing is shown as certain that was only inferred. The seat roster plus the artifact record is the provenance leaf: who, under which key, for which task, from what.

## Provenance (operator, 2026-10-21: "provenance in that system is very important")

Every item and every status transition records who did it, from what, and under which key. Decided in [[provenance]]: signed digests on every markdown, a hash-chained activity log with signed checkpoints, commit trailers, and the provenance graph (markdown ← activity ← seat ← grant ← root) as the sixth graph. Until then the rule is: no anonymous write to `.bboxy/`. The terminal stamps `updated_by: seat:<id>` on every frontmatter write it performs, and a seat's commits carry the `Bboxy-Seat` and `Bboxy-Task` trailers.

## Rules

- The terminal never deletes a work item. `done` and `rejected` are states.
- Frontmatter is the contract; the body is prose. A seat may edit the body of its own task; only Oracle-class or the operator edit `status`, `assignee`, `allowed_paths`.
- No database, no sidecar JSON. If git has it, the terminal has it.
- **The spec signs off, not the agent** (Architect, 2026-10-22). An Executor's report moves a task to `reported`, never further. `accepted` requires a recorded `verify_result = {command, exit_code, evidence: [{criterion, reference}]}` whose `command` equals the task's `verify` and whose `exit_code` is 0, plus one evidence entry per criterion whenever the task's frontmatter lists acceptance criteria; `task_accept` refuses otherwise and names the missing piece (decision #28, 2026-10-23). A task's swarm scope: **every present** seat reference among `assignee` and `delegated_by` must resolve to the caller's registry swarm; one alone anchors the scope, two must match each other, none is ambiguous and refuses; never inferred from a path. So a fresh task with `assignee: none` and `delegated_by: seat:<Oracle>` is delegable without a new field.
- **Hard gates and judgement are separate instruments.** `verify` is deterministic and blocks. `review` is judgement (boundary fidelity, semantic drift, stale assumptions): it advises and escalates in the report, never blocks silently and never substitutes for `verify`.
- **An invariant violation halts the work that broke it.** The task goes `blocked` with `[[invariant|violates]]`; the halt names the invariant, the evidence and what clears it, posted as a `decision`-kind message by the Architect or the Oracle. Nothing ships over a violated invariant.
- **Decisions are recorded when made**, as `type: decision`, with alternatives, choice, reason and date. A decision nobody can find is a rumour.

## Dogfood

This repository's `.bboxy/` becomes `.bboxy/` in one move (git mv, wikilinks unchanged); tasks get the frontmatter on the next touch of each file, specs get `type: spec`, research and runs get `type: note`, `plan.md` workstreams become epics with `children`. `graph.md` stays hand-maintained until the index exists.

## Gaps

- **Gap:** conflict when two seats edit one item's frontmatter: last write wins in the worktree; the terminal shows the conflict marker. Decide if a lock is needed.
- **Gap:** items in a remote seat's repository (ssh/cloud): the remote supervisor indexes and streams them, or the GUI reads over the state stream. Decide with phase 2.

Links: [[vision]] [[mcp-control-plane]] [[mux-daemon]] [[ux-prototype]] [[plan]]
