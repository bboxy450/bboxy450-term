---
authorship:
  device_id: 01M677XWZPW1XGTQYYGA50W74M
  issued_by: test-account:01M677XWYWXAQQM5DEQP83TQYY
  kind: device
  principal: operator:01M677XWZPW1XGTQYYGA50W74M
  public_key: cHjhL+5pI9VNQE+Uhb4HJBnYoPqSIH7aCYU+b9MElWs=
  signer: device:01M677XWZPW1XGTQYYGA50W74M
created: 2026-10-25T23:48:19Z
created_by: device:01M677XWZPW1XGTQYYGA50W74M
id: 01M5TQE3KZA5A5FCQDMJ3PXH97
issued_by: test-account:01M677XWYWXAQQM5DEQP83TQYY
links:
- rel: spec
  to: '[[messaging-attention]]'
- rel: invariant
  to: '[[notices-never-wake]]'
number: 337
owner: Kestrel Voidwing
provenance:
  digest: sha256:c18dbbb33b261d499b752d1e2dd22d9d2f22a19e8a0c5e9a3a31487c476215d1
  frontmatter_digest: sha256:7d022eb249256a70893f86195269ee768c998f023104263afe9977c1bb095da7
  log: .bboxy/log/2026-10-30.jsonl#716
  sig: AJE8noYrEiyKAhgSM7D1b7OvGDJdmaydRRdtWCfsObN1FpLVGPlasMZNX6Z2G6G3rsTdTttIItsRnX4v+fNxDw==
  signer: device:01M677XWZPW1XGTQYYGA50W74M
reconciliation:
- by: operator:01M677XWZPW1XGTQYYGA50W74M
  disposition: implemented
  evidence:
  - https://github.com/bboxy450/bboxy450-term/commit/32fe48ebbb2c77b152d71d9996a384fafbaba71e
  fresh_accepted: false
  kind: historical
  op_id: 01M677Y3BMBGVNEXT7SY59CBHR
  previous_status: open
  reason: Operator-authorized historical msgrec reconciliation, 2026-10-30; implementation already landed on main; no new acceptance claimed.
role: executor
status: done
title: 'MA-1: additive message classes, structured events and receipt wake wire'
type: task
updated: 2026-10-30T20:27:33.739Z
updated_by: device:01M677XWZPW1XGTQYYGA50W74M
verify: cargo test -p bbt-proto --locked --test messaging --test gui_wire_golden
---
# MA-1 messaging wire contract

Assigned by Vesper in brief `99edc18d-9d85-47d3-992d-ad0a6b97e213`, after the native Keychain fixture landed at `b8964515`. Shape pinned by Corvin at `e64096e6`, messaging-attention §2. Scope extension `21042dd8-bd1b-4136-804b-f4d15ad208a0` admits exactly three mechanical downstream ports.

## Scope

- `crates/bbt-proto/src/messaging.rs`: MessageClass derived from the signed MessageKind, six new structured event kinds and bodies exactly as the spec table, WakeMode, and optional receipt wake data omitted on old rows.
- `crates/bbt-proto/tests/messaging.rs` and `tests/golden/gui_wire.tsv`: new class/event/wake contract cases; every pre-existing golden byte remains unchanged.
- `crates/bbt-mux/src/messaging/wire.rs`: named refusal of new event inputs until their service cut; no event admission.
- `crates/bbt-mux/src/messaging/ledger.rs`: existing receipt constructor initializes absent wake data.
- `crates/bboxy450-term/src/messaging.rs`: kind-label projection handles the new variants.

## Acceptance

1. Class is derived from kind, with no sender-supplied class. Presence is event; existing Task remains conversation; the new TaskTransition is event.
2. SlotClaimed, SlotReleased, CutClaimed, Landed, GateResult, and TaskTransition carry exactly the spec's typed fields. Receipt shapes are wake now, none, or deferred with RFC 3339 until; old receipts omit both fields.
3. Existing golden lines are byte-identical; appended cases pin every event and wake state.
4. The current host refuses the new event inputs by a named reason. No new producer, policy store, digest, or native delivery behaviour is introduced.
5. Land this contract alone through land.sh, with its affected checks, strict Clippy and selected invariants. The postsubmit runs the full workspace and parity.

MA-2 to MA-7 follow this landing. MA-9 remains a later separate task after MA-3 and MA-4 land; no e2e promise is added in this cut.
