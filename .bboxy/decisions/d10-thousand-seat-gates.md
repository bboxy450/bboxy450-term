---
id: "01M5JFXASG79RZCJTFPX6RGZRT"
type: "decision"
number: 10
title: "1000 seats: four measured gates before C1 ships"
status: "decided"
decided: "2026-10-22"
operator: []
links:
  - { to: "[[2026-10-22-architecture-round-1#10]]", rel: "supersedes-as-index" }
  - { to: "[[runtime]]", rel: "decides" }
created: "2026-10-22T19:02:54Z"
updated: "2026-10-24T00:10:38Z"
---
# Decision 10: 1000 seats: four measured gates before C1 ships

**1000 seats: four measured gates before C1 ships.** Memory per seat
(1000-seat headless fixture under an RSS budget); bus fan-out (throughput
and p99 under Oracle bursts with the weighted fair queue); supervisor fds
and poller (firehose at 1000 seats, p99 write latency); sidebar and minimap
draw (1000 rows, 50 visible, hitch meter). Budgets come from the first
measurement, never a guess ([[runtime]]).
