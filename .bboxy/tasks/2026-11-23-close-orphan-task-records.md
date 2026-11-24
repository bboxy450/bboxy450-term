---
allowed_paths:
- crates/bbt-mux
- verification
- crates/bboxy450-term/tests/formal_contracts/task_amend_model.rs
- crates/bboxy450-term/tests/formal_contracts/models.rs
assignee: seat:01M8604N98T1PM5801QP4JG51R
authorship:
  kind: seat
  process_generation: '1'
  public_key: bGMqsRvbQXvEDVnOwRt47b6crPtdbIu8ioBX1dU/YNs=
  signer: seat:01M85BW3PMN3QBFPPMWBS866RA
closed:
  by: seat:01M85BW3PMN3QBFPPMWBS866RA
  op_id: 01M86BF1J3S8J1CHGGR544D65D
created: 2026-11-23T23:41:12.572Z
created_by: seat:01M85BW3PMN3QBFPPMWBS866RA
delegated_by: seat:01M85BW3PMN3QBFPPMWBS866RA
id: 01M85CHXH7C2W7Y1GKX83XDE12
number: 1794292861627824
report:
  commit: f445ce8f680dfa10ab1d69629a37722122c45bf4
  op_id: 01M86B1YT23E1RRGAXWXVDGM7C
  report: '[[reports/2026-11-24-close-orphan-task-records.den]]'
  reported_by: seat:01M8604N98T1PM5801QP4JG51R
  summary: Landed signed commit f445ce8f680dfa10ab1d69629a37722122c45bf4 on origin/main. It replaces the blanket missing-swarm refusal for orphan closure and reconciliation with an operation-specific operator and sudo check. The MCP fixture closes the accepted orphan, refuses an ordinary Oracle and refuses a live foreign seat. Dropping the absence guard makes that fixture fail. The mux suite passed 943 tests and the final landing gate passed with 1,001 native/TLC model verdicts. The worktree is clean.
  verify_revision: 1
role: executor
status: done
swarm: 01M7P04CW1T6KY8CMSS6DQM12Q
title: Let the operator (sudo) close task records whose swarm is unrecorded and whose seats are purged
type: task
updated: 2026-11-24T08:41:25.076Z
updated_by: seat:01M85BW3PMN3QBFPPMWBS866RA
verify: cargo nextest run -p bboxy450-term-mux && make test-formal
verify_result:
  command: cargo nextest run -p bboxy450-term-mux && make test-formal
  evidence:
  - criterion: Operator-authorized orphan closure and reconciliation
    reference: crates/bbt-mux/tests/mcp.rs:7760 and crates/bbt-mux/src/mcp/work_items_inventory_tests.rs:970
  - criterion: Ordinary Oracle and live foreign seat refusals
    reference: crates/bbt-mux/tests/mcp.rs:7760
  - criterion: Broken control removes the registry-absence guard
    reference: Nextest e22c78ef-a3d3-4bf2-a721-d08f7741dd15 exited 100. The live foreign assignee fixture returned done.
  - criterion: Current-source formal and landing gates
    reference: Landed formal receipt 971f4030a98b0d04d388b0ae56cd1e9bc0fb0c5bb83141b6d0a4f20cd0ddd267. BBOXY_CACHE_MODE=off make land exited 0.
  exit_code: 0
verify_revision: 1
provenance:
  digest: sha256:60b061e63b27ba05291e30009f31668a06b12db0117ee70dd3c74d81d0e130ce
  frontmatter_digest: sha256:0a19792b85f860e294989f048f0c274c96a156bc9e38e8cef25fcda76a5132a3
  signer: seat:01M85BW3PMN3QBFPPMWBS866RA
  sig: hHZQNVd9Q8HvHtxh3oCLYaW2Y6zisBGRCC1AFVWTe5CUQN9v2CLkUyfsR8pvFGrTDJb/mY6KoQZ+vrJhdgldAw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1650
  event:
    id: 01M86BF302ZRX5SFX7K2MAT0EP
    sequence: 1650
    hash: f31c0e1c89de4c20014185f4324002462d2fabc9051e245aad4ea0a639218377
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

## Report from seat:01M8604N98T1PM5801QP4JG51R

Landed signed commit f445ce8f680dfa10ab1d69629a37722122c45bf4 on origin/main. It replaces the blanket missing-swarm refusal for orphan closure and reconciliation with an operation-specific operator and sudo check. The MCP fixture closes the accepted orphan, refuses an ordinary Oracle and refuses a live foreign seat. Dropping the absence guard makes that fixture fail. The mux suite passed 943 tests and the final landing gate passed with 1,001 native/TLC model verdicts. The worktree is clean.

```json
{
  "op_id": "01M86B1YT23E1RRGAXWXVDGM7C",
  "commit": "f445ce8f680dfa10ab1d69629a37722122c45bf4",
  "report": "[[reports/2026-11-24-close-orphan-task-records.den]]",
  "verify_result": {
    "command": "cargo nextest run -p bboxy450-term-mux && make test-formal",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Operator-authorized orphan closure and reconciliation",
        "reference": "crates/bbt-mux/tests/mcp.rs:7760 and crates/bbt-mux/src/mcp/work_items_inventory_tests.rs:970"
      },
      {
        "criterion": "Ordinary Oracle and live foreign seat refusals",
        "reference": "crates/bbt-mux/tests/mcp.rs:7760"
      },
      {
        "criterion": "Broken control removes the registry-absence guard",
        "reference": "Nextest e22c78ef-a3d3-4bf2-a721-d08f7741dd15 exited 100. The live foreign assignee fixture returned done."
      },
      {
        "criterion": "Current-source formal and landing gates",
        "reference": "Landed formal receipt 971f4030a98b0d04d388b0ae56cd1e9bc0fb0c5bb83141b6d0a4f20cd0ddd267. BBOXY_CACHE_MODE=off make land exited 0."
      }
    ]
  }
}
```
