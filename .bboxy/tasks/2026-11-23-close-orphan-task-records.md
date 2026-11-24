---
allowed_paths:
- crates/bbt-mux
- verification
assignee: seat:01M8604N98T1PM5801QP4JG51R
authorship:
  kind: seat
  process_generation: '1'
  public_key: bGMqsRvbQXvEDVnOwRt47b6crPtdbIu8ioBX1dU/YNs=
  signer: seat:01M85BW3PMN3QBFPPMWBS866RA
created: 2026-11-23T23:41:12.572Z
created_by: seat:01M85BW3PMN3QBFPPMWBS866RA
delegated_by: seat:01M85BW3PMN3QBFPPMWBS866RA
id: 01M85CHXH7C2W7Y1GKX83XDE12
number: 1794292861627824
role: executor
status: claimed
swarm: 01M7P04CW1T6KY8CMSS6DQM12Q
title: Let the operator (sudo) close task records whose swarm is unrecorded and whose seats are purged
type: task
updated: 2026-11-24T05:23:32.647Z
updated_by: seat:01M85BW3PMN3QBFPPMWBS866RA
verify: cargo nextest run -p bbt-mux && make test-formal
provenance:
  digest: sha256:76a3717f48cd7459724266f6fda1eec17130f5875e12a42e9f1ca97eb3afa945
  frontmatter_digest: sha256:2fbb5672cf7d79f9599f953a49850fd42ad8dd39815756fe2305b18423b4e7bc
  signer: seat:01M85BW3PMN3QBFPPMWBS866RA
  sig: 4DjtI08cBwcDp153YNjWnVTSxYwI+hKBLwBSFacmDBrJdSvEb2yE339aOnpiHAhBfq7HFJlUlkyFWTXjpeKCBA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1575
  event:
    id: 01M8604RVW3NBXZQ5NN58B4V6N
    sequence: 1575
    hash: e614d311d851da59c22704cb9350d5662c72a31ef773035bcd2387090a6223a1
---
Our stand: subtract before you add; report what you removed.

## Evidence (2026-11-23, observed)
10 accepted records carry no `swarm` and name only purged seats (assignee `seat:01M649QP6AMVQWCG28X5MQP3JR` / `01M649SZ31YZ93Z4VGP48Z0KH6`, delegator `seat:01M649MS05FAD5CY9CS0Q07H76`):
`tasks/2026-10-29-{crate-naming-classification,identity-records-night-readiness,issue-49-live-swarm-template,issue-72-final-present-contract,native-records-boundary,native-records-current-interface,normal-ci-disk-allocation,read-query-id-contract,seat-diagnostics-producer-contract,seat-recovery-diagnostics-contract}.den`.
`crates/bbt-mux/src/mcp/work_items.rs` `fn authorize` (~1255-1280) refuses every caller: a seat gets "task swarm is ambiguous"; `Actor::Operator` gets "task swarm requires an assignee or recorded swarm". So no door can ever close them.
The other 74 retired-swarm tasks were closed today via sudo_spawn + sudo_as in each retired swarm.

## Do
1. In `authorize`, when no swarm is recorded and every named seat is absent from the registry, let only the operator authority (operator door or a `sudo` seat's own call) act on `task_close` / `task_reconcile`. Live or recorded-swarm tasks keep today's rule. No new flag or concept.
2. Tests: orphan accepted record closes as operator; same record refuses an ordinary Oracle; a record naming a live seat in another swarm still refuses (broken control: drop the "all seats absent" guard → that test fails).
3. Update the task-authority contract in `verification/contracts.toml` (and model if one covers it) with the negative control.
4. `make land`. Do not close the 10 records yourself.