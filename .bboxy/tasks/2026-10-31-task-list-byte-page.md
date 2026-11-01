---
assignee: seat:01M6ABGTWW9QF4DCT486NJ4ACM
authorship:
  kind: seat
  process_generation: '1'
  public_key: dNWbVeHRHyt/TnV6QHFyDXtWIj+UElZjvaWtEgWhcFg=
  signer: seat:01M6ABH71TVWGZ621NTD701PSH
created: 2026-11-01T01:31:28.258Z
created_by: seat:01M6ABH71TVWGZ621NTD701PSH
delegated_by: seat:01M6ABH71TVWGZ621NTD701PSH
id: 01M6ABQ9H5X0NVDTC04349AVNG
number: 507
provenance:
  digest: sha256:2e1a52aba559224062bc1dc27e2a9906217f69c33c966de1472f8079708981f5
  frontmatter_digest: sha256:42b1353fa8ca9f1862c91f331772f58ea8ca86d1c2de424a25f4d949e93648c4
  log: .bboxy/log/2026-11-01.jsonl#45
  sig: u3+YuivsY+0XQGfQCtXxZ05YexSfNQC3lJUNXTNoqB7uMd8Q1YVXkiwXhNnV0Du/etwVLJ03xlcLM1QDcTDfAQ==
  signer: seat:01M6ABH71TVWGZ621NTD701PSH
role: executor
status: claimed
swarm: 01M649102FBS60QWNPZS5VAGF9
title: task_list pages by bytes, not item count (tasks bar response_bound)
type: task
updated: 2026-11-01T01:31:41.846Z
updated_by: seat:01M6ABH71TVWGZ621NTD701PSH
verify: cargo test -p bbt-mux --locked work_items
---
# task_list pages by bytes, not item count

ROLE: EXECUTOR. Worktree `~/mydev/bboxy450-term-taskpage`, branch `task-list-byte-page` (from origin/main 42320d93). Only this worktree.

## Defect (reproduced 2026-10-31 by the Oracle)
The sidebar tasks bar shows `response_bound: result exceeds the bounded response size`. MCP `task_list {limit:128}` returns `response_bound` ("knowledge result exceeds the MCP response bound"); `limit:10` returns 183,809 bytes. Each item carries full `body` (6–16 KB) and `metadata` (5–9 KB). Pages are cut by item count (`crates/bbt-mux/src/mcp/work_items.rs` list path ~L106–230, L824–843) while the whole result is checked against `MAX_RESULT` = 512 KiB (`crates/bbt-mux/src/mcp/mod.rs:32`, `journal.rs:464`, `knowledge.rs:496`). A full board can never fit, so the bar errors.

## Required outcome
1. A list page is cut on bytes too: stop adding items before the serialized result would exceed a margin under `MAX_RESULT`, and return `next` so the caller pages on. One item that alone exceeds the bound is projected with `body_truncated: true`, never a whole-page error.
2. List items are lean: list projection does not carry full bodies/heavy metadata history the tasks bar does not render; `task_status`/`task_get` stays the full read. Check what the GUI tasks bar and `ctl task list` actually consume and keep those fields.
3. The GUI tasks bar follows `next` (or requests a page that fits) and renders the board with no error for the live repo (66+ tasks).
4. Regression test: a fixture of ≥128 tasks with ~16 KB bodies lists every task across pages with no `response_bound`; broken control (byte cut removed) fails.
5. If retained-op replay or paging cursors are touched, update the matching model in `verification/` per AGENTS.md.

## Verify
`cargo test -p bbt-mux --locked work_items` plus the new regression; strict clippy on touched crates; `make test` on the committed source before handoff. Land with `scripts/land.sh` if green.

## Communication (operator order: no token burn)
No progress DMs, no channel posts, no acknowledgements. One `task_report` at the end: files+mechanism, commands+exit codes, evidence per item above, gaps, commit. Message the Oracle only for a blocker that stops all work, in ≤3 lines.
