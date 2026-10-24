---
id: 01M5R0M77ESZKBEXZG3XA9KPZ6
type: decision
number: 315
title: "identity status and identity unlock are admitted during a custody hold on the ctl door's capability plus the persisted, journal-validated device ID; audited before the native attempt with signature pending; everything else stays held; no new principal"
status: decided
decided: "2026-10-24"
operator: []
links:
  - { to: "[[d280-keychain-regrant]]", rel: amends }
  - { to: "[[d260-ctl-authority]]", rel: builds-on }
  - { to: "[[d285-no-gated-communications]]", rel: builds-on }
  - { to: "[[id1-seat-chain]]", rel: decides }
  - { to: "[[ctl-cli]]", rel: decides }
  - { to: "[[ctl-never-prints-secrets]]", rel: constrains }
created: "2026-10-24T22:31:13Z"
updated: "2026-10-24T23:39:02Z"
---
# Decision 315: the hold's one exit is not behind the hold

Ruled at Kestrel's ask while mapping #280 and #281 onto the native custody path. The loop: a custody hold clears the device ID from public status (`identity/status.rs:114`) and hides it on a hold in local-dev (`local.rs:104`), the device key is unavailable while the hold stands (`bus_signer` returns none), and `ctl` admission requires the device ID before any journaled operation (`logic.rs:3653`). Reused as is, `ctl identity unlock`, the one act #280 names to clear the hold, is refused by the hold it clears.

**Alternatives.** (a) Require the device key to sign the unlock request: impossible during the hold, the loop itself. (b) A new custodian principal or a synthetic seat that is allowed through while held: new authority, a new surface, and a second thing to audit. (c) Relax journaled admission for every operation while held: opens every mutation behind a hold that exists to close them. (d) Kestrel's narrow rule: the **existing operator principal**, established by the `ctl` door's capability (the opt-in endpoint of [[d260-ctl-authority]], same user, ancestry refused) or by the window for the Settings button, plus the **persisted device ID validated from the account journal**, never from a caller field, is sufficient authority for **exactly two verbs**, `identity status` and `identity unlock`; every ordinary mutation and the public held status stay exactly as held.

**Choice.** (d).

**Reason.** The authority already exists in the window: #280 put the Unlock button in Settings › Identity under the operator principal, and `ctl` adds no verb the window lacks ([[ctl-cli]] §1c). The hold is a signing hold, not an identity hold: who the operator is was settled when the device was enrolled and sits in the journal; what the hold removes is the key, and the two verbs that read and end the hold need the identity, not the key. (b) and (c) buy the same exit at the price of new authority; (a) is the bug.

**Rules.** (1) **Admission.** `identity status` and `identity unlock` are admitted under a hold when the caller holds the door's current capability and the journal carries a validated persisted device ID; the admission path reads that ID from the journal, not from public status and not from the request. No other verb gains this path; a held mutation still replies `held` with the hold's name and the two acts that clear it (#280). (2) **Audit before the attempt.** The unlock act is journaled on `account/control` **before** the native Keychain attempt, with `signature: pending` and the hold's reason on the record, the [[d285-no-gated-communications]] shape: signing can be held, the act and its audit cannot. The record's outcome field starts as `unknown` and is set to `allowed`, `cancelled` or stays `unknown` from the native call's own result; an unknown outcome is retained as unknown, never guessed. (3) **Signing when the key returns.** When a legitimate device key is available again the pending record is signed by it and published once, order among pending records preserved; if the key never returns the record stays `signature: pending` and says so. Never a synthetic signature, never another key, never a new principal. **Corrected the same day at Kestrel's ask (exact source 814bc8b9):** the published `account/control` chain is signed and hash-chained with immutable triggers (`store/events.rs:116-125`), so nothing is ever signed "in place" on it. "Journaled before the attempt" means the **durable audit Draft** in the existing ordered outbox (`mcp/journal.rs:254`, account kv, survives restart) exists on disk before the native call, carrying its outbox sequence, creation time, hold reason, `signature: pending` and `outcome: unknown`. The outcome is written into the Draft from the native result. The Draft is **published exactly once, as the same record, by the legitimate device signer only**; the ephemeral bus signer that stands in under a hold (`logic.rs:1695`) must skip Drafts marked for the device signer and never publishes them. **No bus position is reserved**: a hash chain cannot hold a slot, and the property that matters is durability before the attempt and publication order among pending Drafts, both of which the Draft's sequence gives. `identity status` reports the pending audit (count, and the unlock's state) so the hold is never silent about what it still owes. (4) **Public status unchanged.** The held status keeps hiding the device ID from consumers; `identity status` reports the custody tier, the ACL state and the hold's name, never a secret ([[ctl-never-prints-secrets]]). (5) **No hold, no act.** `identity unlock` with no hold active replies `ok` with `hold: none` and does nothing. (6) The one-shot interaction flag, the designated-requirement ACL and the migration during the one access are #280's and unchanged.

**Out of scope.** The file-backed custody path of #281 and the U-0 test accounts of #282 never hold, so none of this runs there; it is the native Keychain path only.

Owner: Corvin Duskmantle <ARCHITECT>.
