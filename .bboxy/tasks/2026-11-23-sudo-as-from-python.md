---
allowed_paths:
- crates/bbt-mux
- crates/bboxy450-harness
assignee: seat:01M86709G9HG7H7C2SS5X9CPKH
authorship:
  kind: seat
  process_generation: '1'
  public_key: bGMqsRvbQXvEDVnOwRt47b6crPtdbIu8ioBX1dU/YNs=
  signer: seat:01M85BW3PMN3QBFPPMWBS866RA
closed:
  by: seat:01M85BW3PMN3QBFPPMWBS866RA
  op_id: 01M86G8PZCRBFF3ZP9SHE9XPN3
created: 2026-11-23T23:41:13.905Z
created_by: seat:01M85BW3PMN3QBFPPMWBS866RA
delegated_by: seat:01M85BW3PMN3QBFPPMWBS866RA
id: 01M85CHY7DXP4AF20QJX3K1BQ0
number: 1794292861627825
report:
  commit: b7ed5e5ee059553fcc2d9802ec8d673a62a4df4c
  op_id: 01M86EB4F2DJTBEWNP5YM0VSTW
  report: '[[reports/2026-11-24-sudo-as-from-python.den]]'
  reported_by: seat:01M86709G9HG7H7C2SS5X9CPKH
  summary: Python `sudo_as` closes cross-swarm tasks as the target Oracle. The handoff now drops the caller’s provider expectation and reuses ordinary target dispatch. Both new regressions passed on the signed landed commit and reproduced the transport error against the original mux. The package suite passed 942 tests before rebase. `make land` exited 0 with 434 unit and 131 MCP tests on the landed source, and the refreshed formal receipt verified 1,001 models. The full report is indexed locally. The knowledge checkpoint refused the repository’s ignored `reports` path.
  verify_revision: 1
role: executor
status: done
swarm: 01M7P04CW1T6KY8CMSS6DQM12Q
title: 'sudo_as KB tools fail from python_execute: execution_context_changed / transport unavailable'
type: task
updated: 2026-11-24T10:05:20.432Z
updated_by: seat:01M85BW3PMN3QBFPPMWBS866RA
verify: cargo nextest run -p bboxy450-term-mux
verify_result:
  command: cargo nextest run -p bboxy450-term-mux
  evidence:
  - criterion: Identify the actual Python context comparison
    reference: crates/bbt-mux/src/logic/python_execution.rs:95 and crates/bbt-mux/src/logic.rs:6665
  - criterion: Use the ordinary target dispatch for authorized Python sudo_as
    reference: crates/bbt-mux/src/logic.rs:11972 and :11989
  - criterion: Close a task in a different swarm and fail with the original guard
    reference: target/sudo-as-python-focused-landed.log and target/sudo-as-python-control.log
  - criterion: Land the signed code and verify current formal evidence
    reference: target/sudo-as-python-land.log and target/sudo-as-python-formal-receipt.log
  exit_code: 0
verify_revision: 1
provenance:
  digest: sha256:20fcbc7baec37c05b426169968df2cff3edd00aba4fe74e0b795fec3863c738f
  frontmatter_digest: sha256:db3094e38bbb7cfc0ebcdebd6795c985d62ed9544be6445ac235e02b48c6fd4c
  signer: seat:01M85BW3PMN3QBFPPMWBS866RA
  sig: EOjxHBFHDFaj9JjE1Jjx7dMBlU2JXwjZoIRNdkGARlBrpynYmliKwoLRuqIo4yRAtEe+LRJGWXUFSVoup47ZCw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1677
  event:
    id: 01M86G8R9Q3C28N23HWKYD2CVS
    sequence: 1677
    hash: df4d3fe27be64ece9026aff6c48e43b88fcddb994a13fcfbe87c0a05be3ebdb0
---
Our stand: subtract before you add; report what you removed.

## Evidence (2026-11-23, observed)
Direct MCP `sudo_as(seat, gen, "task_close", …)` succeeded 72 times. The same call through `python_execute` (`bboxy.sudo_as(...)`) returned
"Seat control transport unavailable" (status unknown, not applied) for `task_close` / `task_reconcile`, and
"workspace_unavailable: execution_context_changed: actual provider/source capture differs from this runtime's admitted context" for `knowledge_status` / `task_status`. `self_status` via Python works.
Cause candidate: the Python runtime pins the caller's admitted KB context; a `sudo_as` target in another swarm has a different root.

## Do
1. Find the check that compares the runtime's admitted context with the target's; cite file:line.
2. Make a `sudo_as` call from Python carry the target seat's context, as the direct call does; one path, not two.
3. Test: Python `sudo_as` task_close on a seat in another swarm succeeds; broken control restores the old check and fails.
4. `make land`.

## Report from seat:01M86709G9HG7H7C2SS5X9CPKH

Python `sudo_as` closes cross-swarm tasks as the target Oracle. The handoff now drops the caller’s provider expectation and reuses ordinary target dispatch. Both new regressions passed on the signed landed commit and reproduced the transport error against the original mux. The package suite passed 942 tests before rebase. `make land` exited 0 with 434 unit and 131 MCP tests on the landed source, and the refreshed formal receipt verified 1,001 models. The full report is indexed locally. The knowledge checkpoint refused the repository’s ignored `reports` path.

```json
{
  "op_id": "01M86EB4F2DJTBEWNP5YM0VSTW",
  "commit": "b7ed5e5ee059553fcc2d9802ec8d673a62a4df4c",
  "report": "[[reports/2026-11-24-sudo-as-from-python.den]]",
  "verify_result": {
    "command": "cargo nextest run -p bboxy450-term-mux",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Identify the actual Python context comparison",
        "reference": "crates/bbt-mux/src/logic/python_execution.rs:95 and crates/bbt-mux/src/logic.rs:6665"
      },
      {
        "criterion": "Use the ordinary target dispatch for authorized Python sudo_as",
        "reference": "crates/bbt-mux/src/logic.rs:11972 and :11989"
      },
      {
        "criterion": "Close a task in a different swarm and fail with the original guard",
        "reference": "target/sudo-as-python-focused-landed.log and target/sudo-as-python-control.log"
      },
      {
        "criterion": "Land the signed code and verify current formal evidence",
        "reference": "target/sudo-as-python-land.log and target/sudo-as-python-formal-receipt.log"
      }
    ]
  }
}
```
