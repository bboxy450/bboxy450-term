---
id: "01M5TWW5JBW3JWDQ2HM0KZXVR1"
type: "task"
number: 338
title: "MA-2: receiver-owned attention policy, role defaults and own-seat MCP tools"
status: "open"
role: "executor"
owner: "Velvet Vulpine"
verify: "cargo test -p bbt-proto --locked --test registry_attention --test gui_wire_golden; cargo test -p bbt-mux --locked --test attention_policy --test mcp"
links:
  - { to: "[[messaging-attention]]", rel: spec }
  - { to: "[[notices-never-wake]]", rel: invariant }
created: "2026-10-26T01:23:22Z"
updated: "2026-10-26T01:23:22Z"
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
