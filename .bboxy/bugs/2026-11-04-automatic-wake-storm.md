---
type: bug
title: Self-authored task notices and automatic wake storms consume inference
status: open
owner: seat:01M6MADHFD19P4TKM7P7X6QESA
id: 01M6ME2N3FY6RTM7M91GW9HWA6
delegated_by: seat:01M6MADHFD19P4TKM7P7X6QESA
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-04T23:36:12.764Z
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
provenance:
  digest: sha256:7f325bef0e32f397f8499bd399610ffb8dcd6f74002f6a20026525b5b41cb424
  frontmatter_digest: sha256:b8a4ec4167060e7c8b4a97f4f4ab8e2a2c313a6b72ce16d6418f6f158f5b29c2
  signer: seat:01M6MADHFD19P4TKM7P7X6QESA
  sig: egK9b3eQMbtxyQMqo9DRVMSeKle/9bMNoCrfBAm5SuKNakmLNLHHShmBzfbq1otKbjFneoIlhE4dpCSMIa4NBw==
  log: .bboxy/log/2026-11-04.jsonl#130933
---
Direct operator implementation request to Sable Quill on 2026-11-04; current scope and circuit contract are recorded in [[notes/2026-11-04-wake-circuit-breaker]]. This finding is owned and implemented by this solo seat; no delegated assignment or additional seat is created.

Observed: Vesper's task delegated_by and assignee were the same seat. Native task operations generated 24 self notifications (one delegation, 22 amendments, one report), which became 24 redundant turns after the long work turn. Counts and token deltas were matched against the actual message ledger and Codex transcript; no raw transcripts are committed.

Required outcome: suppress self-authored automatic wakes and implement a recipient-local durable automatic-wake circuit with retained inbox/audit data, bounded admissions, no reset through agent ACK/replay/restart, explicit operator rearm, diagnostics and finite model/API negative controls. Verify through make test-wake-circuit-breaker, mandatory native/Java formal verification, source license checks and the canonical signed landing gate. Keep backend execution and ongoing sessions intact; this request does not authorize terminating the operator's running backend.