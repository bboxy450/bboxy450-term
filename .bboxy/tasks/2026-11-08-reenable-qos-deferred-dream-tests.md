---
type: task
title: Re-enable the 28 QoS-deferred harness Dream budget tests via a serialized interactive-QoS test lane
role: executor
allowed_paths:
- tests/
- Makefile
- .config/nextest.toml
verify: make test-dream-c1 && make test-dream-mcp
derived_from:
- '[[tasks/2026-11-06-qos-builds-never-starve-mcp]]'
id: 01M6XXWN6MZVW9ZV6TEV5VCT3Y
status: open
delegated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-08T15:54:33.000Z
created: 2026-11-08T15:54:33.000Z
number: 93377961366385
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
updated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
authorship:
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  kind: seat
  public_key: DbWCjlMnSNUcBZMB2JrdfJ36/iQgSFRbtz78XE3WlhU=
  process_generation: '1'
provenance:
  digest: sha256:f316b993a9300928acb7aea00a058e850d7a66a282358e579ece81cdd99cfdc3
  frontmatter_digest: sha256:666268071b4c2d7233dbe776def730de23485755206683785f1f70f7295742e4
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  sig: 6dsrqU8Z5fhb/FSiSpsgT6fxixR5KeZTZOXrqwAI+ksvr01j1BKEJ3H1fDKecV6wcv1OMXM1SP3x2Rs+cuJDAA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150754
  event:
    id: 01M6XXWNRPBKWWPYE7CHCV5FTA
    sequence: 150754
    hash: 204521e79de5203dcc25ee8de7dda6d133213c622ad656162e43f0c21fb1c93f
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
