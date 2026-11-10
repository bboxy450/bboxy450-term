---
type: research
title: 'Architecture review: MCP-first, skills in the KB, worktree and close-out rigor'
id: 01M70TAXH29PD7VXW6QVJYHA1A
updated: 2026-11-09T18:50:09.242Z
created: 2026-11-09T18:50:09.242Z
number: 93377961366484
created_by: seat:01M70FBP1RV5AWZ5AE3ZZWKNHB
updated_by: seat:01M70FBP1RV5AWZ5AE3ZZWKNHB
authorship:
  signer: seat:01M70FBP1RV5AWZ5AE3ZZWKNHB
  kind: seat
  public_key: SkH0jBJLMgNQ8eyjFw4BxnIDzq9wMmgZUeBUslwMcCw=
  process_generation: '1'
provenance:
  digest: sha256:5f1aca516b83cad66ed3915fd9196972bfabe140bf1058e58467a7af7249a20f
  frontmatter_digest: sha256:124d3c3a5459fa6c523cc2f3b1761b504d3484597ba72b8d77eefe53de95ff18
  signer: seat:01M70FBP1RV5AWZ5AE3ZZWKNHB
  sig: 6/yDLRJ7MiwQb07xBmg/u53f69CMygoFhRWoOKnjiri477+5feT/5hpVPYRQf8zqSoTIJ0T42HYFObR1kub4Dg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151108
  event:
    id: 01M70TAY6Q1J91HE04BB50MQ6T
    sequence: 151108
    hash: 245a619818ef4f90e5cd7ca7e932e216b766b9fc412cb95e68ca476eab13412e
---
# Architecture review: MCP-first, skills in the KB, worktree and close-out rigor

Operator direction (2026-11-09): MCP is the future. Rely less on skills and CLI. Deliver skill guidance through the system prompt at harness start. Tighten rigor around worktrees and closing tasks. Keep hints. The bboxy MCP server should search skills, and the KB should hold skills as a document path so they gain dreaming, provenance and improvement loops.

Source: origin/main after 161ce9173, read-only.

## Current state (file:line)

### Launch guidance already reaches the system prompt
- **Workflow skill:** embedded `crates/bbt-mux/src/adapters/skills/bboxy450/SKILL.md`, rendered by `adapters/launch.rs:186-191` and `harness-wake-up launch.rs:407`.
- **bboxy450-harness:** `--append-system-prompt "<startup>\n\n<skill>"` (wake-up launch.rs:538-545).
- **Codex:** `developer_instructions` (launch.rs:648-668).
- **Claude:** startup text only through `--append-system-prompt`. The skill arrives as a generated `--plugin-dir` skill (launch.rs:427-455, 585-597), so Claude follows a different path from the other two harnesses.
- **MCP `initialize`:** carries `instructions` (`mcp/bridge.rs:289`).
- **`~/.agents` and `.claude/skills`:** never read.

### Skills
- `mcp/skills.rs` serves the builtin workflow skill, the builtin KB skill (`:17`) and enabled plugin skills through `skill_list` and `skill_read` only. URIs are `bboxy://skills/{root}/{source}/{digest}/{name}/SKILL.md` (`:84`).
- There is **no search**.
- MCP resources mirror the skills only (`bridge.rs:318-366`). There are no KB resources and no `prompts/list`.
- `knowledge/types.rs:22-112` has **no `skill` type**. `recipe` (`:94`) is the nearest: a procedure with verification, written with Oracle authority.

### CLI still required
These `ctl` commands have no MCP tool (`ctl.rs:49-151`):
- `task create`
- `seat rename/restart/undo-close/resize/wait/capture`
- `plugin *`, `knowledge select/reset`, `app *`, `events *`
- `debug logs/journal/metrics/ps/snapshot`
- `dream inspect/cancel`

Specs that still prescribe `ctl`: `.bboxy/specs/work-items.md:156-161` and `.bboxy/specs/plugins.md:149`. AGENTS.md:159 tells the Oracle to run raw `git worktree remove`, which bypasses `workspace_release` and leaves its record stale.

### Worktrees
- **Lifecycle:** `mcp/workspaces.rs:64`. Release is rigorous: `workspaces/git.rs:504-580` checks dirt, ancestry and `git cherry`, and `:666` runs `lsof`.
- **What is missing:** no link from accept or seat close to release (`logic.rs:5659-5686`). No orphan audit; `workspace_list` does "no filesystem scan" (`:48`). Several live attempts per task are allowed.
- **Measured today:** 10 worktrees.
  - 5 records are `failed`/`workspace_dirty` and all 5 tasks are `accepted`. The dirty paths are not reported.
  - 3 hand-made worktrees have no record, including one this seat made today and removed by hand.

### Close-out
- `task_accept` (`work_items.rs:1451`, acceptance `:1609`) sets `accepted`. It does not check that `report.commit` is on the integration ref.
- Nothing moves `accepted → done`. The only writer of `done` is `task_reconcile`, which refuses accepted tasks (`:1353`).
- Every `task_*` hint points only to `task_status` (`hints.rs:302`).
- Close-out is the Oracle's manual 3-step (AGENTS.md:156-161).

## Recommended design

