---
id: 01M5PRA3C4FA03CFD5P99HJZD1
type: invariant
number: 268
title: "the message pump's cost is independent of the number of idle swarms"
status: pending
slow: true
gate: pending-r5i
check: cargo test -p bbt-mux --locked --test logic pump_scans_only_dirty_swarms -- --exact 2>&1 | grep -q 'ok. 1 passed; 0 failed'
cost: a pump that scans every swarm makes the cockpit slower for every swarm the operator keeps, idle or not, and the maintenance budget was 90 percent pump at 64 swarms
links:
  - { to: "[[d267-idle-swarms-cost-nothing]]", rel: decision }
  - { to: "[[messaging]]", rel: constrains }
  - { to: "[[mux-daemon]]", rel: constrains }
created: "2026-10-24T10:46:38Z"
updated: "2026-10-24T10:46:38Z"
---
# The message pump's cost is independent of the number of idle swarms

**Property.** With one active swarm and N idle swarms (N at least 64, past the
connection pool size), one pump call scans exactly the ledgers of swarms with
pending work, which is one, and opens no connection to an idle swarm's ledger;
the count of ledgers scanned per call equals the size of the dirty set, for
every N ([[d267-idle-swarms-cost-nothing]]). The timing that motivated this
(1.3 ms at 4 swarms, 96 ms at 64, 133 ms at 65, Rook's R5g runs) is evidence in
the R5 diagnostics; the gate is the count, because a count does not depend on
the machine.

**Check.** `cargo test -p bbt-mux --locked --test logic pump_scans_only_dirty_swarms -- --exact 2>&1 | grep -q 'ok. 1 passed; 0 failed'`
— exit 0 held. The test does not exist yet; the fix after R5g (R5i) creates it
and its commit removes `slow` and `gate` from this file.

**Cost of a violation.** The cockpit gets slower for every swarm the operator
keeps, idle or not; at 64 swarms the pump was about 90 % of the maintenance
budget and the catch-up stall of window 4 had a second cause.

**Evidence.** Rook's R5g runs, raw SHAs 9f9b1fdc, 29d67438, 60b0dc0b (2026-10-24).

Owner: Corvin Duskmantle <ARCHITECT>.
