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
updated: "2026-10-25T09:54:49Z"
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
(fixed, self-service slots); the supervisor's retained-notification retry
sweep that restarted from the first seat every pass and starved the tail
(Rook's 967/50 analysis: three watched seats, zero admissions in 12,409
attempts; fair rotation approved, gate [[no-seat-starves]]); the
machine-wide sccache server that every gate compiled through (six Executors
saw clients idle at 0 % CPU for three to seven minutes, 2,649 hits against
36,258 misses, so the cache cost every build and paid almost none; fixed at
a29f839d: `scripts/ci-local.sh` and `scripts/invariants.sh` export
`RUSTC_WRAPPER= RUSTC_WORKSPACE_WRAPPER=`, the global config untouched;
an empty `rustc-wrapper` in a repo `.cargo/config.toml` did **not** override
the global one, tested, so the bypass is the environment, not a config file);
logic's serial one-seat `ReadRing` RPCs to the supervisor (the measured 967/50
run on 0e97dc32: 6 824 round trips, 93 % of an 8 618 ms input-to-GPU, decode
0.8 s; Sable designs the batched or pipelined read, gate [[ring-reads-batched]]).

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
**The claim is atomic, the line is a note (Oracle, 026a07af).** After Seraph
and Sable took the third slot at once (four gates on the machine; Seraph
yielded after a minute of compile), the claim moved out of the channel:
`scripts/ci-local.sh` takes its slot with `mkdir /tmp/bboxy450-gate-slot-<n>`,
three slots at 25 GiB free or more, else two, dead-owner slots reclaimed,
exit 75 when all are taken (retry later). The SLOT lines stay as notes for
humans and detectors and still name their slot number, but a channel line no
longer grants or loses a slot; the directory does. For a gate started from a
tree older than 026a07af, which has no lock, the fallback stands: the later
TAKEN line by message time yields within a minute and re-queues, ties broken
by the lexicographically greater key yielding.

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
