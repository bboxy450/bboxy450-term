---
id: "01M5JTRF68MAQDR0SJCZ68CPBN"
type: "decision"
number: 19
title: "The PTY ceiling is a product fact, not a bug"
status: "decided"
decided: "2026-10-22"
operator: []
links:
  - { to: "[[2026-10-22-architecture-round-1#19]]", rel: "supersedes-as-index" }
created: "2026-10-22T22:12:29Z"
updated: "2026-10-24T00:10:38Z"
---
# Decision 19: The PTY ceiling is a product fact, not a bug

**The PTY ceiling is a product fact, not a bug (from Seraph's C1d
measurement).** macOS `kern.tty.ptmx_max` is 511 on this Mac, raisable
only by a transient admin sysctl the kernel caps. Decided: capacity is
published per host and shown on the host row; a spawn past it is refused
with the reason named; the launcher offers the documented raise and
another host; "1000 seats" is per swarm across hosts; per-Mac gates run
at 500 by default and once at the raised ceiling. Rejected: the app
running sudo or installing a LaunchDaemon itself (silent privilege, not
our shape); lowering the target (the target is the swarm, hosts are the
unit); pretending with pseudo-seats that have no PTY (a seat is a PTY).
Gap closed the same day: the raise ceiling is 999 on this build (999 accepted, 1100 refused); the at-limit help's LaunchDaemon sets 999.
