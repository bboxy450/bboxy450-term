---
allowed_paths:
- crates/bbt-mux
- crates/bboxy450-term
- Makefile
authorship:
  kind: seat
  process_generation: '1'
  public_key: bGMqsRvbQXvEDVnOwRt47b6crPtdbIu8ioBX1dU/YNs=
  signer: seat:01M85BW3PMN3QBFPPMWBS866RA
created: 2026-11-24T10:05:24.043Z
created_by: seat:01M85BW3PMN3QBFPPMWBS866RA
delegated_by: seat:01M85BW3PMN3QBFPPMWBS866RA
id: 01M86G8VBBFZFF5XH8NNTW9KNM
number: 1794292861627875
status: open
swarm: 01M7P04CW1T6KY8CMSS6DQM12Q
title: 'make test-python-execute-native is red on main: managed-worktree orient has no hints; chart test needs its install'
type: task
updated: 2026-11-24T10:05:24.043Z
updated_by: seat:01M85BW3PMN3QBFPPMWBS866RA
verify: make test-python-execute-native
provenance:
  digest: sha256:1d8357086062b6fd663633cb0f3a8e95ce0c38272accea8a73371407646aeb4f
  frontmatter_digest: sha256:e95cdaa93718007bc779876e4380f4ca8eafbad4eec1cefed22726a89cd45a9c
  signer: seat:01M85BW3PMN3QBFPPMWBS866RA
  sig: YWOZl+zrdiFAwFBTCzq/i+QZqw3iHxu1L8TN/J2PfHh0biiuHkUOaxctL1JjztvftbnShJpGYKM3tVZl2cmXAg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1679
  event:
    id: 01M86G8VW0ZMJ83S1RQWP2NJ45
    sequence: 1679
    hash: 840a0e96f1695c9fb8d10080c2d05ad4ff2d14632e68b897b32231d8f7c2682f
---
Our stand: subtract before you add; report what you removed.

## Evidence (2026-11-24, Oracle, observed)
`make test-python-execute-native` on origin/main b7ed5e5ee exit 2: 6 run, 4 passed, 2 failed.
- `mcp::python_execution_tests::python_managed_worktree_executes_python_and_mcp_against_canonical_kb` panics at
  `crates/bbt-mux/tests/mcp/python_execution.rs:489` (`oriented…["emitted"][0]["hints"]` is None). Same failure on the parent 2d6841da8, so it predates b7ed5e5ee.
- `python_chart_artifact_observed_bytes_survive_harness_cleanup_and_native_restart` panics at `:277` "private chart install": it needs
  `BBOXY_CHART_TEST_INSTALL`, which only `make test-chart-artifacts` provides; this target runs it without one.

## Do
1. Find when orient stopped emitting `hints` through `mcp.call('native','orient')` (git bisect the test); fix the product or the test, whichever is wrong per the orient contract; cite file:line.
2. Make `test-python-execute-native` either provide the chart install or exclude that test (it has its own target). One owner per test.
3. `make test-python-execute-native` exit 0; `make land`.