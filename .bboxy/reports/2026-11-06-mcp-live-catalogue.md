---
type: report
title: MCP live catalogue, bounded wakes and recovery verification
task: '[[tasks/2026-11-06-mcp-live-catalogue-large-wake-dead-seat-close]]'
author: seat:01M6S0BP5BV4P07WSJSP2X6M27
created: 2026-11-06T19:56:14.178Z
commits:
- 24d03c76b690a4e90b26c3fe81b7211fac051940
verify:
  command: cargo nextest run -p bboxy450-term-mux -E 'not (test(local_document_commit_keeps_unrelated_staging_and_refuses_no_change_and_symlinks) | test(held_private_git_hook_keeps_other_swarm_input_resize_and_status_live) | test(priority_burst_overflow_delivers_every_message_once_in_each_priority_fifo) | test(mux_document_writes_stamp_and_sign_digests_and_file_watch_observes_real_changes))' && make test-formal-contracts
  exit_code: 0
id: 01M6S6XR16BRSBEMJ9DV76V6HM
updated: 2026-11-06T19:56:14.178Z
number: 93377961366102
created_by: seat:01M6S0BP5BV4P07WSJSP2X6M27
updated_by: seat:01M6S0BP5BV4P07WSJSP2X6M27
authorship:
  signer: seat:01M6S0BP5BV4P07WSJSP2X6M27
  kind: seat
  public_key: lN5ZDIQX1O21KR9+0EKpfOusyQxoJjvrXmcXZktT1kQ=
  process_generation: '1'
provenance:
  digest: sha256:72e4a3a08a49d57926dda1d6515ff26c0428ab39389dd0fc6989c9ac10f6449b
  frontmatter_digest: sha256:8a8d5e8e520eb188153b35c9c0d58be22bd84c133d862cc1ab7525f029defe4a
  signer: seat:01M6S0BP5BV4P07WSJSP2X6M27
  sig: g2RZAZoHfMyK8+AzVg9DKz+Uptv5x3GePfqS2LwBaVBUGjmALUNrVMaBNbQVYLm1wAm4yOmqwaRncqtGXyKWCw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/149792
  event:
    id: 01M6S6XS2ZVVBXZWS860TKFSB8
    sequence: 149792
    hash: 816698a368d73a294dd60190444b839fcff2b11e192c3595f0b9ebd28a56c24b
---
Landed source commit 24d03c76b690a4e90b26c3fe81b7211fac051940 on origin/main. All four requested changes are implemented. The amended verification excludes exactly four failures reproduced at base 85aa28c2a0362305b4fd5566db6d3d44a514fba5; they are tracked in [[tasks/2026-11-06-mux-preexisting-test-failures]].

- crates/bbt-mux/src/mcp/bridge.rs:286, :515, :552 and crates/bbt-mux/src/logic.rs:10955 — advertise tools.listChanged, observe the current catalogue and grant epoch during idle stdio service, and emit notifications/tools/list_changed. Tool lists use current authority; valid layout_read calls are refused after downgrade. Initialization stays local; an initial native catalogue probe caused two artifact-readiness regressions and consumed an import relay request, so that probe was removed.
- crates/bbt-mux/src/mcp/work_items.rs:1501 — both delegation and reassignment send role/path/fetch instructions independent of brief size. The existing 16 KiB hard wake bound remains. The private API witness delegates and reassigns an unchanged 40 KiB UTF-8 body.
- crates/bbt-mux/src/logic.rs:5428 — retain upstream commit 3771188e's orphan-close helper, including supervisor absence, recovering-attachment, generation, swarm and caller-authority checks. Add lifecycle closed_retained and reason process was gone to its result. Live close dispatch remains through the existing window path. The private witness rejects a supervisor-owned target without a viewport and rejects a wrong generation.
- crates/bbt-mux/src/mcp/knowledge.rs:453 — preserve omitted host stamps and name differing protected fields. Preserve upstream reserved-task initialization while continuing to refuse changes to status, assignee, delegated_by and role.
- crates/bbt-mux/tests/mcp/live_catalogue.rs:11, :87, :128, :190 — four native/socket acceptance witnesses. Existing amendment, artifact and 250-record import regressions also pass.
- crates/bbt-mux/src/mcp/live_models.rs and verification/contracts.toml:1336 — add LiveCatalogue, DelegationPointer and AbsentSeatClose, their broken controls, source/test mappings, corpus entries, crate inventory and generated TLA+/CFG files.

