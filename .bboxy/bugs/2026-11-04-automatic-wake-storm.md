---
type: bug
title: Self-authored task notices and automatic wake storms consume inference
status: reported
owner: seat:01M6MADHFD19P4TKM7P7X6QESA
id: 01M6ME2N3FY6RTM7M91GW9HWA6
delegated_by: seat:01M6MADHFD19P4TKM7P7X6QESA
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-05T00:37:36.934Z
created: 2026-11-04T23:25:02.249Z
number: 93377961363621
created_by: seat:01M6MADHFD19P4TKM7P7X6QESA
updated_by: seat:01M6MADHFD19P4TKM7P7X6QESA
authorship:
  signer: seat:01M6MADHFD19P4TKM7P7X6QESA
  kind: seat
  public_key: Rg4zwPSh/8Jytfz7nuhB5zfZyLtAZj9dFmnXz9d411U=
  process_generation: '1'
verify_revision: 1
verify: make land
artifacts:
- '[[01M6MJ4RX3HH93AAS66QZY6KKP]]'
verify_result:
  command: make land
  exit_code: 0
  evidence:
  - criterion: Self-loop prevention and durable recipient circuit
    reference: crates/bbt-mux/src/messaging/ledger/circuit.rs:52; .bboxy/runs/2026-11-04-wake-circuit-targeted.json
  - criterion: Source-bound formal parity and canonical signed landing
    reference: .bboxy/runs/2026-11-04-wake-circuit-landed.json; SHA256 0da4d1d20ec441d27590f95be4520547b89ae4477749d4f72a6551d9df791421
  - criterion: Unattended native/process behavior and idle cancellation
    reference: .bboxy/runs/2026-11-04-wake-circuit-idle.json; make smoke idle frames=0 callbacks=0 over 10.005s
  - criterion: Approved signed installation and preserved running owners
    reference: .bboxy/runs/2026-11-04-wake-circuit-landed.json; installed BBOXYSourceRevision=5b6110d36a6c4a0c0315a70528b86c9d8bffe076
report:
  summary: 'Landed and installed the durable automatic-wake circuit breaker: self notices stay inbox-only; a ninth automatic admission within a recipient''s 60-second window latches further wakes until a new direct operator message. Signed source 5b6110d3 passed make land (493 app/353 mux/15 corpus; all 387 native/Yarn/Java models), 44 focused tests, causal guard mutation, 39 process boundary tests and zero-callback headless smoke. Full committed report contains hashes, bounds and prerequisite repairs. Existing backend/session remains running; activation requires starting the backend on the installed new binary. No task_accept tool is exposed in this catalogue.'
  commit: 5b6110d36a6c4a0c0315a70528b86c9d8bffe076
  reported_by: seat:01M6MADHFD19P4TKM7P7X6QESA
  op_id: 01M6MJ7H7EQ287NMX2EB9BFP3F
  verify_revision: 1
  report: '[[reports/2026-11-04-wake-circuit-breaker.md]]'
provenance:
  digest: sha256:4c132714182524bbdf0ba5150fd5ca8b56406e31b9e132402586b8beaae248a3
  frontmatter_digest: sha256:cea4c7f85711205789b1df85470028e35721f1bde10fceb77ab4f42636042f5d
  signer: seat:01M6MADHFD19P4TKM7P7X6QESA
  sig: ZEv8DGViT1Polvn3y/4bHPYooe0H6dVPdOjbOIOPjTJz3XCwPiQEzmlGKinGERGKOMJ8UHU+ZtgWKOAC7/X1DA==
  log: .bboxy/log/2026-11-05.jsonl#131559
---
Direct operator implementation request to Sable Quill on 2026-11-04; current scope and circuit contract are recorded in [[notes/2026-11-04-wake-circuit-breaker]]. This finding is owned and implemented by this solo seat; no delegated assignment or additional seat is created.

Observed: Vesper's task delegated_by and assignee were the same seat. Native task operations generated 24 self notifications (one delegation, 22 amendments, one report), which became 24 redundant turns after the long work turn. Counts and token deltas were matched against the actual message ledger and Codex transcript; no raw transcripts are committed.

Required outcome: suppress self-authored automatic wakes and implement a recipient-local durable automatic-wake circuit with retained inbox/audit data, bounded admissions, no reset through agent ACK/replay/restart, explicit operator rearm, diagnostics and finite model/API negative controls. Verify through make test-wake-circuit-breaker, mandatory native/Java formal verification, source license checks and the canonical signed landing gate. Keep backend execution and ongoing sessions intact; this request does not authorize terminating the operator's running backend.

## Report from seat:01M6MADHFD19P4TKM7P7X6QESA

Landed and installed the durable automatic-wake circuit breaker: self notices stay inbox-only; a ninth automatic admission within a recipient's 60-second window latches further wakes until a new direct operator message. Signed source 5b6110d3 passed make land (493 app/353 mux/15 corpus; all 387 native/Yarn/Java models), 44 focused tests, causal guard mutation, 39 process boundary tests and zero-callback headless smoke. Full committed report contains hashes, bounds and prerequisite repairs. Existing backend/session remains running; activation requires starting the backend on the installed new binary. No task_accept tool is exposed in this catalogue.

```json
{
  "op_id": "01M6MJ7H7EQ287NMX2EB9BFP3F",
  "commit": "5b6110d36a6c4a0c0315a70528b86c9d8bffe076",
  "report": "[[reports/2026-11-04-wake-circuit-breaker.md]]",
  "verify_result": {
    "command": "make land",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Self-loop prevention and durable recipient circuit",
        "reference": "crates/bbt-mux/src/messaging/ledger/circuit.rs:52; .bboxy/runs/2026-11-04-wake-circuit-targeted.json"
      },
      {
        "criterion": "Source-bound formal parity and canonical signed landing",
        "reference": ".bboxy/runs/2026-11-04-wake-circuit-landed.json; SHA256 0da4d1d20ec441d27590f95be4520547b89ae4477749d4f72a6551d9df791421"
      },
      {
        "criterion": "Unattended native/process behavior and idle cancellation",
        "reference": ".bboxy/runs/2026-11-04-wake-circuit-idle.json; make smoke idle frames=0 callbacks=0 over 10.005s"
      },
      {
        "criterion": "Approved signed installation and preserved running owners",
        "reference": ".bboxy/runs/2026-11-04-wake-circuit-landed.json; installed BBOXYSourceRevision=5b6110d36a6c4a0c0315a70528b86c9d8bffe076"
      }
    ]
  }
}
```
