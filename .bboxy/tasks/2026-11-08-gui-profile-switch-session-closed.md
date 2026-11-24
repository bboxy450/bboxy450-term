---
allowed_paths:
- src/frontends/gui/bridge.rs
- tests/support/seat_socket.rs
authorship:
  kind: seat
  process_generation: '1'
  public_key: 0X00kpb5zTGUeot1iS3xss6zajL5tcixiAgioL6WETw=
  signer: seat:01M878FXVG76NYQYEEEXAHQ108
created: 2026-11-08T21:18:52.768Z
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
delegated_by: seat:01M878FXVG76NYQYEEEXAHQ108
id: 01M6YGEFZQ0DFSMW3K2QPYH3GT
number: 93377961366405
reconciliation:
- by: seat:01M878FXVG76NYQYEEEXAHQ108
  disposition: superseded
  evidence:
  - 'successor: tasks/2026-11-24-harness-test-debt.md step 4'
  fresh_accepted: false
  kind: historical
  op_id: 01M878KCDPZYSPB6NK0H2R6KZE
  previous_status: open
  reason: Carried into the live term swarm.
role: executor
status: done
swarm: 01M649102FBS60QWNPZS5VAGF9
title: 'Harness: GUI profile switch never emits SessionClosed for the old session (seat_socket 2 s timeout)'
type: task
updated: 2026-11-24T17:10:35.459Z
updated_by: seat:01M878FXVG76NYQYEEEXAHQ108
verify: make test-dream-c1
provenance:
  digest: sha256:f3e11e76481dd86e15391e0795d091ffd9ed3a6b3bfb2937a39747db726333ce
  frontmatter_digest: sha256:35660a49ce3d44bca06289d499a09a305f9c4f3d6d2cfe95bbfcba4809e70cef
  signer: seat:01M878FXVG76NYQYEEEXAHQ108
  sig: a1ihmNG0ZZ6tN/hsbOVymCXmkIQUGHXOmP37Le4t89SJkCUZWQkmYMEJpya0dtOF8ItzR5NowJtFTMfaCM/rCg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1875
  event:
    id: 01M878KDBEAHZ9QRWQCQC5P5M0
    sequence: 1875
    hash: fe4481d39be5ca6e055fee3cd9f46130a3735f6fee93dd61474a1ed3eb25edc0
---
ROLE: EXECUTOR

## Problem
Harness GUI profile switch: the fixture at tests/support/seat_socket.rs:607 times out at old_session_closed_wait (2 s). The child read the switch command (enqueued Ok), but no SessionClosed arrived for the old session; only old-session ItemStarted/Completed events were seen. It was found during Q2's exact four-chain on fd28caeb (log 8fc119e4...3b66; evidence in Q2's E/retained-fd28caeb-FAILED-seat-socket-v1/). Whether it is pre-existing on harness main is being classified by Q2.

## Goal
Find out why the old session does not emit SessionClosed on a GUI profile switch, in src/frontends/gui/bridge.rs and the engine session lifecycle. Fix it, keeping the 2 s bound and the assertions unchanged, and remove any KNOWN-DEFECT ignore. Write a deterministic test that reproduces it under load.

## Scope
Worktree, allowed paths and verify are set at dispatch (bboxy450-harness repository).
