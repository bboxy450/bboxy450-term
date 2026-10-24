---
id: 01M5PRA3C48H0TFHT0KDMPDZDS
type: decision
number: 267
title: "swarms per device are unbounded by design, so an idle swarm costs the message pump nothing; the pump scans only swarms with pending work"
status: decided
decided: "2026-10-24"
operator: []
links:
  - { to: "[[messaging]]", rel: decides }
  - { to: "[[mux-daemon]]", rel: decides }
  - { to: "[[idle-swarms-cost-nothing]]", rel: constrains }
  - { to: "[[d23-recipient-batching]]", rel: builds-on }
  - { to: "[[vision]]", rel: builds-on }
created: "2026-10-24T10:46:38Z"
updated: "2026-10-24T10:46:38Z"
---
# Decision 267: an idle swarm costs nothing

Ruled by the Architect on 2026-10-24 from Rook's R5g diagnosis: the message pump (the logic maintenance phase `MaintenanceMessages`) cost 1.3, 96 and 133 ms per call at 4, 64 and 65 swarms, about 11 to 14 s of a 12 to 15 s maintenance budget (raw SHAs 9f9b1fdc, 29d67438, 60b0dc0b); the suspected source is a ready and events scan over every ledger with a 64-connection pool, which the step from 64 to 65 points at.

**Is the swarm count bounded by design?** No. [[vision]] sizes a swarm at a thousand seats and puts several swarms on a device; the knowledge base mounts several roots; the operator's own machine runs the cockpit swarm beside fixtures and probes. A bound on swarms per device would be a bound invented to fit a scan, and the measured shape, cost linear in swarms and a cliff at the pool size, is a scan over things that have nothing to say.

**Alternatives.** (a) Bound swarms per device and document it. (b) Make the pump's per-call cost independent of the number of **idle** swarms: the pump visits only swarms with pending work, known from a dirty set the ledgers mark on write (the shape `bbt-rt`'s `DirtySet` already gives the UI), and holds no per-swarm connection open for a swarm that is idle. (c) Raise the pool size.

**Choice.** (b).

**Reason.** (a) trades the product's shape for an implementation detail; (c) moves the cliff. (b) is the same principle that made the thousand-seat bus pass ([[d23-recipient-batching]]): do work per thing that happened, never per thing that exists. Invariant [[idle-swarms-cost-nothing]].

**Rules.** The pump keeps a dirty set of swarms with pending ready or event work, marked on write and cleared on drain; a pump call touches exactly those swarms and no ledger of an idle one; connections to idle swarms' ledgers are not held open against a fixed pool. The check is deterministic, a count of ledgers scanned per pump call, and the timing stays in the R5 diagnostics as evidence. Rook fixes it after R5g (Oracle's order).

Owner: Corvin Duskmantle <ARCHITECT>.
