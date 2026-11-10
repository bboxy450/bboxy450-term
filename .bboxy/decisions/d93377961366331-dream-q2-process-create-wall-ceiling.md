---
type: decision
title: 'Dream Q2: process_create wall ceiling 250 ms, excess over 25 ms recorded'
status: decided
derived_from:
- '[[decisions/d93377961366280-dream-q2-external-helper-budget]]'
- '[[decisions/d93377961365126-dream-q2-helper-creation-and-lifetime-budget]]'
id: 01M6WNB33V85A6GT5JDB9AY836
updated: 2026-11-08T04:05:54.411Z
created: 2026-11-08T04:05:54.411Z
number: 93377961366331
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
updated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
authorship:
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  kind: seat
  public_key: DbWCjlMnSNUcBZMB2JrdfJ36/iQgSFRbtz78XE3WlhU=
  process_generation: '1'
provenance:
  digest: sha256:ce5ba83befa60bc072f6c3104bda4ac10b8a738a0fcdc3d7a7c71ca68286e59b
  frontmatter_digest: sha256:023d5054742f625088753fbebdaf9b4006f6b9844a7492772544e1d725315797
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  sig: 9/1BHbhocMpINHiBEQfZNq6vKqk/Np8qePDgkDSpLsiINPTChz+HX/keqE2/Qw7l6edDa2Wg/pgsGG0GSzNnDA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150547
  event:
    id: 01M6WNB3TK9AX14T67WKA8Y133
    sequence: 150547
    hash: 13b79c0943c6c3b8943ea5713a7d1d0ec479766c5a698f0658689052baecc384
---
Context: builder run 01M6WMTMXKA8BFZHWGQP7FX028 on frozen INPUT58 (ba22c67c), controller 52183, ended with actual wait4 exit 81.
- The first prepared posix_spawn (65086 process_create tag) took 34.686 ms of native wall against the 25 ms limit. CPU was 0.563 ms; inclusive time was 0.689 ms.
- Machine load average was about 4 on 10 cores.
- The helper was killed and reaped by its owner; the group is gone. Debt of 7.380 s was paid.
- Ledger sha256 99d0a765...a720.
66280 already retired every per-grant wall bound because wall time depends on load, and made wait4 CPU the verdict. It kept the 25 ms process_create wall unchanged.

Decision (Oracle, under never-block-the-swarm; the Architect may revise):
1. process_create only: CPU stays at 2 ms (hard) and inclusive stays at 3 ms (hard). The native wall ceiling becomes 250 ms (hard): above 250 ms the call is still refused, with the effect retained, killed and settled as before.
2. A wall time between 25 and 250 ms is admitted, but recorded: the exact excess goes into the terminal/ledger as process_create_wall_excess_ns plus a count. It is never clamped or hidden.
3. All other native classes keep 2 ms CPU, 25 ms wall and 3 ms inclusive. The ordinary 1 ms quantum, 9x debt, the 250 ms per-command and 3 s per-job CPU caps, the 30 s job deadline, the 16 MiB RSS limit and the reap/settlement law are all unchanged.
4. Reason: during a spawn, the proof owner is parked and the protected control thread is independent, so a slower spawn does not delay cancel or control. Wall time under contention costs no CPU, and debt is charged on CPU. The 250 ms ceiling still bounds an uninterruptible spawn: one ceiling of 250 ms is under 1% of the 30 s deadline, and 16 forms at the ceiling would be 4 s.
5. Required controls: a positive with a 34 ms process_create wall is admitted and records its excess; a 251 ms wall is refused (exit 81); a CPU mutant above 2 ms is still refused; a mutant using the old 25 ms bound must fail the 34 ms positive (broken control).
6. This is not a lucky retry. The rerun is valid only on a new source that implements this decision, and run 01M6WMTMXKA8BFZHWGQP7FX028 stays FAILED 81.

Alternatives rejected:
- Keep 25 ms and rerun until it passes: a lucky retry, which is forbidden.
- Drop the creation wall bound entirely: leaves no ceiling.
- Raise every native class: wider than the evidence supports.
