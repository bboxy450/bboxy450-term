---
allowed_paths:
- crates/bbt-mux
- crates/bboxy450-term
- Makefile
assignee: seat:01M86NEZ4J2DKQE0MMGA2V5SNC
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
role: executor
status: claimed
swarm: 01M7P04CW1T6KY8CMSS6DQM12Q
title: 'make test-python-execute-native is red on main: managed-worktree orient has no hints; chart test needs its install'
type: task
updated: 2026-11-24T11:36:12.785Z
updated_by: seat:01M85BW3PMN3QBFPPMWBS866RA
verify: make test-python-execute-native
provenance:
  digest: sha256:1d8357086062b6fd663633cb0f3a8e95ce0c38272accea8a73371407646aeb4f
  frontmatter_digest: sha256:97232807c7e0d4c19adbdf66a3faf2e6c4adac8c611d47070c3dd478ce2e9c70
  signer: seat:01M85BW3PMN3QBFPPMWBS866RA
  sig: kNCvJBWHBh2L9PSHYu99Xk+UHt2ERCMXqMkKFQSdueiQnUKhwhSINLC1USlAPW5VYhTEhw2YOLGV9+yNFRuiBQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1706
  event:
    id: 01M86NF52WMK9QF08K8TE9CBM7
    sequence: 1706
    hash: 71f7e824eb6cd9c9e2d59c707c094071bb7e6c490adf45e2d612369bb8dbe182
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