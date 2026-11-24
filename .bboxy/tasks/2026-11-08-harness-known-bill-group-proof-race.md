---
allowed_paths:
- src/dream/resolve.rs
authorship:
  kind: seat
  process_generation: '1'
  public_key: 0X00kpb5zTGUeot1iS3xss6zajL5tcixiAgioL6WETw=
  signer: seat:01M878FXVG76NYQYEEEXAHQ108
created: 2026-11-09T02:13:08.881Z
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
delegated_by: seat:01M878FXVG76NYQYEEEXAHQ108
id: 01M6Z19AWMH2VVHJTN7ZCHYBDG
number: 93377961366420
reconciliation:
- by: seat:01M878FXVG76NYQYEEEXAHQ108
  disposition: superseded
  evidence:
  - 'successor: tasks/2026-11-24-harness-test-debt.md step 2'
  fresh_accepted: false
  kind: historical
  op_id: 01M878KDEM1S3DT1DB195S3Y53
  previous_status: open
  reason: Carried into the live term swarm.
role: executor
status: done
swarm: 01M649102FBS60QWNPZS5VAGF9
title: 'Harness: intermittent "known bill precedes group-proof upgrade" race in external_total_budget_terminal_wait4 test'
type: task
updated: 2026-11-24T17:10:36.632Z
updated_by: seat:01M878FXVG76NYQYEEEXAHQ108
verify: 'cargo test --offline --locked --release --lib dream::resolve::selected_helper_process_tests:: -- --test-threads=1'
provenance:
  digest: sha256:bada31bc35ed976af91df36027a017dc72391cc910650cf1a679dfb92eba362e
  frontmatter_digest: sha256:aa6c08e3427b335b5c79d181126a0f1fd3c3de96aa67db5885e5cf7f0b9c4c80
  signer: seat:01M878FXVG76NYQYEEEXAHQ108
  sig: GUJF+XTiEeCwCyJcEtNHGP0g05yIb/dWEb4GRRdSCpw/Mu2SOX5MDoV8dzFkUDyoUnHbAGxtJC9VbhplMnF1Bw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1877
  event:
    id: 01M878KEG2WJF1485BVN1MZ13P
    sequence: 1877
    hash: b3a76cb9c37e3f5d353ad4a9c2f8f8d7687ad1b9c1310f7ae9c9bd716acada5b
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
