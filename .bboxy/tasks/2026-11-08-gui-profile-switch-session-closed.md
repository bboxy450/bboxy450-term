---
type: task
title: 'Harness: GUI profile switch never emits SessionClosed for the old session (seat_socket 2 s timeout)'
role: executor
allowed_paths:
- src/frontends/gui/bridge.rs
- tests/support/seat_socket.rs
verify: make test-dream-c1
id: 01M6YGEFZQ0DFSMW3K2QPYH3GT
status: open
delegated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-08T21:18:52.768Z
created: 2026-11-08T21:18:52.768Z
number: 93377961366405
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
updated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
authorship:
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  kind: seat
  public_key: DbWCjlMnSNUcBZMB2JrdfJ36/iQgSFRbtz78XE3WlhU=
  process_generation: '1'
provenance:
  digest: sha256:f3e11e76481dd86e15391e0795d091ffd9ed3a6b3bfb2937a39747db726333ce
  frontmatter_digest: sha256:285469cabe37cd7410696daf97724c2e44eeb0fedcdf38f8ff5a39dafd8d7dfa
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  sig: 9ats5pH6jSDSITPk6rfYVupDCFH8+8MrFor6NArjf4nI9yA9PUkMDdjH1e20k9uXwPsuOBO8nAWgTQ7QSnaBBw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150835
  event:
    id: 01M6YGEHE72Y565TGSAXGSHE8E
    sequence: 150835
    hash: dd6fb9ef8b6b3db4113d9ffa295f8f1c8bf90eb2ef42bd35e1aad9f6e7220ce1
---
ROLE: EXECUTOR

## Problem
Harness GUI profile switch: the fixture at tests/support/seat_socket.rs:607 times out at old_session_closed_wait (2 s). The child read the switch command (enqueued Ok), but no SessionClosed arrived for the old session; only old-session ItemStarted/Completed events were seen. It was found during Q2's exact four-chain on fd28caeb (log 8fc119e4...3b66; evidence in Q2's E/retained-fd28caeb-FAILED-seat-socket-v1/). Whether it is pre-existing on harness main is being classified by Q2.

## Goal
Find out why the old session does not emit SessionClosed on a GUI profile switch, in src/frontends/gui/bridge.rs and the engine session lifecycle. Fix it, keeping the 2 s bound and the assertions unchanged, and remove any KNOWN-DEFECT ignore. Write a deterministic test that reproduces it under load.

## Scope
Worktree, allowed paths and verify are set at dispatch (bboxy450-harness repository).
