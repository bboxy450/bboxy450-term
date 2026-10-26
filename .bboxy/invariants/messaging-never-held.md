---
id: 01M5QP9QJZJERD9TMX5T65J287
type: invariant
number: 286
title: "a custody hold never holds local delivery: envelopes travel with an honest pending grade and are signed when custody returns, never dropped or forged"
status: active
check: cargo test -p bboxy450-mux --locked --test messaging_flow custody_hold_never_holds_local_delivery -- --exact 2>&1 | grep -q 'ok. 1 passed; 0 failed'
cost: communication gated on a key is a swarm that stops when a Keychain locks; the operator ran into it on the first real morning
links:
  - { to: "[[d285-no-gated-communications]]", rel: decision }
  - { to: "[[messaging]]", rel: constrains }
  - { to: "[[id1-seat-chain]]", rel: constrains }
  - { to: "[[never-block-swarm]]", rel: sibling }
created: "2026-10-24T19:30:43Z"
updated: "2026-10-25T04:37:31Z"
---
# A custody hold never holds local delivery

**Property.** With identity custody unavailable (the key store held, as on
2026-10-24), an operator DM and a seat-to-seat message on the same device are
delivered to their local recipients with `signature: pending` and the hold's
reason, shown as pending and never as signed; receipts are issued with that
grade; the events lane keeps flowing; when custody returns, the journaled
envelopes are signed in order and the recipients' grade upgrades to signed; no
envelope is dropped and none carries a placeholder signature; an envelope for a
pinned remote receiver waits and says so ([[d285-no-gated-communications]]).
The fixture includes a **first-ever held startup** with no public issuer hint:
the rows are delivered as pending logical intents with no author claim, and on
custody return each gains one appended grade-upgrade event bound to its id and
digest, with the delivered bytes unchanged and no second delivery; the
`operator/inbox` consumer is the same consumer before and after custody return
(one logical operator reference, the device bound to it on enrolment), so no
recipient identity moves and no second consumer appears.

**Check.** `cargo test -p bboxy450-mux --locked --test messaging_flow custody_hold_never_holds_local_delivery -- --exact 2>&1 | grep -q 'ok. 1 passed; 0 failed'`
— exit 0 held. The test covers first-ever held startup through the real admitted
GUI socket and local seat APIs, pending operator and seat delivery, the durable
events lane, recovery on the same operator reference and consumer, immutable
intent and event bytes, bound signed upgrades, FIFO and no second native
delivery. Its remote fixture proves unsigned work remains local until signed.

**Cost of a violation.** Communication gated on a key is a swarm that stops
when a Keychain locks; the operator ran into it on the first real morning.

**Evidence.** NG1 source `d0b2a7b21bf212846389ba6458f414290314c0c1`:
`cargo test -p bboxy450-mux --locked --test messaging_flow -- --nocapture` — exit 0,
3 passed; the real GUI custody recovery fixture completed in 30.30 s.
Raw log SHA256 `ac609df6cd2b65a123024cc6bb47d286340bceadc489386d5ffe0079098b6b6c`;
The exact named check on `56883989ed09055e932c037f58d0f211a76e3fb3` also passed
with the remote guard and all completion/device bindings, SHA256
`f890eda7b142aad6c7a4e63fc8f4bbcb0021eea954248714c159f7d3e9075971`.
Details and earlier failing controls in `.bboxy/runs/ng1-backend-focused.log`.
Counter-evidence before NG1: `GuiEventsHeld: signing held` on the release app,
2026-10-24.

Owner: Corvin Duskmantle <ARCHITECT>.
