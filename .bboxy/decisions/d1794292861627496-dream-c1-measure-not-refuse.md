---
type: decision
title: 'Dream C1: 1 ms stays the target and pacing unit; a step is refused only above 50 ms'
status: decided
decided: 2026-11-12
operator: fix C1 and get that entire thing working ... simplify DREAMING and LESS is more ... build on working stuff rather than try to ship not working complexity ... KISS
id: 01M78Y74794X1NCBYZSC3W41SQ
updated: 2026-11-12T22:31:54.879Z
created: 2026-11-12T22:31:54.879Z
number: 1794292861627496
created_by: seat:01M78E743PF9RK3DA9Y3WRWTWD
updated_by: seat:01M78E743PF9RK3DA9Y3WRWTWD
authorship:
  signer: seat:01M78E743PF9RK3DA9Y3WRWTWD
  kind: seat
  public_key: lZh8jo1S9rWtfvb9+EUefnBFfbJ6FtBuf6dXC19gMpk=
  process_generation: '1'
provenance:
  digest: sha256:357d5cb930395510ad92dac703aceda2314b4a6bbf636bc6c643f3b791bb65a2
  frontmatter_digest: sha256:c94e7fc71727633e8c4d53f24689b65ed01f5fa9ba1fd055ca6b181f5330f1c7
  signer: seat:01M78E743PF9RK3DA9Y3WRWTWD
  sig: BOMGGKBt/NDDlpf6FpA/SYRNc1gfm3yHevAQfZa2n4xNOoJC3Zw9dX45Gmew9tNPgWhqcRoO+Pk9IW+pJZ+ABQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151646
  event:
    id: 01M78Y7506SPSSR47T5M6ZWNM6
    sequence: 151646
    hash: 2e675a6c5d9ee59d8cbaf45af052170d664e764f6f1ab801a9c80a032da81e66
---
# Dream C1: 1 ms stays the target and pacing unit; a step is refused only above 50 ms

Date 2026-11-12. Amends [[decisions/d93377961365086-dream-native-quantum-and-debt]] for
one point only: when a step's CPU time is grounds for refusal. Task
[[tasks/2026-11-12-dream-c1-measure-not-refuse]]. Closes the cause of
[[bugs/mux-dream-c1-cpu-budget-tests-flaky]] and of
[[tasks/2026-11-07-dream-c1-native-unit-budget-overruns]].

## What was wrong

Every cooperative step of a Dream C1 job was refused with `ResourceLimit` if it used more
than 1 ms of thread CPU. A debug build on a shared machine does not hold that: recorded
steps ran 1.0 to 2.2 ms at low load. The refusal therefore measured the machine, and
Dream failed at random, in tests and in the product alike.

Evidence, mux library, `--retries 0`, unchanged source:

| State | Runs | Result |
|---|---|---|
| Before (main at `9b8944fc6`) | 3 | 8, 6 and 4 failures of 517, a different subset each run |
| After this decision, before removing threshold tests | 5 | the same 9 failures every run: the tests that prove refusal at exactly 1 to 2 ms |

Two landings of unrelated work were refused by these tests on 2026-11-12.

## Alternatives

| | Choice | Why not |
|---|---|---|
| A | Keep 1 ms as a refusal and split every code path into smaller steps (the 2026-11-07 task) | Open since 2026-11-07 without result; it chases the scheduler, and every new path reopens it |
| B | Delete every refusal site and the counters that gate on them | About 30 sites and their tests; the larger and riskier change |
| C | Enforce only in a release-profile budget gate | A second profile and a second gate to maintain; the product would still refuse at random |
| **D** | **One threshold constant: refuse above 50 ms** | chosen |

## Decision

- `UNIT` (1 ms) is unchanged as the **target** and as the **pacing unit**: after a step
  the owner still rests nine times `max(1 ms, used)`.
- `OVERRUN` (50 ms) is the only refusal threshold. Every site that compared a step's CPU
  with 1 ms, or a reported maximum with 1,000,000 ns, now compares with `OVERRUN`. Fifty
  times the target still stops a runaway step.
- `max_unit_cpu_ns` and the other maxima keep reporting the real figure, so drift from the
  1 ms target stays visible in the host metrics.
- The separate 25 ms wall-clock limit on a tagged native call is removed. Wall time is
  bounded by the job deadline.
- The nine tests whose only content was "a 1 to 2 ms step is refused" are deleted. The
  tests that inject `OVERRUN + 1 ns` still prove the refusal path.

## What this gives up

The claim that a Dream step never holds a thread for more than 1 ms of CPU. It was not
true before either; it is now reported instead of asserted. The amortized allocation
(rest after work) is unchanged, so a step of 2 ms is followed by 18 ms of rest.

Not changed here: the producer side in `bboxy450-harness`, the worker-metric validators in
`bboxy450-harness-wake-up` (still 1,000,000 ns), heap, deadline, authority and source
rules. If the worker validators flake, they follow this decision.
