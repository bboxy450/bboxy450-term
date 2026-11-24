---
allowed_paths:
- crates/bboxy450-harness-wake-up/tests/dream_c1.rs
- scripts/qos-policy.py
authorship:
  kind: seat
  process_generation: '1'
  public_key: 0X00kpb5zTGUeot1iS3xss6zajL5tcixiAgioL6WETw=
  signer: seat:01M878FXVG76NYQYEEEXAHQ108
created: 2026-11-09T04:22:45.380Z
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
delegated_by: seat:01M878FXVG76NYQYEEEXAHQ108
id: 01M6Z8PN901DXTECVHJ0GY5YTW
number: 93377961366432
reconciliation:
- by: seat:01M878FXVG76NYQYEEEXAHQ108
  disposition: implemented
  evidence:
  - commit 53a9e215f — dream_bootstrap.rs:28 WINDOW 2 s
  - crates/bboxy450-harness-wake-up/tests/dream_c1.rs:464 — not ignored
  fresh_accepted: false
  kind: historical
  op_id: 01M878KHFZ6MS6KFVTASWA114K
  previous_status: open
  reason: Window raised, recorded in ICD; test unignored.
role: executor
status: done
swarm: 01M649102FBS60QWNPZS5VAGF9
title: Term wake-up C1 private-process test misses framing readiness window under load
type: task
updated: 2026-11-24T17:10:40.768Z
updated_by: seat:01M878FXVG76NYQYEEEXAHQ108
verify: make test-dream-mcp
provenance:
  digest: sha256:7ebd4d6aa6785ede35be8d3c012a07ac91ba6e5116766eb517f4ead5ea1e45b3
  frontmatter_digest: sha256:a6fb58c3345a49b62114b0bdd4fc041e5d76bd7aa7fd4fa0151dab6bfa187ebf
  signer: seat:01M878FXVG76NYQYEEEXAHQ108
  sig: H+McEL1e8ufLT+S33fEVV3U0l8SyY87Ok5hKChnv+NIYNzuWslBkYF4d3Re2/rOb1jD+xunQ5R0b/1x19tpsAg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1885
  event:
    id: 01M878KJH4A02AQV5KBV5YE4YE
    sequence: 1885
    hash: c0c09b41c283c7b404e4f1931e2bccbf2ffbf7d28b8edb1bd2037be6661c34d2
---
ROLE: EXECUTOR

## Problem
bboxy450-harness-wake-up::dream_c1::actual_private_process_clears_authority_environment_and_uses_fixed_argv (crates/bboxy450-harness-wake-up/tests/dream_c1.rs:361) intermittently misses its "original framing readiness window": the owned protocol fixture is not ready in time, and stderr is unavailable after the actual reap. It passed in a 2018-test land run, then failed in the next one (/tmp/dream-q3-645f-coverage-land.log, sha256 ec1bc0ce...8813) under shared load.

## Goal
- Make the readiness failure diagnosable: retain the child's stderr and phase before reaping.
- Decide whether the window is a product law or a fixture assumption. If it is a fixture assumption, base readiness on an explicit ready signal rather than wall time. If it is law, run the test in the default-QoS timed lane.
- Remove any KNOWN-DEFECT ignore. Change no budgets or assertions without a written reason.

## Scope
bboxy450-term, crates/bboxy450-harness-wake-up/tests/dream_c1.rs plus its fixture and scripts/qos-policy.py. Worktree and verify are set at dispatch.
