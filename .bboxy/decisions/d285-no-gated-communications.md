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
updated: "2026-10-24T19:30:43Z"
---
# Decision 285: no gated communications

Operator, 2026-10-24: "non blocking architecture", "no gated communications". Two gates found the same day, ruled by the Oracle on the spot and recorded here.

**1. Messaging under a custody hold.** Today the release app holds all messaging (`GuiEventsHeld: signing held`) while identity custody is pending, so communication is gated on the Keychain. Ruled: **signing can be held; delivery cannot.** While custody is unavailable, local delivery (same device, same account) continues under the device's honest current state: the envelope carries `signature: pending` and the custody hold's reason, receipts and the native frame show it as **pending**, never as signed, and the journal keeps the unsigned envelopes in order; when custody returns, they are signed in order and receivers upgrade their grade from `pending` to `signed`. Nothing is dropped and nothing is forged: no placeholder signature, ever. What crosses a device boundary waits, because pinned receivers refuse unsigned envelopes ([[id1-seat-chain]] §2b) and that refusal is right. The events lane is never held: the hold becomes a per-row grade, not a lane-wide stop. This is ID1d's "a custody hold does not stop the terminal and does not break the event chain" applied to messaging. Rejected: holding delivery until signing (the gate); signing with a fallback key (a forgery); dropping (never-delete).

**2. A brief over ICQ for a Codex Executor.** Codex Executors treat every peer message as granting no execution, so they block until the operator authorizes in their pane. Ruled: an Oracle's brief authorizes **the in-scope work of the assignment and nothing else**: edits, builds, tests and the records push inside the Executor's own worktree and the brief's `allowed_paths`; never anything outside that worktree, never destructive git, publishing, changes to the user's harness directories, or desktop input. **The authorization is the task record**, signed and numbered in the tree with `allowed_paths` and `verify`, delivered by `Delegate` ([[work-items]]); a chat line is not an authorization, and an Executor checks the task record exists before acting. The mechanism that lets Codex act on it without a pane prompt is the hooks and rules configuration under `~/.codex`, which the Oracle proposes and owns; the Architect's rule is only what it may grant. Rejected: a blanket "peer messages authorize" (laundering); operator presence as the gate (the thing the order forbids).

**Rules.** [[messaging]] and [[l9-messaging-tasks]]: `signature: pending` with the hold reason, local delivery continues, order preserved, signed on return, remote waits. [[work-items]]: the task record is the authorization for Executors. Invariant [[messaging-never-held]], pending its test.

Owner: Corvin Duskmantle <ARCHITECT>.
