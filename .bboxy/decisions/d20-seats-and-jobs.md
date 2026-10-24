---
id: "01M5JTT00RB9RE27A0NVMCPTMW"
type: "decision"
number: 20
title: "A seat is a PTY; a pipe-mode harness run is a job"
status: "decided"
decided: "2026-10-22"
operator: []
links:
  - { to: "[[2026-10-22-architecture-round-1#20]]", rel: "supersedes-as-index" }
created: "2026-10-22T22:13:19Z"
updated: "2026-10-24T00:10:38Z"
---
# Decision 20: A seat is a PTY; a pipe-mode harness run is a job

**A seat is a PTY; a pipe-mode harness run is a job (from the measured
PTY cap).** `sudo sysctl -w kern.tty.ptmx_max=1100` returns Invalid
argument; the believed hard cap is 999. Decided: (a) per-machine gate
budgets are set at the machine's measured maximum; (b) headless harness
runs (`claude -p`, `codex exec`) never become seats; they are **jobs**
on the existing `spawn_command` path, a row under the launching seat
with Inferred grade, no screen, no input, no PTY, and therefore not
capped; the seat model, the sidebar and the 7501 observation stay
untouched; (c) at the cap the product refuses with the reason and
offers another host ([[d19-pty-ceiling|#19]]). "1000 agents" is a swarm across hosts, or a
few hundred seats plus any number of jobs. Rejected: pipe-mode seats
(a seat without a screen breaks every seat invariant: observation,
guard, scrollback, resume); raising the target by lying about PTYs.
A job row spec is written when a brief needs it, not before.
