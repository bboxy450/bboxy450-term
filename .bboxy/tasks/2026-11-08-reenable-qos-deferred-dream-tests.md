---
allowed_paths:
- tests/
- Makefile
- .config/nextest.toml
authorship:
  kind: seat
  process_generation: '1'
  public_key: 0X00kpb5zTGUeot1iS3xss6zajL5tcixiAgioL6WETw=
  signer: seat:01M878FXVG76NYQYEEEXAHQ108
created: 2026-11-08T15:54:33.000Z
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
delegated_by: seat:01M878FXVG76NYQYEEEXAHQ108
derived_from:
- '[[tasks/2026-11-06-qos-builds-never-starve-mcp]]'
id: 01M6XXWN6MZVW9ZV6TEV5VCT3Y
number: 93377961366385
reconciliation:
- by: seat:01M878FXVG76NYQYEEEXAHQ108
  disposition: superseded
  evidence:
  - 'successor: tasks/2026-11-24-harness-test-debt.md step 1'
  fresh_accepted: false
  kind: historical
  op_id: 01M878KFFJWTBFEZ2T6C6GSV08
  previous_status: open
  reason: Carried into the live term swarm.
role: executor
status: done
swarm: 01M649102FBS60QWNPZS5VAGF9
title: Re-enable the 28 QoS-deferred harness Dream budget tests via a serialized interactive-QoS test lane
type: task
updated: 2026-11-24T17:10:38.594Z
updated_by: seat:01M878FXVG76NYQYEEEXAHQ108
verify: make test-dream-c1 && make test-dream-mcp
provenance:
  digest: sha256:f316b993a9300928acb7aea00a058e850d7a66a282358e579ece81cdd99cfdc3
  frontmatter_digest: sha256:022d8d92d26d6c218d88bea04f67bae38b072941c974985b596272af768ea3c6
  signer: seat:01M878FXVG76NYQYEEEXAHQ108
  sig: vWPimzJJQP5OUkPiuSZcuEHDhbL1Kwkj/IL3Y3P5bMpeibf6XhhBoqbGKGJJtnAn8WO702WxRVRP3FTmW0hxDA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1881
  event:
    id: 01M878KGD54DXE808VVTHKZ8K2
    sequence: 1881
    hash: dce299da92f09ce9cce23943324a01e4aade9a530c0614d342b460ec00a2813e
---
ROLE: EXECUTOR

## Problem
28 harness Dream tests carry `#[ignore = "load-sensitive budget/wall window; re-enable via tasks/2026-11-06-qos-builds-never-starve-mcp"]`: 21 QoS, 5 qualification, 1 manual benchmark, per Q2's inventory in the INPUT63 tuple. That QoS task is ACCEPTED, so nothing owns re-enabling them.
Each one failed only because a 1 ms ordinary/control quantum, or a 250 ms wall window, was exceeded by 2-4% on a loaded shared machine (load 3-4 on 10 cores; efficiency-core scheduling). Examples:
- 1,012,958 ns, bootstrap held birth
- 1,032,458 ns, prepared_qualification stale-source arm
- 1,027,125 ns, real_worker_reuses... Validate
- a 253 ms readiness poll
While they stay ignored, these budget laws have no executable check.

## Goal
Make these tests deterministic under shared load WITHOUT changing the product laws (1 ms quantum, 9x debt, caps, 250 ms/30 s):
1. Run budget-measuring tests in a dedicated harness test lane at interactive QoS (the same mechanism the app uses for its interactive threads), and serialize that lane (one budget test at a time). Then re-enable each ignored test and record its measured max unit over N=5 runs.
2. Any test still over budget at interactive QoS on a quiet lane is a real product defect: report its unit composition rather than re-ignoring it.
3. No threshold, assertion or law change. Any test that stays ignored needs a written reason in the report.

## Scope
Worktree: TBD by the Oracle at dispatch, in the bboxy450-harness repository. Allowed paths: tests/** and the Makefile/nextest config for the test lane only.
Verify: TBD at dispatch, the lane target plus the affected crate tests.
