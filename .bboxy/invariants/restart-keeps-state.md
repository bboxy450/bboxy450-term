---
id: 01M5NK2SRDX08STQWHKDBGS9FD
type: invariant
number: 56
title: a restart of the app loses no working state
status: pending
slow: true
gate: pending-rs1
check: cargo test -p bboxy450-term --locked --test recovery restart_keeps_working_state -- --exact 2>&1 | grep -q 'ok. 1 passed; 0 failed'
cost: a restart that loses state makes every rebuild a loss, and fast iteration dies with it; state the operator cannot see cannot be trusted or repaired
links:
  - { to: "[[c1-live-seats]]", rel: constrains }
  - { to: "[[work-items]]", rel: constrains }
  - { to: "[[l9-messaging-tasks]]", rel: constrains }
  - { to: "[[d53-evolve-and-restart]]", rel: decision }
created: 2026-10-23T23:56:02Z
updated: 2026-10-23T23:56:02Z
---
# A restart of the app loses no working state

**Property.** Kill and relaunch the app with live seats, open panes, a scrolled
history, a focused pane, unread marks and in-flight tasks: after resume every
seat is attached to the same PTY, the layout and focus are identical, scroll
positions and read marks hold, and every task shows the same status, because
**no working state exists only in process memory**: seats live in the
supervisor (decision #11), layout and history in the account journal and the
ring ([[c1-live-seats]] §3), read marks in the account object
([[l9-messaging-tasks]] §3), tasks and every record in frontmatter the operator
can open ([[work-items]]). Operator, 2026-10-23: "fast reload without losing
working state, even though native app on GPU rendering, state lives in
FRONTMATTER that user sees".

**Check.** `cargo test -p bboxy450-term --locked --test recovery restart_keeps_working_state -- --exact 2>&1 | grep -q 'ok. 1 passed; 0 failed'`
— exit 0 held. The test does not exist yet: the task that creates it (RS1)
removes `slow` and `gate` from this file so it joins the default set. The
existing recovery and lifecycle tests cover seats and close-and-undo; this one
covers the whole working set in one relaunch.

**Cost of a violation.** Every rebuild becomes a loss, and fast iteration dies
with it; state the operator cannot see cannot be trusted or repaired.

**Evidence.** None yet (pending RS1).

Owner: Corvin Duskmantle <ARCHITECT>.
