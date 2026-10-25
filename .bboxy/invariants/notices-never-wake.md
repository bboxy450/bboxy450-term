---
id: 01M5TGD48SYT55GN92MDM8HMAK
type: invariant
number: 335
title: "notices never wake: under default policy an event-class message is stored and receipted for every recipient and causes no native delivery to a model seat; a conversation message to a seat whose policy says wake does"
status: pending
slow: true
gate: pending-messaging-attention
check: cargo test -p bbt-mux --locked --test messaging_flow events_are_stored_and_never_wake_by_default -- --exact 2>&1 | grep -q 'ok. 1 passed; 0 failed'
cost: every status notice becomes a model turn and a full context re-read for every seat in the room; on 2026-10-24 and 25 that was about a hundred empty turns in one pane alone
links:
  - { to: "[[messaging-attention]]", rel: spec }
  - { to: "[[messaging-never-held]]", rel: builds-on }
  - { to: "[[resource-ledger]]", rel: builds-on }
created: "2026-10-25T21:45:27Z"
updated: "2026-10-25T21:45:27Z"
---
# Notices never wake

**Property.** With every seat on its role-class default policy
([[messaging-attention]] §3), an event-class message posted to a room is
stored for and receipted to every recipient (`recipient-stored`, `wake: none`)
and causes **no** native delivery to any model seat; a conversation message
to a seat whose policy for that destination is `wake` is delivered natively
once; and the always-wake exceptions (operator, delegation, own thread) wake
regardless of policy. [[messaging-never-held]] holds throughout: nothing is
held, only the wake is withheld.

**Check.** The named test, exit 0 held. **Pending** until MA-3 lands it; the
flags keep it out of the default run until then.

**Cost of a violation.** Every status notice becomes a model turn and a full
context re-read for every seat in the room.

**Evidence.** The Architect's pane, about a hundred empty turns answering
channel broadcasts over 2026-10-24 and 25, before turning the channel's wake
off; Codex's 10.5 billion input tokens for 37.6 million output.

Owner: Corvin Duskmantle <ARCHITECT>.
