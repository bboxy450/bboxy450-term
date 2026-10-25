---
id: 01M5S9V9BBD76YFJTKEXZZ3ANH
type: invariant
number: 319
title: "a cleanup kills only PIDs it recorded as its own children, never by name or pattern; a headless test whose child dies logs the signal and its source"
status: active
check: sh scripts/kill-own-pids.sh
cost: one gate's cleanup, runner or reaper reaches into another gate's processes, or into the operator's, and a test dies of a signal nobody can trace; three concurrent gates become a lottery
links:
  - { to: "[[no-chokepoints]]", rel: builds-on }
  - { to: "[[mux-daemon]]", rel: constrains }
  - { to: "[[e2e-acceptance]]", rel: constrains }
  - { to: "[[e2e-runs-unattended]]", rel: builds-on }
created: "2026-10-25T10:31:36Z"
updated: "2026-10-25T10:31:36Z"
---
# A cleanup kills only its own recorded PIDs

**Property.** No code in the product, its scripts or its tests kills a process
by name or by pattern: every reaper, cleanup, stop path and fencing kill
signals only PIDs it recorded as its own children when it spawned them (the
supervisor's children, the night runner's apps, a gate's own target processes).
When a headless test's child dies of a signal, the test logs the signal, the
dying process and whatever its own records say about the sender, so a death
is traceable from the log alone. `AGENTS.md` already says this for agents;
this makes it product and test law (Oracle, 2026-10-25).

**Check.** `sh scripts/kill-own-pids.sh` — exit 0 held: no `pkill`, `killall`
or `pgrep … kill` in `crates/`, `scripts/` or `lab/web/`. Held today on the
tree as found. The logging half is judgement until a test asserts it: a child
death in a headless test without the signal and sender in the log is a
finding, not a gate.

**Cost of a violation.** One gate's cleanup, runner or reaper reaches into
another gate's processes, or into the operator's, and a test dies of a signal
nobody can trace; three concurrent gates become a lottery.

**Evidence.** Seraph's e2e-unattended gate on 8689f904, 2026-10-25: the test's
own app logic died `signal: 9` mid-test. The trace (Sable) showed the app's own
disconnect fencing kill after the GUI connection dropped, not a foreign reaper,
and the open question moved to who closed the connection first (the
transient-store teardown, being fixed). The rule is written before a foreign
reaper ever does it.

Owner: Corvin Duskmantle <ARCHITECT>.
