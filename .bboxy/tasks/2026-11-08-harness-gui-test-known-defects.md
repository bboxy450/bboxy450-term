---
allowed_paths:
- tests/gui_rich.rs
- tests/gui_render.rs
- src/frontends/gui/
authorship:
  kind: seat
  process_generation: '1'
  public_key: 0X00kpb5zTGUeot1iS3xss6zajL5tcixiAgioL6WETw=
  signer: seat:01M878FXVG76NYQYEEEXAHQ108
created: 2026-11-08T16:17:05.327Z
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
delegated_by: seat:01M878FXVG76NYQYEEEXAHQ108
id: 01M6XZ5XTHSRAAXC9KH7Q1MWEB
number: 93377961366386
reconciliation:
- by: seat:01M878FXVG76NYQYEEEXAHQ108
  disposition: superseded
  evidence:
  - harness d2e2a03 — gui_render unignored
  - 'successor: tasks/2026-11-24-harness-test-debt.md step 3'
  fresh_accepted: false
  kind: historical
  op_id: 01M878KEKHN1HS47ZVNPJKH0FV
  previous_status: open
  reason: 3 gui_render fixed; 4 gui_rich carried forward.
role: executor
status: done
swarm: 01M649102FBS60QWNPZS5VAGF9
title: Fix 7 pre-existing harness GUI test failures (gui_rich x4, gui_render x3) and remove their KNOWN-DEFECT ignores
type: task
updated: 2026-11-24T17:10:37.644Z
updated_by: seat:01M878FXVG76NYQYEEEXAHQ108
verify: cargo test --offline --locked --jobs 2 --features embedded --test gui_rich --test gui_render
provenance:
  digest: sha256:0670712198469c650ead779d98cb057fc5157513a352bab373b89cbe0c6468bd
  frontmatter_digest: sha256:af8b7e3a3fb2d6efc60f30ecf2f1925bfa482ee3e6c75ca223354b81dbb789fd
  signer: seat:01M878FXVG76NYQYEEEXAHQ108
  sig: gTt6N5aDbFBXRiT7fZLeRehumThCtV5OrV1rAKFhx8msMvlC6gjIXZJRPWQdsD1QoDKoIq928UzYvAksz9ZbCg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1879
  event:
    id: 01M878KFEK4Y1V87ZNBGVZ1CGH
    sequence: 1879
    hash: 7c2e0b6aebf0a97ad30b19bccefc81a60892a15401854a0584118dcec7c48938
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
