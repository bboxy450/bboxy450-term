---
type: task
title: Fix 7 pre-existing harness GUI test failures (gui_rich x4, gui_render x3) and remove their KNOWN-DEFECT ignores
role: executor
allowed_paths:
- tests/gui_rich.rs
- tests/gui_render.rs
- src/frontends/gui/
verify: cargo test --offline --locked --jobs 2 --features embedded --test gui_rich --test gui_render
id: 01M6XZ5XTHSRAAXC9KH7Q1MWEB
status: open
delegated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-08T16:17:05.327Z
created: 2026-11-08T16:17:05.327Z
number: 93377961366386
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
updated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
authorship:
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  kind: seat
  public_key: DbWCjlMnSNUcBZMB2JrdfJ36/iQgSFRbtz78XE3WlhU=
  process_generation: '1'
provenance:
  digest: sha256:0670712198469c650ead779d98cb057fc5157513a352bab373b89cbe0c6468bd
  frontmatter_digest: sha256:53b915215861e01ccd7d5c4cedfe59bd1c8c7785626f91e274286b5fc1f89181
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  sig: 2zLMyuzG0ozz+KGilEelpz+4iCJcroU0PijEQbo2Oon/8mFksCz5ZoXUdklO5dfJasBYTpttiNb3AeiX15V4Ag==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150758
  event:
    id: 01M6XZ5YGRGP4GB4J1CTDAKA8C
    sequence: 150758
    hash: 5e95cc0394a29c46eef6bb21af3743ebb3e6496cf60b7e4281364d1b39266d99
---
ROLE: EXECUTOR

## Problem (found 2026-11-08 by the Run Python card polish, worktree bboxy450-harness-python-cell)
Seven harness GUI tests fail on UNCHANGED harness main 390b5836. They were reproduced by restoring base source.
- tests/gui_rich.rs:
  - gui_polish_mcp_cards_show_real_raw_results_on_expansion
  - markdown_output_mermaid_tool_result_is_a_native_figure_and_execute_stays_literal
  - markdown_output_reasoning_is_dim_prose_and_live_text_is_literal
  - the_view_shows_live_tool_role_and_tail_then_the_full_terminal_blob (viewport timeout)
- tests/gui_render.rs:
  - terminal_body_has_no_command_or_exit_line_and_keeps_its_padding
  - tight_tool_heads_fold_rule_and_feed_gap
  - v1_additions_have_quiet_notices_and_no_native_rows
They went unnoticed because scripts/land.py ran changed gui test files WITHOUT the `embedded` feature (0 tests, exit 4), which that cut fixes.

## Goal
For each test: decide whether the test or the renderer is wrong, from the code, the spec and the current visual design; fix it; and remove its KNOWN-DEFECT ignore. No threshold loosening without a written reason.
Render headlessly through the existing capture paths. Never touch the desktop.

## Verify
cargo test --offline --locked --jobs 2 --features embedded --test gui_rich --test gui_render
All tests must pass and no KNOWN-DEFECT ignores may remain.
