---
type: task
title: 'Harness: intermittent "known bill precedes group-proof upgrade" race in external_total_budget_terminal_wait4 test'
role: executor
allowed_paths:
- src/dream/resolve.rs
verify: 'cargo test --offline --locked --release --lib dream::resolve::selected_helper_process_tests:: -- --test-threads=1'
id: 01M6Z19AWMH2VVHJTN7ZCHYBDG
status: open
delegated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-09T02:13:08.881Z
created: 2026-11-09T02:13:08.881Z
number: 93377961366420
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
updated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
authorship:
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  kind: seat
  public_key: DbWCjlMnSNUcBZMB2JrdfJ36/iQgSFRbtz78XE3WlhU=
  process_generation: '1'
provenance:
  digest: sha256:bada31bc35ed976af91df36027a017dc72391cc910650cf1a679dfb92eba362e
  frontmatter_digest: sha256:ced24ffd03f333e2b9938b3300fc75c4159f6bdd0cb7185c25b33eeb500abb73
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  sig: xqMA5NKOu72A6h4lGbtmF5WKC8zbrYenzVXlPT2rnRCJe27Hs1qHIf70kJzqke9CuMp8IZGneKpeaqP5P2S1Aw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150884
  event:
    id: 01M6Z19BR4PCA2GQKXY29A6DX2
    sequence: 150884
    hash: 5bf85ea1a517bfc6dc11e02e6147364805b6eaeca35b736f16e6417c894d781e
---
ROLE: EXECUTOR

## Problem
dream::resolve::selected_helper_process_tests::external_total_budget_terminal_wait4_refuses_unseen_and_rolled_overrun fails INTERMITTENTLY at src/dream/resolve.rs:4719 (earlier ~4857): "known bill precedes group-proof upgrade", left 0, right 2.
- It failed in the concurrent debug land run (pycell2 land), then passed on the next run.
- It then failed again in the SERIALIZED RELEASE lane (--test-threads=1) at load about 6.6 (/tmp/embfix-land2.log:895-987, copied to bboxy450-term target/oracle-evidence/embfix-land2.log).
So it is not only concurrency: there is an ordering race between recording the known bill and the group-proof upgrade, either in the controller or in the test's observation order.
It is KNOWN-DEFECT-ignored on harness main (Oracle, 2026-11-08) so that unrelated landings can proceed.

## Goal
Find the race: can the group-proof upgrade be observed before the known bill is recorded under scheduling variance? Fix the ordering in the source (or the test observation, if the test is wrong), with a deterministic control for both orders and a mutant that reproduces 0 vs 2. Remove the ignore. No budget or law change.

## Scope
bboxy450-harness, src/dream/resolve.rs plus its tests. Worktree and verify are set at dispatch (Q2 is the owner of this code).
