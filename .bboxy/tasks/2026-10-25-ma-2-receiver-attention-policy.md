---
authorship:
  device_id: 01M677XWZPW1XGTQYYGA50W74M
  issued_by: test-account:01M677XWYWXAQQM5DEQP83TQYY
  kind: device
  principal: operator:01M677XWZPW1XGTQYYGA50W74M
  public_key: cHjhL+5pI9VNQE+Uhb4HJBnYoPqSIH7aCYU+b9MElWs=
  signer: device:01M677XWZPW1XGTQYYGA50W74M
created: 2026-10-26T01:23:22Z
created_by: device:01M677XWZPW1XGTQYYGA50W74M
id: 01M5TWW5JBW3JWDQ2HM0KZXVR1
issued_by: test-account:01M677XWYWXAQQM5DEQP83TQYY
links:
- rel: spec
  to: '[[messaging-attention]]'
- rel: invariant
  to: '[[notices-never-wake]]'
number: 338
owner: Velvet Vulpine
provenance:
  digest: sha256:1bb8ad4ed81514fae9cde621fcc0d331b1df0d6c60f02d3188c00b6ddf043100
  frontmatter_digest: sha256:c2543223aa66e91c8a965435c40ede66e6eb3a03cf97f0695a8f7951300eef00
  log: .bboxy/log/2026-10-30.jsonl#725
  sig: sZQCSRLTRWUO4IHXTBDyityu2zzVuV2g4SEa6UgjLOflF11NAe4K9Uj99aljs/VMUxl037ztH68cDCDBL7d2Dw==
  signer: device:01M677XWZPW1XGTQYYGA50W74M
reconciliation:
- by: operator:01M677XWZPW1XGTQYYGA50W74M
  disposition: implemented
  evidence:
  - https://github.com/bboxy450/bboxy450-term/commit/de98476dfaa50d0773fb6f2dda72a507a778ac94
  fresh_accepted: false
  kind: historical
  op_id: 01M677Y7ZQ3Q0AXG6WQK4AV7XD
  previous_status: open
  reason: Operator-authorized historical msgrec reconciliation, 2026-10-30; implementation already landed on main; no new acceptance claimed.
role: executor
status: done
title: 'MA-2: receiver-owned attention policy, role defaults and own-seat MCP tools'
type: task
updated: 2026-10-30T20:27:38.487Z
updated_by: device:01M677XWZPW1XGTQYYGA50W74M
verify: cargo test -p bbt-proto --locked --test registry_attention --test gui_wire_golden; cargo test -p bbt-mux --locked --test attention_policy --test mcp
---
# MA-2 receiver attention policy

Assigned by Vesper in brief `82414f4f`, reconfirmed by `92b934cc` and `b58ea50a`, after M2. Scope extension `74b31640-59ca-4cbc-b974-d89297871f7a` admits the additive typed SeatRecord field; Corvin recorded the same class/name and conversation-only rules at `9dbe7be3`.

## Scope

- `crates/bbt-proto/src/registry.rs`, `tests/registry_attention.rs`, and the existing GUI golden: an optional typed policy, omitted on legacy records; land this contract first.
- `crates/bbt-mux/src/messaging/attention.rs`, `src/registry.rs`, `src/mcp/attention.rs`, their module declarations, the tool catalogue and logic control dispatch: bounded own-seat policy storage and a pure decision port.
- `crates/bbt-mux/tests/attention_policy.rs` and `tests/mcp.rs`: role defaults, modes, exceptions, persistence, schema and authority/replay fences.

## Acceptance

1. Policy maps destination and MessageClass to WakeMode on the durable seat record. An omitted field selects defaults and every old wire byte remains unchanged.
2. Class::Oracle selects Oracle defaults; exact architect/creativity role names select theirs; all other seats select Executor defaults. Every default Event is inbox.
3. Only conversation from the operator principal, delegated to the receiver, or replying in a thread it started always wakes. Explicit event policies are the receiver's choice. Mute is unchanged.
4. attention_get and attention_set act only on the admitted seat; a seat, including an Oracle-class seat, cannot edit another receiver. Mutations use the existing operation journal and survive restart and unrelated registry updates.
5. Native delivery, receipt wake reporting, digests and reply validation remain MA-3, MA-4 and MA-5. No renderer, new identity or second registry store.

Focused implementation tests precede handoff; the mandatory presubmit runs once through the existing landing path. No operator interaction.
