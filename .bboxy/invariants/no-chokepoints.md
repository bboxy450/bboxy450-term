---
id: 01M5QPC51Y45D28JR1FXZW118A
type: invariant
number: 287
title: "no bottlenecks and no chokepoints, in the swarm and in the product"
status: active
check: sh scripts/never-block.sh && sh scripts/cross-os.sh
cost: one approval, one agent, one queue, one worker, one scan, one pool or one lock that everything waits on is the swarm or the app running at the speed of its slowest part
links:
  - { to: "[[never-block-swarm]]", rel: sibling }
  - { to: "[[idle-swarms-cost-nothing]]", rel: constrains }
  - { to: "[[watched-replay-priority]]", rel: constrains }
  - { to: "[[d267-idle-swarms-cost-nothing]]", rel: decision }
  - { to: "[[d253-checkpoint-policy]]", rel: decision }
  - { to: "[[d285-no-gated-communications]]", rel: decision }
  - { to: "[[mantras]]", rel: mantra }
created: "2026-10-24T19:32:03Z"
updated: "2026-10-24T21:17:17Z"
---
# No bottlenecks and no chokepoints

Operator, 2026-10-24, verbatim: "no bottlenecks and no chokeponits - that is alvo invariant in our swarm design and architecture". Two halves.

**Property, the swarm.** No single approval, grant, agent or queue that work
waits on: gate slots are self-service (`AGENTS.md`), merges proceed on the
author's own gate log, the Oracle rules on the spot and the Architect records
after, blocked work is reassigned and never parked ([[never-block-swarm]]), an
Oracle brief is authorization for in-scope Executor work
([[d285-no-gated-communications]]).

**Property, the product.** No design where one path serialises everything: no
single worker, no global scan over things that exist rather than things that
happened, no fixed pool everything queues on, no lock held across IO. Known
instances, each with its own gate: the message pump that scanned every swarm
([[d267-idle-swarms-cost-nothing]], gate [[idle-swarms-cost-nothing]]); serial
replay that starved watched seats ([[d253-checkpoint-policy]] R5h, gate
[[watched-replay-priority]]); the GUI knowledge read held on logic (fixed, the
held-read responsiveness check); the Oracle as the slot-grant chokepoint
(fixed, self-service slots).

**Wire grammar of the self-service protocol (2026-10-24).** Self-service only
works when every detector parses the channel the same way, so the slot and
quiet lines are prefix-matched, never read from prose. A line **begins** with
one of `SLOT <n> TAKEN`, `SLOT <n> RELEASED`, `QUIET <name> START, until <HH:MM TZ>`
or `QUIET <name> END`; anything after the prefix is a note for humans. A quiet
window opened by key K closes only on a line beginning `QUIET <name> END` from
the same key K, or when its deadline passes; the words "QUIET END" anywhere
else in a message are prose and close nothing. Cause: Seraph's detector read
Rook's note "or my earlier QUIET END" inside a release line as an END and
started a compile inside the 967 window (stopped after ten seconds, cleaned
up). The grammar is the fix; a detector that matches substrings is the bug.

**Check.** Executable today: `sh scripts/never-block.sh` (no open task parked on
a person) and `sh scripts/cross-os.sh` (no single OS binding path through
shared crates). The product instances are gated by their own invariants,
[[idle-swarms-cost-nothing]] and [[watched-replay-priority]], pending their
tests and referenced here rather than duplicated. **Judgement, not a gate:**
whether a new design serialises everything is read at each architecture round
(the ZOOM OUT of `AGENTS.md`) and in every spec's lens line; a serialising path
found there is a decision to remove it, named with its gate, not a quiet fix.

**Cost of a violation.** One approval, one agent, one queue, one worker, one
scan, one pool or one lock that everything waits on is the swarm or the app
running at the speed of its slowest part.

**Evidence.** The four instances above, all from 2026-10-24; two fixed, two
gated and in flight.

Owner: Corvin Duskmantle <ARCHITECT>.
