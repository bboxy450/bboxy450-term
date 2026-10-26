---
id: "01M5VV6Y755T0T4BDGYQ7ZXHZ7"
type: "task"
number: 342
title: "MA-7: private wake ledger and nightly input-token attribution"
status: "open"
role: "executor"
owner: "Sable Embertail"
verify: "python3 -I -B scripts/test-resource-ledger.py"
links:
  - { to: "[[messaging-attention]]", rel: spec }
  - { to: "[[resource-ledger]]", rel: spec }
  - { to: "[[notices-never-wake]]", rel: invariant }
created: "2026-10-26T10:13:33Z"
updated: "2026-10-26T10:13:33Z"
---
# MA-7 wake accounting

Assigned by Vesper in `cfcf252c-92aa-48d8-9b0f-5c1ad7573cb4`, sequenced after the launcher promises by `6fa7b0f1-90de-4888-ade7-c83dc5ef1548`. MA-3's rebased implementation `ffd8c568` is an ancestor of main; #333's nightly inference reader already landed at `5d1b4c76`.

## Scope

- `crates/bbt-mux/src/messaging/` and the existing native-delivery boundary in `logic.rs`: metadata-only wake records for the admitted attempt, destination/class/mode/message IDs and native binding/outcome.
- `scripts/inference-ledger.py`, existing resource-ledger helpers/tests and the nightly invocation only as required for per-turn input-token fill.
- Private ledger only; no message/tool/thinking content, credentials, capability, provider call, new product wire or board renderer.

## Acceptance

1. One stable wake identity per native attempt, including a batched/digest delivery; record seat, destination, class, policy mode and message IDs. Inbox-only traffic has no wake cost line.
2. Native-written/queued/unknown remain their actual evidence, never a claim that a model accepted a turn. Accounting errors never stop or retry delivery.
3. Nightly token attribution uses actual, unambiguous session/turn or request identities. Missing or conflicting metadata remains unknown, never guessed from nearby timestamps or charged twice.
4. A ledger fixture proves batching, mode/class attribution, deduplication/restart, exact token linkage, unknowns and body exclusion; preserve the notices-never-wake and messaging-never-held checks.

No central collector or new correctness gate. The resource-ledger 90% machine-CPU target is outside this cut and remains unproven.
