---
id: 01M5QP4ZKPM5FSMTDS0Z6Q4875
type: invariant
number: 284
title: "never block the swarm: no open task is parked on a person, and no test needs the operator"
status: active
check: sh scripts/never-block.sh
cost: a parked task is a seat idle and an operator paged; the swarm exists so that neither happens
links:
  - { to: "[[d282-app-tests-itself-unattended]]", rel: decision }
  - { to: "[[e2e-runs-unattended]]", rel: sibling }
  - { to: "[[work-items]]", rel: constrains }
  - { to: "[[mantras]]", rel: mantra }
created: "2026-10-24T19:28:08Z"
updated: "2026-10-24T19:28:08Z"
---
# Never block the swarm

Operator, 2026-10-24, verbatim: "test and e2e tests and smoke tests should not require MY INPUT, fully autonomous! NEVER BLOCK SWARM". Two halves, one checkable now.

**Property.** (a) No open task or brief is parked on a person: no task record
whose status is not closed says it waits on, is blocked on, needs or requires
the operator, the Architect, the Oracle or a ruling. The Oracle rules on the
spot and the Architect records after; blocked work is reassigned, never parked.
(b) No test needs the operator: smoke, e2e and every CI tier run with stdin
closed, no window and no login-Keychain backend call; that half is the gate
[[e2e-runs-unattended]] (#283), pending the runner, and is referenced, not
duplicated, here.

**Check.** `sh scripts/never-block.sh` — a grep over `.bboxy/tasks/*.md` (the
records, not the derived index) for a non-closed task that parks work on a
person; exit 0 held, non-zero names the task and the phrase. Fast, in the
default set, run by every records push. Its first run flagged the task index's
PKG1 row, "DEFERRED … Was: BLOCKED on operator", historical text in a derived
index; the records themselves are clean, which is why the scan reads them.

**Cost of a violation.** A parked task is a seat idle and an operator paged;
the swarm exists so that neither happens.

**Evidence.** `sh scripts/never-block.sh` — `never-block: held` at the commit
that adds it.

Owner: Corvin Duskmantle <ARCHITECT>.
