---
type: task
title: Fix 4 bboxy450-term-mux tests failing on main 85aa28c2
role: executor
allowed_paths:
- crates/bbt-mux/
verify: cargo nextest run -p bboxy450-term-mux -E 'test(local_document_commit_keeps_unrelated_staging_and_refuses_no_change_and_symlinks) | test(held_private_git_hook_keeps_other_swarm_input_resize_and_status_live) | test(priority_burst_overflow_delivers_every_message_once_in_each_priority_fifo) | test(mux_document_writes_stamp_and_sign_digests_and_file_watch_observes_real_changes)'
status: open
id: 01M6S2KKQC3CB1N58K2BECKMMN
delegated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-06T18:40:48.856Z
created: 2026-11-06T18:40:48.856Z
number: 93377961366075
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
updated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
authorship:
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  kind: seat
  public_key: DbWCjlMnSNUcBZMB2JrdfJ36/iQgSFRbtz78XE3WlhU=
  process_generation: '1'
provenance:
  digest: sha256:3ba781fa4f61779e5a7ef95133bb58a7a8d175fa21d0682a65c26a98b7c6ec57
  frontmatter_digest: sha256:7e3994f59ccf49e99302e1e3bc4303ce453dcf9cbac82e0fb8b414ceb4f240de
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  sig: OjykFwZO4zQLB+YVyFSrWuzC5v5SGMbBCorGbMVfQuPgoxaRIIZJMbCyumPCeN/KIXAWwOoVknvj54arlNAlCw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/149715
  event:
    id: 01M6S2KNVPT7W3N5XZ45PP0981
    sequence: 149715
    hash: fac65f4351e86b12593a3b3b24fe041ec28227e821ef2456414ed16ad105d3e9
---
# Four mux tests fail on main

They fail on main at 85aa28c2 (evidence: `/tmp/mcp-live-baseline-seven.log`, 3 pass / 4 fail, run by seat 01M6S0BP5BV4P07WSJSP2X6M27 in a managed checkout; the Oracle read the log):

- `harmony_host::local_document_commit_keeps_unrelated_staging_and_refuses_no_change_and_symlinks`
- `harmony_host::held_private_git_hook_keeps_other_swarm_input_resize_and_status_live`
- `messaging::priority_burst_overflow_delivers_every_message_once_in_each_priority_fifo` (165/192 delivered)
- `storage_bus_index::mux_document_writes_stamp_and_sign_digests_and_file_watch_observes_real_changes` (retired `.bboxy/log` provenance URI)

First decide whether each one is environment-dependent (for example, the managed-checkout layout or the parent HEAD) or a real product regression, and fix the cause rather than the assertion. A product regression gets its fix plus any needed model update; a stale test gets corrected to the current spec, with that spec cited. Lean landing: run these tests and the touched crates only, then `make land`. Write the report `reports/2026-11-06-mux-preexisting-failures.md` and call `task_report` once.

## Oracle amend (2026-11-07 ~19:00): two more PRE-EXISTING app failures and one restored-boundary gap join this task
Found by the One Test Binary executor (report `reports/2026-11-06-one-test-binary.md`) and reproduced with the ORIGINAL separate binaries on the before checkout (`before/probe-0.log`), so they are not caused by test consolidation:
- `bboxy450-term` `harness_theme::harness_theme_worker_preserves_identity_draft_and_latest_aba`: expected no agent/model operations, observed one.
- `bboxy450-term` `rich_text_safety::rich_text_safety_private_mcp_expansion_and_inline_paint`: literal-tail renderer assertion.
- `make test-harness-status-control`: restored (non-mutant) execution misses `session_started`; the restored native boundary is unproven.
They now carry `#[ignore = "KNOWN DEFECT: pre-existing on main; fix: tasks/2026-11-06-mux-preexisting-test-failures"]` (the tests, not the Make gate). Fix the root causes and re-enable them as part of this task. Also note the flaky `rich_client::delayed_cancel_cannot_deny_a_new_approval` (epoch calibration timeout; it passed in the final land).
