---
authorship:
  kind: seat
  process_generation: '1'
  public_key: fewHFqX1/aT03A8Ik46eNy0fXOwtv5Rt1gZmJu1Aids=
  signer: seat:01M649MS05FAD5CY9CS0Q07H76
created: 2026-10-29T18:56:36.379Z
created_by: seat:01M649MS05FAD5CY9CS0Q07H76
id: 01M64GAV0G76GW072S09MPGW9S
links:
- rel: context
  to: '[[vision]]'
number: 427
provenance:
  digest: sha256:31ec07b9e303c2cedb11ffd7f8b52dad2f9acb98e3fef7e2834ef1bafb2dfc47
  frontmatter_digest: sha256:846e79edb7922d434190e7d94a9ae88cfc252747cfa6001f4b3f1cb06647d269
  log: .bboxy/log/2026-10-29.jsonl#1326
  sig: eJOVpcz3U/448gkaQ3sqUKrvuoHoI2xUV+GafdYFlAXpw3ZVcnlEIe+1Ao5hxkgtDOj2KQaJNALMLu5Wm5hwDA==
  signer: seat:01M649MS05FAD5CY9CS0Q07H76
title: 'Journey: deliver work and keep conversations responsive'
type: note
updated: 2026-10-29T18:56:36.379Z
updated_by: seat:01M649MS05FAD5CY9CS0Q07H76
---
<!-- bboxy450-term: https://github.com/bboxy450/bboxy450-term
Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md -->
# Deliver work and keep conversations responsive

Back to [[implementation-journey]]. Signed mentions (#51), typed wake causes (#60), independent Chat/Tasks sections and recent-history restoration are landed. They do not prove every delivery or retained-backlog promise.

| Issue / next task | What remains | Boundary and verification |
|---|---|---|
| [#21 · M1](https://github.com/bboxy450/bboxy450-term/issues/21) | Full exact-destination composer delivery and device-signed receipt evidence | Preserve M1's complete promise and distinguish durable storage, native submission and model ACK. make test-chat-sections is useful component evidence; retained backlog is separately #73. |
| [#22 · M3](https://github.com/bboxy450/bboxy450-term/issues/22) | Real-harness task receipt and visible assigned-task row | M3.sh still says harness stand-in and partial. Prove one oracle-class message per operation, unchanged verify/authority and produced native row. |
| [#23 · M5](https://github.com/bboxy450/bboxy450-term/issues/23) | Real Worker and Oracle ask for unassigned inventory and receive the existing refusal | M5.sh's stand-in pass is not the real-harness promise. Account-wide unassigned inventory stays operator-only. |
| [#24 · M6](https://github.com/bboxy450/bboxy450-term/issues/24) | Automatic priority delays without deletion, preserving groups/mute/one release | Current partial M6 proves retained groups and manual unmute; retain priority/class and actual delay/ACK evidence. |
| [#25 · F1](https://github.com/bboxy450/bboxy450-term/issues/25) | One-swarm stream/channels and arrival within one event-lane delivery, without polling | Canonical F1.sh is absent. Implement through the existing runner; preserve bounded event admission. |
| [#26 · F2](https://github.com/bboxy450/bboxy450-term/issues/26) | Operator read marks/unread survive restart; viewing marks read | F2.sh is absent. A diagnostics read mark and a conversation read mark are different state. |
| [#27 · F3](https://github.com/bboxy450/bboxy450-term/issues/27) | Logic restart resumes cursors without a duplicate or lost row | F3.sh is absent. GUI restart component tests and EOF diagnostics do not substitute for this complete night. |
| [#28 · F4](https://github.com/bboxy450/bboxy450-term/issues/28) | Live filter migration delivers still-granted pending rows, suppresses revoked rows and emits one audit | F4.sh is absent. Preserve old/new grant scopes and actual consumer state. |
| [#73 · retained chat P1](https://github.com/bboxy450/bboxy450-term/issues/73) | Exact direct-seat subjects, fair backlog recovery and bounded unsuccessful wake receipts | Reproduce stale/not-ready retries plus retained history privately; assert a fresh composer row/actual receipt within existing control/frame budget. No deletion, silent read-mark advance or broad wildcard grant. |

Concrete source at reviewed main: `crates/bbt-mux/src/messaging/service.rs:1029` builds operator GUI message subjects without the canonical direct-seat subject; `ledger.rs:1759` publishes direct contexts. Repeated unsuccessful wake outcomes also need bounded/coalesced treatment. These are #73's mechanisms, **not** proof of a 967-seat ingestion repair.

Recent `719e857b` bounds the trusted GUI subject catalogue and `c82a611e` adds a private retained-membership regression. [Issue #73](https://github.com/bboxy450/bboxy450-term/issues/73) explicitly records why that fresh-history proof missed the retained receipt backlog. `cbd89f57` independently restores recent conversation history.

Use [[messaging-seats]], [[logic-gui]], [[d285-no-gated-communications]] and the canonical Make targets `test-chat-sections`, `test-task-contract`, `test-gui-events`, `test-signed-mentions` plus the relevant complete night. The existing targets are starting evidence, not a new FULL verdict.



Admission note, 2026-10-29: this dated triage was first prepared in signed private commits de793c11b7dd3bfd17ac453d8c47c98dc91adbae and 0efe0d917de225fa702667dc646b7981ae7edecc. The authenticated host now assigns this local copy a fresh identity and global number; the private draft numbers are not imported as current authority. Follow [[2026-10-29-zero-issues-cohort]] for subsequent assignments and verified dispositions. Unresolved promises remain unresolved.
