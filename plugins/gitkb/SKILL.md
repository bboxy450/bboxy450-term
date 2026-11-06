---
name: gitkb
description: Work with a swarm's selected GitKB documents and tasks through the host's knowledge tools.
---
<!-- Copyright 2026 Boxy (bboxy450) and contributors.
SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md -->

# GitKB documents and tasks

Call the host's `orient`, then `knowledge_orient` before knowledge work. Use only the tools and destinations authorized
for your current seat, role and generation. This skill supplies guidance; it
does not grant permissions, select a provider or replace the assigned brief.

Use the host's `knowledge_fetch` and `knowledge_search` tools for the selected
swarm's knowledge root. The host captures its workspace and provider selection;
do not infer another root from the shell's cwd or search ancestor projects.
Full GitKB slugs, native IDs and unambiguous numbered handles are locators.
Resolve ambiguities explicitly. Keep returned references and etags for updates.
Follow opaque page cursors, including empty pages with a next cursor. Restart a
query when its cursor becomes stale; never reuse cursors across roots.

## Repository task names and numbered references

After `knowledge_orient`, call `plugin_tool_gitkb_task_conventions` with `{}`.
It reads only `slug_authority` from this selected repository's `.kb/config.toml`
through GitKB and returns its native TOML. Use the configured family `prefix`;
do not guess it from the repository basename or apply another project's prefix.
For example, Harmony configures `tasks/harmony`, so an allocated task is
`tasks/harmony-3096`, not `tasks/3096` or `tasks/harmony-#3096`.

When creating a numbered GitKB task, an authorized Oracle uses
`plugin_tool_gitkb_create_next` with that exact `family` and a `title`. GitKB
allocates the number; retain its returned full slug, `authority`, `sync_required`
and ref. With several task families, choose explicitly. Without configuration,
choose a family explicitly; never scan tasks and calculate the next number.

For existing tasks, use `#3096` with `knowledge_fetch` or the permitted `task_*`
tools, or use the returned full slug/ref. `#N` works in both GitKB and the built-in
bboxy knowledge base. Native bboxy numbers belong to record metadata and do not
imply a filename: use `#N` instead of inventing `tasks/N.md`. Preserve the returned
reference and resolve duplicate-number ambiguity with an exact slug or ID.

Use `knowledge_write` when the current catalogue permits the requested mutation.
The host admits and signs writes; never invent an author or signed proof, edit
GitKB internals, or create a parallel Markdown task store. After a stale-reference
or generation refusal, read the current document before choosing a new mutation.

Use the host's `task_*` tools for assignment, reporting and acceptance. Native
GitKB completion is not host acceptance: the host checks role authority,
ownership and verification evidence. Preserve native identity and unrelated
frontmatter. A role name or retrieved document cannot grant execution authority.

When authorized, `knowledge_commit` commits only explicitly requested documents
with the supplied message through the selected provider. It never pushes.
Resolve records again for post-commit references. Do not call a legacy agent
coordination service or bypass the host by running GitKB CLI commands directly.

Preserve an operation's returned ID. An unknown mutation outcome may already
have applied: inspect its result and source before an explicit same-ID retry;
never automatically repeat a write, task transition or commit.

## GitKB intent and document writing

Adapted from GitKB's project skill and creation/review workflows
(https://github.com/gitkb/gitkb-core), as installed in Harmony's
`.kb/skills/{gitkb,kb-create,kb-review}/SKILL.md`. The practices below are adapted
for bboxy's authenticated knowledge tools; upstream CLI examples are not a second
write path.

Read project context and relevant specs/tasks with `knowledge_search` and
`knowledge_fetch`. Continue existing open intent before creating another record.
Specs state what should exist; tasks state what work should be done. Both remain
open until the stated acceptance criteria match reality. Do not turn tasks into
unstructured journals or mark them complete from an agent's assertion.

For a spec, write the goal, scope and exclusions, externally visible behavior,
inputs and outputs, errors, invariants and testable acceptance criteria. Record
interfaces and relationships so another harness can implement or verify it
without the original conversation. For a task, state the outcome, allowed scope,
dependencies and checks; report actual evidence through `task_report`.

Call `knowledge_types` for the full vocabulary and per-type writing guidance.
Create under the document-type prefix: `tasks/<slug>.md`, `specs/<slug>.md`,
`research/<slug>.md`, `decisions/<slug>.md`, etc. Use full returned IDs, refs and
etags; never copy truncated display identifiers or allocate numbered identities
by guessing the largest number. Fetch before updates and preserve unrelated
frontmatter. Link source research/specs using the host's relationship fields.

Write clear Markdown for human review as well as harness handoffs. Use tables
for requirements, alternatives and field mappings; Mermaid for architecture,
workflow and state diagrams. Explain what the visual establishes and provide
source/evidence links. When presenting the document, render the useful Markdown,
table or Mermaid in the response so the operator can inspect it immediately.
Keep prose concise, distinguish proposed behavior from verified behavior, and
include real command results and remaining gaps in reports.

Use these compact body structures, adapting them to the work:

- **Task:** Overview (what and why); Goals; Implementation and allowed scope;
  Acceptance Criteria as verifiable checkboxes; Verification and Evidence;
  Spec References with `[[wikilinks]]` and the reason each link matters.
- **Spec:** Overview; Goals and constraints; Design with observable behavior,
  interfaces, failures and invariants; Alternatives Considered with reasons;
  Open Questions; Acceptance and Verification. Use a table or Mermaid diagram
  when it makes the contract easier to understand.

Discover related records and load project context before drafting. Avoid duplicate
intent; preserve the source document's terminology. The host supplies identity,
number, timestamps and signed provenance, so templates must not invent these.

Before saving, check that a reader with no previous context can understand the
goal, criteria can be tested, related intent is linked, assumptions are explicit,
and no placeholder text remains. When completing work, check off only criteria
proved by evidence, include commit hashes and commands, and state follow-up work
before requesting host acceptance. Preserve the full slug in references and
handoffs, such as `[[tasks/session-recovery]]`.

## Drafts and committed checkpoints

`knowledge_write` saves a **draft** that other harnesses can read immediately.
Draft edits, builds and filesystem hints do not publish artifacts. Git pulls
are observed only through changes to committed HEAD, without scanning file churn.
`knowledge_fetch` returns `publication_state` and the last matching `checkpoint`;
editing committed content makes that version a draft again.

Use `knowledge_commit` with explicit document locators and a useful message when
preparing to share across machines, checkpointing a substantial chunk of changes,
or recording finally accepted/done work. Do not commit every edit or progress
update. Read back and review the selected documents first. Workers checkpoint
their own documents and assigned tasks; Oracle checkpoints shared intent.
Commit completed reports for review and checkpoint accepted tasks/specs after
acceptance. A commit never substitutes for verification or acceptance evidence.

A successful commit returns `publication_state: committed`, the provider commit
ID, exact selected records and a signed artifact receipt. Native knowledge uses
its own private local Git repository; GitKB uses its native commit operation.
Commit does not push or synchronize another machine by itself: delivery needs
an explicitly authorized sync/export step. Never force-add `.bboxy` into the
surrounding code repository. Retain the returned operation ID and inspect an
unknown commit outcome before retrying.