### 1. Close-out as one MCP transition (do first: smallest change, highest rigor)
- **`task_accept`** also requires every `report.commit` to be reachable from the workspace `integration_ref`. If not, it is refused with the missing SHAs.
- **New Oracle tool `task_close`** (`accepted → done`) runs four steps, each with its own receipt:
  1. verify the commits are on main;
  2. `workspace_release` every non-released attempt of the task;
  3. `seat_close` the assignee if it has no other live task;
  4. `knowledge_commit` the task and its report.

  A failed step leaves the task `accepted` and returns the failing step plus a hint. It is idempotent per op_id.
- **Hints:**
  - `task_accept` → `task_close`;
  - `seat_close` → `workspace_get`/`workspace_release`;
  - failed release → the dirty-path list.
- **Model:** add `accepted → done` to the task lifecycle model, with a broken control that closes with an unreleased workspace or an off-main commit.

### 2. Worktree audit and recovery
- **`workspace_audit`** (read): joins `git worktree list --porcelain`, workspace records and task status. It reports orphans (no record), accepted tasks whose attempts are not released, failed releases with their dirty paths, and tasks with more than one live attempt. It is bounded and read-only. Cleanup stays explicit.
- **`workspace_release`:**
  - returns the dirty paths;
  - accepts `generated_only` to clean ignored build outputs (`target/` and similar) and nothing else;
  - when the writer seat is closed and its pid is gone, it moves `in_use → ready` itself instead of waiting for a `workspace_get`.
- **AGENTS.md:159:** say `workspace_release` instead of raw `git worktree remove`.

### 3. Skills as KB documents
- **New document type `skill`**, path `skills/<name>/SKILL.md`. Its frontmatter is Agent-Skills compatible (`name`, `description`) plus the usual bboxy fields, so files import from and export to `~/.agents/skills` byte for byte. Provenance, signing, `derived_from` and `knowledge_commit` apply unchanged. A skill revision is a document revision.
- **Skill sources:** `skill_list`/`skill_read` gain `origin: kb`. The URI keeps its digest, so a read is pinned to an exact revision for provenance.
- **Precedence:** the builtin workflow skill cannot be overridden by KB or plugin skills, because it carries the authority text. The KB beats a plugin for the same name only with an explicit `supersedes`.
- **New `skill_search`:** the same paging and cursor as `knowledge_search`, over name, description and body, across builtin, kb and plugin skills. It returns URIs, not bodies.
- **Improvement loop:**
  1. A dream or worker proposes a skill revision as a draft, with `derived_from` links to the reports and evidence that motivated it.
  2. The Oracle reviews it and commits it.
  3. `provenance_lineage` shows which skill revision each seat read (`skill_read` is already journalled), so outcomes can be compared across revisions.
- **Skill vs recipe:** keep both. A skill is model guidance loaded into context. A recipe is a verified procedure with run receipts. This does not conflict with the recipe-composition decision, which declined a new type for procedures only.
- **Write authority:** like `recipe`, writes need Oracle authority. Workers may draft under `skills/` only as proposals.

### 4. Launch guidance: one path for every harness
- Append the same text to the system prompt for all three harnesses: the workflow skill, plus a **compact skill index** (name, description and URI per available skill, bounded to a few KB).
- Claude stops relying on the generated `--plugin-dir` skill for the workflow text. The plugin can stay for slash-command convenience.
- `initialize.instructions` repeats the index, so a harness that drops the system prompt still sees it.
- Bodies are fetched on demand with `skill_read` (progressive disclosure). This keeps the operator's "skills guided via system prompt" without pasting every skill into context.

### 5. MCP over CLI
- **Add MCP tools, in this order:**
  1. `task_create`, so tasks stop requiring `knowledge_write` or `ctl`;
  2. `seat_restart` and `seat_rename`;
  3. `debug_ps` and `debug_logs`, bounded and own-seat for workers;
  4. `dream_cancel`.
- **Operator-only actions** (`plugin trust`, `app stop`) stay outside seat MCP by design ([[decisions/d260-ctl-authority]]), and should be listed in `orient` as operator-only so agents stop asking.
- **KB documents as resources:** expose them as MCP resources (`bboxy://kb/{root}/{slug}`), reusing the `knowledge_fetch` authorization.
- **Specs:** update work-items.md:156-161 and plugins.md:149 to name the MCP tool first.

## Suggested cut order

Each cut is one Executor task with a formal-model update where state changes:
1. `task_accept` checks the commit is on main, plus the new hints. This is a model change.
2. `task_close`. This is a model change.
3. `workspace_audit`, plus dirty-path and `generated_only` release, plus the AGENTS.md fix.
4. The `skill` type with kb-origin `skill_list`/`skill_read`, plus `skill_search`.
5. The launch skill index for all harnesses.
6. `task_create`, then the remaining CLI-only tools.

## Decisions needed (Oracle or Architect)

- **D-a:** the KB becomes canonical for bboxy skills, and `~/.agents/skills` becomes an export target for non-bboxy harnesses. Today AGENTS.md names `~/.agents` as the source of truth.
- **D-b:** `task_close` closes the assignee seat automatically, or only hints at it.
- **D-c:** `generated_only` cleanup is self-service for the workspace owner, or Oracle-only.
