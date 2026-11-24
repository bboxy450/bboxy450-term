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
created: 2026-11-23T23:41:13.905Z
created_by: seat:01M85BW3PMN3QBFPPMWBS866RA
delegated_by: seat:01M85BW3PMN3QBFPPMWBS866RA
id: 01M85CHY7DXP4AF20QJX3K1BQ0
number: 1794292861627825
role: executor
status: claimed
swarm: 01M7P04CW1T6KY8CMSS6DQM12Q
title: 'sudo_as KB tools fail from python_execute: execution_context_changed / transport unavailable'
type: task
updated: 2026-11-24T07:23:27.778Z
updated_by: seat:01M85BW3PMN3QBFPPMWBS866RA
verify: cargo nextest run -p bbt-mux
provenance:
  digest: sha256:515d802ffd28a875ad9723bcabc5144ecb2f4d042e9b553e0be2ad3484b297f7
  frontmatter_digest: sha256:07741b246c3f8356323ab5a1692fae74f8b9aeb9ce1ca0d55de20e9ebdc8a053
  signer: seat:01M85BW3PMN3QBFPPMWBS866RA
  sig: L2URADwYuRmR7an+8EiZYMyTmABkslrlpra3zbXE7M4t/Efk374N7TeC8D/WO+fmhAS4iaYbcmD/n8t02PUaDg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1625
  event:
    id: 01M8670BA4SEZ34VMJHKVHX3GB
    sequence: 1625
    hash: 064815a84703fca8597087e4defe8ae85f3935bb56c3fe21b68b4e54f09b5462
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