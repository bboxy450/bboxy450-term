---
authorship:
  device_id: 01M677XWZPW1XGTQYYGA50W74M
  issued_by: test-account:01M677XWYWXAQQM5DEQP83TQYY
  kind: device
  principal: operator:01M677XWZPW1XGTQYYGA50W74M
  public_key: cHjhL+5pI9VNQE+Uhb4HJBnYoPqSIH7aCYU+b9MElWs=
  signer: device:01M677XWZPW1XGTQYYGA50W74M
created: 2026-10-26T03:03:17Z
created_by: device:01M677XWZPW1XGTQYYGA50W74M
id: 01M5V2K3452HJ3HFFQPTV27FBN
issued_by: test-account:01M677XWYWXAQQM5DEQP83TQYY
links:
- rel: spec
  to: '[[messaging-attention]]'
- rel: invariant
  to: '[[notices-never-wake]]'
- rel: invariant
  to: '[[messaging-never-held]]'
number: 339
owner: Velvet Vulpine
provenance:
  digest: sha256:0b5455af1d7509f498159ef6e0bd676e80d3c5ac65e297616d8aace544a0b910
  frontmatter_digest: sha256:ace2bf729e68de9bf1b80b2ff3781b66fa9593c8b0bd17370c8c0eb9ebfcab5e
  log: .bboxy/log/2026-10-30.jsonl#734
  sig: mgFVEOQFlcVQUySndSi3m6itL+Gx3yH70Cv8sIJZwkOOaYXI2B9hu/v9L6vrv4YZ4gzwoB6gagzaqghfvQ9bCA==
  signer: device:01M677XWZPW1XGTQYYGA50W74M
reconciliation:
- by: operator:01M677XWZPW1XGTQYYGA50W74M
  disposition: implemented
  evidence:
  - https://github.com/bboxy450/bboxy450-term/commit/efe90bedb6cad68d60dbc6937bd78d38fe44cb9f
  fresh_accepted: false
  kind: historical
  op_id: 01M677Y9673MHN2R77G9AYZ65K
  previous_status: open
  reason: Operator-authorized historical msgrec reconciliation, 2026-10-30; implementation already landed on main; no new acceptance claimed.
role: executor
status: done
title: 'MA-3: native delivery follows receiver attention without holding inbox traffic'
type: task
updated: 2026-10-30T20:27:39.712Z
updated_by: device:01M677XWZPW1XGTQYYGA50W74M
verify: cargo test -p bbt-mux --locked --test messaging_flow
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
