---
id: "01M5V2K3452HJ3HFFQPTV27FBN"
type: "task"
number: 339
title: "MA-3: native delivery follows receiver attention without holding inbox traffic"
status: "open"
role: "executor"
owner: "Velvet Vulpine"
verify: "cargo test -p bbt-mux --locked --test messaging_flow"
links:
  - { to: "[[messaging-attention]]", rel: spec }
  - { to: "[[notices-never-wake]]", rel: invariant }
  - { to: "[[messaging-never-held]]", rel: invariant }
created: "2026-10-26T03:03:17Z"
updated: "2026-10-26T03:03:17Z"
---
# MA-3 native attention

Assigned by Vesper in `92b934cc-2297-4abb-ba19-4dec74ec5fbd`, MA-2 through MA-5 after MA-1. MA-2's pure port is preserved. Corvin pins admission-time decisions at `cd3c6c53`; only subsequent admissions see attention_set, while mute remains live. Digest time uses the service's injected time port, per `ceae50c7`.

## Scope

- `crates/bbt-mux/src/messaging/service.rs`: derive trusted principal/delegation/thread facts and consult the receiver policy before native scheduling.
- `crates/bbt-mux/src/messaging/ledger.rs`: persist receipt wake state and keep wake:none inbox rows outside the indexed native retry scan; preserve FIFO, authority, mute and crash fencing.
- `crates/bbt-mux/tests/messaging_flow.rs`: the named notices-never-wake test, admission pinning, and the protected custody-hold invariant.
- Existing messaging/native-boundary fixtures in `crates/bbt-mux/tests/` and service cfg(test) only as required to explicitly request Wake for their unchanged native contract.

## Acceptance

1. Events under every role default are durable recipient-stored inbox rows with wake:none and no native work, not fake holds. A conversation whose receiver policy says Wake is prepared once.
2. Conversation-only operator, delegated-task and own-thread exceptions remain forced Wake. A sender never supplies receiver policy or authority facts.
3. Receipt decisions are pinned at recipient admission. Policy edits never wake archived Inbox rows; a pending digest retains its deadline and admission policy. Actual mute, pause and grant fences remain live.
4. The indexed native scan grows with pending native work, not inbox history. Legacy receipts are classified without replaying completed native attempts.
5. The existing messaging-never-held check stays green: pending local custody grades, immutable intents, ordered signature upgrades and no duplicate native delivery.

MA-4 owns digest window/cap aggregation and the ID summary; MA-5 owns typed reply refusal/hints. No renderer, app clock bridge or second delivery path. Rook confirmed production service/ledger and messaging_flow are clear in `a61f13ee`; preserve his cfg(test) keeps comments on union. One own O1 target, two jobs, signed evidence, no operator input.
