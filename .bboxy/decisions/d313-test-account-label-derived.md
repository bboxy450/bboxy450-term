---
id: 01M5QRDW1EF6FBBZ697V6F008Q
type: decision
number: 313
title: "the test-account label is a property of the account root, derived by the receiver and never read from a caller field; seat envelopes under a test account carry issued_by only when it equals that derived label; grant and account wire bytes stay unchanged"
status: decided
decided: "2026-10-24"
operator: []
links:
  - { to: "[[id1-seat-chain]]", rel: decides }
  - { to: "[[l9-messaging-tasks]]", rel: decides }
  - { to: "[[d282-app-tests-itself-unattended]]", rel: builds-on }
  - { to: "[[d269-ctl-door-and-issuer]]", rel: builds-on }
  - { to: "[[d281-custody-facade-and-automation-identity]]", rel: builds-on }
  - { to: "[[e2e-runs-unattended]]", rel: constrains }
created: "2026-10-24T20:07:56Z"
updated: "2026-10-24T20:07:56Z"
---
# Decision 313: where the test-account label lives

Ruled at Kestrel's ask while building the independent file backend of #282 (the file-first implementation approved at 57ebf9ad). The seam: `crates/bbt-mux/src/messaging/wire.rs:125` refuses `door` and `issued_by` on any non-operator envelope, and `DeviceGrant`, `AccountHead` and `Signed` deny unknown fields; yet #282 says everything a test account signs carries `issued_by: test-account:<run>`, and the seats a night spawns are seat principals, not the operator.

**Alternatives.** (a) Kestrel's narrow rule. The generated test-account context is persisted with its own verified root in the run's private account journal; the root certificate UID identifies the test account and the public signed context links the run. The label appears in signed provenance author and context and in the operator `Origin`. For **seat envelopes**, `issued_by` is permitted only under a test account and only when it **exactly equals the label the receiver derives from its own verified view of that root context**, never a value taken from a caller field; `door` stays operator-only. Existing account and grant wire bytes are unchanged. (b) Give the strictly shaped grant DTOs an optional field for the label. (c) No per-envelope label at all: seats keep today's refusal and the frame derives "test account, run X" from the account alone.

**Choice.** (a).

**Reason.** The label is a fact about the signing **account**, so its source of truth is the account root the receiver has already verified; an envelope field can then only agree with it or be refused, which is why (a) adds no authority and no forgery surface. (b) weakens the deny-unknown-fields gate on every real grant, forces a wire bump and a new branch in every verifier, to serve a test-only concern; a grant is the wrong carrier for a property of the root. (c) is byte-for-byte the smallest, but it leaves night-signed seat records without the literal label the spec and the morning review read, and the label is the whole point of the morning review ("never dumb information"). The real account's wire behaviour is unchanged by (a): a non-test account refuses `issued_by` on seat envelopes exactly as today.

**Rules.** (1) The receiver computes the expected label `test-account:<run>` from the verified root certificate UID and the signed run context in its own journal; it never trusts the envelope field as a source. (2) A seat envelope under a test account with `issued_by` absent or different from the derived label is refused with a reason that names the mismatch; under a non-test account the refusal at `wire.rs:125` stands unchanged. (3) `door` remains `gui | ctl` on operator-principal envelopes only. (4) The native frame and the provenance card show the test-account label on every record of the run, so nothing of the night can read as the operator's ([[d269-ctl-door-and-issuer]] attribution rule). (5) The run id in the label is the run directory's id; a label from another run fails the exact match, so a replayed envelope fails too. (6) No field is added to `DeviceGrant`, `AccountHead` or `Signed`; their wire bytes and their deny-unknown-fields posture are untouched.

**Out of scope.** The file store, root generation and restart work of #282 proceed unchanged; no login Keychain and no real operator key is involved anywhere in this path.

Owner: Corvin Duskmantle <ARCHITECT>.
