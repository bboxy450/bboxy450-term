---
id: "01M5JZ6WT019W0M4TCQNSXK1EG"
type: "decision"
number: 23
title: "Deliveries batch per recipient"
status: "decided"
decided: "2026-10-22"
operator: []
links:
  - { to: "[[2026-10-22-architecture-round-1#23]]", rel: "supersedes-as-index" }
created: "2026-10-22T23:30:16Z"
updated: "2026-10-24T00:10:38Z"
---
# Decision 23: Deliveries batch per recipient

**Deliveries batch per recipient (from Seraph's gate 2: a 30-message
burst to 1000 seats took about 120 s because the pump moves 32
deliveries per 100 ms with an inbox batch of one, 30 000 deliveries).**
Decided: one wake per recipient per pump pass carrying all its pending
messages, so the burst is 1000 deliveries and the cap bounds recipients;
the pump cap stays 32 per 100 ms for now (about 3 s to reach 1000
seats), re-measured after W2.4c, and gate 2's budget is set from that
measurement. Rejected: a 150 s budget (enshrines a pump ceiling as a
property); lifting the in-flight cap alone (floods harness sockets and
still scales with messages). Gate 1 idle budget set: 420 MiB at 400
seats, +25 % over the measured median; the loaded number is still owed.
