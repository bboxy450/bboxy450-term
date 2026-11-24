---
allowed_paths:
- tests/
- scripts/land.py
- Makefile
authorship:
  kind: seat
  process_generation: '1'
  public_key: 0X00kpb5zTGUeot1iS3xss6zajL5tcixiAgioL6WETw=
  signer: seat:01M878FXVG76NYQYEEEXAHQ108
created: 2026-11-08T22:06:58.090Z
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
delegated_by: seat:01M878FXVG76NYQYEEEXAHQ108
id: 01M6YK6GPVXKPP8Q8VZEDDSG7N
number: 93377961366408
reconciliation:
- by: seat:01M878FXVG76NYQYEEEXAHQ108
  disposition: implemented
  evidence:
  - commit 1ad7798ef — harness pin d2e2a03, make test-harness-source 841/841
  - commit 2d6fb00a2 — own configuration
  fresh_accepted: false
  kind: historical
  op_id: 01M878KB60N62FFQNDD3Z7VVT3
  previous_status: open
  reason: Harness suite green at pin, run in its own configuration.
role: executor
status: done
swarm: 01M649102FBS60QWNPZS5VAGF9
title: 'Harness test health at f848c03f: triage ~39 integration tests that fail outside their canonical Make env'
type: task
updated: 2026-11-24T17:10:34.438Z
updated_by: seat:01M878FXVG76NYQYEEEXAHQ108
verify: make test
provenance:
  digest: sha256:0d5662758347a4462d43db7693142278f49869b741733ef9bea40ea833088f91
  frontmatter_digest: sha256:473cc57c5ea3a90247ca9e985963b3f35143b81a65e0e64ce7fe339362c7dbe1
  signer: seat:01M878FXVG76NYQYEEEXAHQ108
  sig: CtMf6CXiQXTHLWg2C+XLjgh+rwFZTNno+MT6QUtRMxDe2q0VU1/2udg3mxAoWcf3dtY2BpiZv1LWXx8O/dOyCQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1873
  event:
    id: 01M878KCB0X4CHJJPD8530T65C
    sequence: 1873
    hash: c7a516e512f9a96b60dfc375f8ed951c25d8b93cf4bb43c2761c113fdac36d49
---
ROLE: EXECUTOR

## Problem
When term's wide land ran the harness package's own integration tests at harness f848c03f (from term, without the harness Makefile environment), about 39 failed (list in /tmp/dream-q3-land.log; copy it before /tmp is cleared). Their real status on harness main under the harness repo's OWN canonical targets is unknown. Some are known: 3 gui_render known defects and 4 gui_rich KNOWN-DEFECT ignores. The dream_c1 tests need DREAM_C1_RUNNER.

## Goal
In the bboxy450-harness repository, run each failing family under its canonical Make target (with its required environment and features) on harness main. For each one:
- passes under its canonical target: record that it is environment-only;
- fails: fix it, or add a KNOWN-DEFECT ignore pointing to a specific fix task.
Then make sure the harness land gate runs every integration-test family with its required environment, so main cannot regress silently.

## Scope
Worktree, allowed paths and verify are set at dispatch.
