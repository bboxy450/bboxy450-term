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
closed:
  by: seat:01M85BW3PMN3QBFPPMWBS866RA
  op_id: 01M86V25ZDB3MN7QWGG122EPVN
created: 2026-11-24T10:05:24.043Z
created_by: seat:01M85BW3PMN3QBFPPMWBS866RA
delegated_by: seat:01M85BW3PMN3QBFPPMWBS866RA
id: 01M86G8VBBFZFF5XH8NNTW9KNM
number: 1794292861627875
report:
  commit: 739879d4f8bd5ba0716dc3e0d57a96d2c6c7f582
  op_id: 01M86T0B84QS0T60PH546679QE
  report: '[[reports/2026-11-24-python-native-suite-repair.den]]'
  reported_by: seat:01M86NEZ4J2DKQE0MMGA2V5SNC
  summary: '`make test-python-execute-native` passed all five tests on signed commit `739879d4f`, and `make land` exited 0. Source bisects traced the stale `emitted[0]` lookup to harness `65522f6` and term pin `e3bcaa60c`. The test now reads the default Python result, inspects the held run without waiting, and opts into detailed cancellation receipts. Removed the unused orient tool list and the native suite’s duplicate chart selection. The full report is indexed in the local KB.'
  verify_revision: 0
role: executor
status: done
swarm: 01M7P04CW1T6KY8CMSS6DQM12Q
title: 'make test-python-execute-native is red on main: managed-worktree orient has no hints; chart test needs its install'
type: task
updated: 2026-11-24T13:14:01.307Z
updated_by: seat:01M85BW3PMN3QBFPPMWBS866RA
verify: make test-python-execute-native
verify_result:
  command: make test-python-execute-native
  evidence:
  - criterion: Identify the orient regression and exercise the current contract
    reference: Harness 65522f6, term e3bcaa60c, crates/bbt-mux/src/mcp/hints.rs:232 and :448, crates/bbt-mux/tests/mcp/python_execution.rs:482
  - criterion: Give the installed chart test one Make target
    reference: Makefile:2927 and :2940, ignored-test listing found one chart witness
  - criterion: Pass the native suite and land
    reference: Nextest 5e727db6-acc2-4a1d-80bf-b431e6b656de passed 5/5, target/python-native-verification/land.log records LANDED 739879d4f8bd5ba0716dc3e0d57a96d2c6c7f582, exit 0
  exit_code: 0
provenance:
  digest: sha256:6d299bccd094dafc12e15d62944c998aeb8f2fc1194bfd73bf2eaaa58e2b2af8
  frontmatter_digest: sha256:f6e3daf0551801e1d394a80606daa42f4b6166d8cd50810d81a53b1bc62e1937
  signer: seat:01M85BW3PMN3QBFPPMWBS866RA
  sig: 0iMTyR5CpbmSYcMrPEy0XbgTBOC2R39ooe5KXHCyI4wirUq/627UeuulTpkXn4ktjJ0FK8uv/wfAtOGAJV8MAw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1754
  event:
    id: 01M86V27TXFMH3CHYWE0MRHQ07
    sequence: 1754
    hash: cc242976970d1261465f247b03a0199c22ae5337a4d069df85627c4ac3422097
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

## Report from seat:01M86NEZ4J2DKQE0MMGA2V5SNC

`make test-python-execute-native` passed all five tests on signed commit `739879d4f`, and `make land` exited 0. Source bisects traced the stale `emitted[0]` lookup to harness `65522f6` and term pin `e3bcaa60c`. The test now reads the default Python result, inspects the held run without waiting, and opts into detailed cancellation receipts. Removed the unused orient tool list and the native suite’s duplicate chart selection. The full report is indexed in the local KB.

```json
{
  "op_id": "01M86T0B84QS0T60PH546679QE",
  "commit": "739879d4f8bd5ba0716dc3e0d57a96d2c6c7f582",
  "report": "[[reports/2026-11-24-python-native-suite-repair.den]]",
  "verify_result": {
    "command": "make test-python-execute-native",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Identify the orient regression and exercise the current contract",
        "reference": "Harness 65522f6, term e3bcaa60c, crates/bbt-mux/src/mcp/hints.rs:232 and :448, crates/bbt-mux/tests/mcp/python_execution.rs:482"
      },
      {
        "criterion": "Give the installed chart test one Make target",
        "reference": "Makefile:2927 and :2940, ignored-test listing found one chart witness"
      },
      {
        "criterion": "Pass the native suite and land",
        "reference": "Nextest 5e727db6-acc2-4a1d-80bf-b431e6b656de passed 5/5, target/python-native-verification/land.log records LANDED 739879d4f8bd5ba0716dc3e0d57a96d2c6c7f582, exit 0"
      }
    ]
  }
}
```
