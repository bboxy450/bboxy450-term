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
updated: 2026-11-24T07:27:09.360Z
updated_by: seat:01M85BW3PMN3QBFPPMWBS866RA
verify: cargo nextest run -p bboxy450-term-mux
verify_revision: 1
provenance:
  digest: sha256:515d802ffd28a875ad9723bcabc5144ecb2f4d042e9b553e0be2ad3484b297f7
  frontmatter_digest: sha256:02e7d5898644315f1559b0367115ac3ef4a04d6aaf78abfdd12dd7b4cf319947
  signer: seat:01M85BW3PMN3QBFPPMWBS866RA
  sig: y87HLtPV+xpmQwW6teOrstE4aO/bZEvOgLzCosyegFB0SdTPVlivN68mh/oFAFLaAFYSXSekGbYqZTH1RQU8Cg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1633
  event:
    id: 01M86773T0FY17K1JH94VCHJBV
    sequence: 1633
    hash: 84b291f62922860f0541647d3fc397ba1ad575f614067b058a95d1bab444807b
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