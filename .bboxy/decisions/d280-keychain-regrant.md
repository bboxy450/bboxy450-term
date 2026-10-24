---
id: 01M5QNA6KXW1AW8GCJ1KHG5MZ1
type: decision
number: 280
title: "Keychain re-grant: one interactive access on an explicit operator act (ctl identity unlock or Settings › Identity › Unlock), then the no-dialog default returns; new items carry an ACL keyed to the team-signed designated requirement so rebuilds keep access"
status: decided
decided: "2026-10-24"
operator: []
links:
  - { to: "[[id1-seat-chain]]", rel: decides }
  - { to: "[[ctl-cli]]", rel: decides }
  - { to: "[[d17-device-key-custody]]", rel: amends }
  - { to: "[[d24-defer-code-signing]]", rel: builds-on }
  - { to: "[[mantras]]", rel: mantra }
created: "2026-10-24T19:13:30Z"
updated: "2026-10-24T23:39:02Z"
---
# Decision 280: Keychain re-grant on an explicit act

Ruled by the Architect on 2026-10-24 from Vesper's run of the release app on the operator's real identity: the login-Keychain item `bboxy450-term`, created on 2026-10-23 by an earlier build, refuses every new build; `identity/keychain.rs` disables user interaction on purpose, so the app sits at "Pending: Keychain interaction required; signing held" forever, no prompt ever appears, and the operator has to edit the item's Access Control by hand. `set-generic-password-partition-list` alone did not clear it.

**Alternatives.** (a) Allow Keychain interaction by default (a dialog at any moment, the thing the no-silent-prompt principle forbids). (b) An explicit operator-only act, `ctl identity unlock` or the Unlock button in Settings › Identity (one action, two doors), permits **exactly one** interactive Keychain access for this app: the next access runs with interaction enabled, macOS shows its own dialog, and whether the operator allows or cancels, the no-dialog default is restored; the act and its outcome are journaled on `account/control`. New items are created with an ACL keyed to the **team-signed designated requirement** (the approved Apple Development identity, team UQB3368A84, and the app identifier), so every rebuild signed with that identity keeps access without a prompt; a legacy item that refuses the designated requirement is migrated during the one interactive access: the secret is read, a new item with the keyed ACL is written and verified, and the old item is removed only after the new one answers. The hold message says exactly how to clear it. **Migration mechanics (same day, at Kestrel's ask):** a generic-password item is unique per service and account, so the new item cannot coexist under the original label; the migration stages it under the **fixed internal label `<label>-regrant-v1`** for the same device, deterministic, versioned, never caller-controlled. Order: write the staged item with the keyed ACL; verify it by a **non-interactive** read that returns the same secret (that read is the proof the ACL works for this build); delete the old item; create the original label from the staged copy and verify it the same way; delete the staged copy. Loads try the original label, then the staged one, and a run that finds a staged item resumes the migration from where it stopped. Every failure or crash leaves at least one verified copy readable without a prompt once the staged write has been verified, and before that point the old item is untouched. Each step's outcome is journaled under the same unlock audit. (c) Delete and re-create the item silently on refusal (loses the custody chain; and a silent delete of a credential is the kind of thing the operator never authorized).

**Choice.** (b).

**Reason.** The hold is correct, fail closed, and useless if it says only that it is held: a refusal that does not say what clears it is dumb information ([[mantras]] 20). Interaction stays bound to an operator act, so nothing ever prompts on its own, and the designated-requirement ACL removes the cause: the item was keyed to one build's signature instead of to the identity every build shares. (a) breaks the principle; (c) breaks the chain.

**Rules.** [[id1-seat-chain]] §4: the hold state names the two acts; the one-shot interaction flag; the ACL rule for new items and the migration during the one access; the journal line. [[ctl-cli]]: noun `identity` with `status` (the custody tier, the item's ACL state, whether a hold is active) and `unlock` (the one-shot act), both under the operator principal, `door: ctl`, audited like every verb. Dev binaries are signed with the same identity by `scripts/sign-run.sh`, so the ACL covers them. The e2e promise I1 reads against it.

Owner: Corvin Duskmantle <ARCHITECT>.