The detached base worktree ran only the seven originally failing tests once, sharing this seat's target directory: 3 passed, 4 failed, exit 100 (/tmp/mcp-live-baseline-seven.log). Both harmony_host failures reproduce private-KB versus parent-Git assumptions; messaging priority FIFO reproduces 165/192 delivered messages; storage_bus_index reproduces the retired .bboxy/log/ prefix assertion. The artifact tests and import passed at base; all three subsequently passed with the local-initialization correction. The temporary worktree was removed and its absence checked with exit 0.

Validation used canonical Make profile settings (opt-level 1, debug 0), with private fixture identities and no operator input:

| Command | Actual result |
| --- | --- |
| Assigned verify.command in this report's frontmatter | exit 0 on landed 24d03c76; 761/761 nextest cases passed (57 skipped: 53 existing ignores plus 4 explicit baseline exclusions); formal contracts 24/24 and strict Clippy passed |
| BBOXY_UPDATE_MCP_MODELS=1 cargo nextest run --locked -p bboxy450-term-mux --features model --lib -E 'test(mcp_live_contract_models)' | exit 0; all three positive models pass and all three broken controls fail safety |
| cargo nextest run --locked -p bboxy450-term-mux --test mcp with scoped/amendment/artifact/import filter | exit 0; 10/10 passed |
| make test-licenses | exit 0; 34 crates, source notices and negative fixtures held |
| make land | exit 0; strict mux all-target Clippy, 482/482 lib/MCP tests, dependent check, full formal gate and push |
| make test-formal FORMAL_REUSE=1 on landed HEAD | exit 0; 562 models, 209 contracts and 34 crates; native/reference graph/verdict parity held |
| Pinned signature verifier on the delivered commit | exit 0; 1 verified, no exceptions |
| make check-formal-evidence after build cleanup | exit 0; source 24d03c76 and evidence digest 0fc6016c69464134e6eb2e8ba983564ee208a4de52bee3b3123d0560ce8bb752 |

Fresh Java comparisons before the moving-main rebase checked LiveCatalogue 6 states/17 edges, its broken control 11/31; DelegationPointer 9/8 and broken 9/8; AbsentSeatClose 20/98 and broken 24/102. The final source-bound run reused those identical, hash-checked model artifacts and recorded current landed HEAD. Bounds, fairness/availability assumptions and implementation-only coverage are explicit in the contracts; this is not a proof of arbitrary Rust or OS behavior.

Landing follow-up: scripts/land-fast.sh:81 rebases after a rejected push and runs only cargo check before retrying. The first current-HEAD formal evidence check therefore failed (exit 2 from Make). I refreshed full formal verification and reran the assigned verification on the landed commit. The landing script's missing post-rebase test/formal refresh remains outside this task's allowed paths.

No full make test, app reinstall or deployment was performed. Raw logs stay in /tmp/mcp-live-*.log. Removed this seat's generated target outputs except target/formal; the detached baseline worktree is absent. Formal evidence is retained at /Users/feral/mydev/bboxy450-term/.git/bboxy-formal-evidence/0fc6016c69464134e6eb2e8ba983564ee208a4de52bee3b3123d0560ce8bb752/evidence.json. Worktree state: clean; HEAD equals origin/main at the final check.

Log SHA-256: baseline-seven 59ce40847c5dfbb7b3b1bcbf4ee4b9a8548ac84a93d3229a115057345213ded2; verify-landed 51d673b082141d17b858d8b666ac37a3acf2bddcabe324d9d901385f42e07437; formal-landed 3b7b757a67f91c569844c2b272105d4aee6cbf0281e2085f1c59419864753a3a; land-final 700ff911e05acda95eac14f5aea9d6822ca9415f4111bb1a92797fdc8abb8ae2. Each names /tmp/mcp-live-<name>.log.
