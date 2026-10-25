---
id: 01M5QP9QJZ5XXP9CHWNFX98KKM
type: decision
number: 285
title: "no gated communications: local delivery continues under a custody hold with an honest pending grade and is signed when custody returns, never dropped or forged; an Oracle's brief authorizes a Codex Executor's in-scope work in its own worktree, carried as a task record, never a chat line"
status: decided
decided: "2026-10-24"
operator:
  - "non blocking architecture"
  - "no gated communications"
links:
  - { to: "[[messaging]]", rel: decides }
  - { to: "[[l9-messaging-tasks]]", rel: decides }
  - { to: "[[id1-seat-chain]]", rel: builds-on }
  - { to: "[[work-items]]", rel: decides }
  - { to: "[[never-block-swarm]]", rel: builds-on }
  - { to: "[[messaging-never-held]]", rel: constrains }
  - { to: "[[d282-app-tests-itself-unattended]]", rel: builds-on }
created: "2026-10-24T19:30:43Z"
updated: "2026-10-25T01:31:30Z"
---
# Decision 285: no gated communications

Operator, 2026-10-24: "non blocking architecture", "no gated communications". Two gates found the same day, ruled by the Oracle on the spot and recorded here.

**1. Messaging under a custody hold.** Today the release app holds all messaging (`GuiEventsHeld: signing held`) while identity custody is pending, so communication is gated on the Keychain. Ruled: **signing can be held; delivery cannot.** While custody is unavailable, local delivery (same device, same account) continues under the device's honest current state: the envelope carries `signature: pending` and the custody hold's reason, receipts and the native frame show it as **pending**, never as signed, and the journal keeps the unsigned envelopes in order; when custody returns, they are signed in order and receivers upgrade their grade from `pending` to `signed`. Nothing is dropped and nothing is forged: no placeholder signature, ever. What crosses a device boundary waits, because pinned receivers refuse unsigned envelopes ([[id1-seat-chain]] §2b) and that refusal is right. **First-ever held startup (Rook's NG1 preflight, same day).** On the first held startup there may be no public issuer hint at all (`identity/custodied.rs:63-79`) and no leaf to derive (`:117-132`), while the ledger admits only a verified signed envelope (`messaging/ledger.rs:207`); `signature: pending` cannot then name an author. Ruled: local pending delivery covers this case too, and the pending representation is an **explicitly pending logical intent**: the logical sender as known (device reference if persisted, seat, generation), the request, the hold reason and the admission order, with **no cryptographic author claim** (`author` absent, never a placeholder key), shown as pending with the reason and read at the inferred grade. When custody returns, the signed envelope is **materialised from the intent and appended as a grade-upgrade event bound to the intent's id and digest**, the delivered bus bytes preserved and the recipient not delivered a second time: the frame's grade flips, the message does not move. An ephemeral signer standing in during a later hold is observation, never authority. Pinned remote verification stays strict: a pending intent never leaves the device. This is the same shape as #315's realisation (pending now, signed completion appended later, bound by id and digest, never rewritten), one pattern for both. The events lane is never held: the hold becomes a per-row grade, not a lane-wide stop. This is ID1d's "a custody hold does not stop the terminal and does not break the event chain" applied to messaging. Rejected: holding delivery until signing (the gate); signing with a fallback key (a forgery); dropping (never-delete).

**2. A brief over ICQ for a Codex Executor.** Codex Executors treat every peer message as granting no execution, so they block until the operator authorizes in their pane. Ruled: an Oracle's brief authorizes **the in-scope work of the assignment and nothing else**: edits, builds, tests and the records push inside the Executor's own worktree and the brief's `allowed_paths`; never anything outside that worktree, never destructive git, publishing, changes to the user's harness directories, or desktop input. **What carries the authorization depends on where the Executor runs (Oracle's correction, same day).** For **in-app seats**, Claude or Codex inside the cockpit, the authorization is the task record, signed and numbered in the tree with `allowed_paths` and `verify`, delivered by `Delegate` ([[work-items]]); a chat line authorizes nothing and the seat checks the record exists before acting. For **this swarm's own Executors outside the app**, briefed over ICQ and not through app records, an **authenticated ICQ brief from the Oracle key listed in `.bboxy/graph.md`** is the authorization for in-scope work in the Executor's own worktree, with the same refusals (outside the worktree, destructive git, publishing, merging, spending, the user's harness directories, desktop input, any other key); that is now the rule in `~/.agents/AGENTS.md`, with the operator's "dont ask for permissions ask for forgivness". The mechanism that lets Codex act without a pane prompt is the hooks and rules configuration under `~/.codex`, which the Oracle proposes and owns; the Architect's rule is only what either form may grant. Rejected: a blanket "peer messages authorize" (laundering); operator presence as the gate (the thing the order forbids).

**Rules.** [[messaging]] and [[l9-messaging-tasks]]: `signature: pending` with the hold reason, local delivery continues, order preserved, signed on return, remote waits. [[work-items]]: the task record is the authorization for Executors. Invariant [[messaging-never-held]], pending its test.

Owner: Corvin Duskmantle <ARCHITECT>.
