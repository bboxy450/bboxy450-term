---
type: task
title: Term wake-up C1 private-process test misses framing readiness window under load
role: executor
allowed_paths:
- crates/bboxy450-harness-wake-up/tests/dream_c1.rs
- scripts/qos-policy.py
verify: make test-dream-mcp
id: 01M6Z8PN901DXTECVHJ0GY5YTW
status: open
delegated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-09T04:22:45.380Z
created: 2026-11-09T04:22:45.380Z
number: 93377961366432
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
updated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
authorship:
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  kind: seat
  public_key: DbWCjlMnSNUcBZMB2JrdfJ36/iQgSFRbtz78XE3WlhU=
  process_generation: '1'
provenance:
  digest: sha256:7ebd4d6aa6785ede35be8d3c012a07ac91ba6e5116766eb517f4ead5ea1e45b3
  frontmatter_digest: sha256:bf0fa9628861d56eb4aa1591fd057133c8660380e41a0bfc9d5c8d5c5831c36c
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  sig: XnjUGK/UJCRk/LEPl5Bf0vAGdZyBxD65NYIwi4310GemALkaugcp3eByfP6j3aS0QugEh3cEf5G9TcJoAE7yDA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150926
  event:
    id: 01M6Z8PP3SP1YX5AY8DSM5QMN4
    sequence: 150926
    hash: f1cb6ee186a37a9857edeab8f2e10bb5508668885c72629bef9b312be12aa14c
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
