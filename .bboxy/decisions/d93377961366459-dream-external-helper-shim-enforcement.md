---
type: decision
title: 'Dream external helpers: enforce in-process (exec shim + RLIMIT_CPU + in-shim GS1), verify once by wait4; drop live observation gates'
status: decided
derived_from:
- '[[decisions/d93377961366280-dream-q2-external-helper-budget]]'
- '[[decisions/d93377961366331-dream-q2-process-create-wall-ceiling]]'
- '[[tasks/2026-11-09-architect-review-external-helper-budget]]'
id: 01M70FFZ0PHRJ4HJMKS7Y7JPYD
updated: 2026-11-09T15:47:16.681Z
created: 2026-11-09T15:40:40.184Z
number: 93377961366459
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
updated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
authorship:
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  kind: seat
  public_key: DbWCjlMnSNUcBZMB2JrdfJ36/iQgSFRbtz78XE3WlhU=
  process_generation: '1'
provenance:
  digest: sha256:2b4bbf9d9f4e8dc92c0ead35c384bd7ee5673c576c53b116f089a3dcc942e7fd
  frontmatter_digest: sha256:eaed0b8728c7f2bc8d71e40b8cfbba49184ee007f69a8fb8316a2c4c6ed3886b
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  sig: ktek0w89ar6OZd7RGOdlrzHpG/xggDK1xPxmgqjEgsFDn2Qlmq+mRr75TegvJYJuQjw063mRScwGouM+xFo6Bw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151035
  event:
    id: 01M70FW2PGZ8TENHVX3P8ZX0SN
    sequence: 151035
    hash: 1d705b82806b9695d32c13fa8e6de0f6adf9c7bc07311be16a58a4e958ccff0a
---
Review requested by the operator ("do review yourself"), 2026-11-09. Supersedes the LIVE-ENFORCEMENT and verifier-custody MECHANISMS of 66280 item 4 (live part), item 8 (members discovered by sampling), 65960 GS1 (pre-ACK verifier observation) and 65489 remnants. It keeps every LAW value.

## Finding
INPUT54-69 failed on a new physical class almost every run, always in the same mechanism: the controller OBSERVING live external processes from outside (sampling RI every <=50 ms across 27 syscall sites, per-member identity/start/counter monotonicity, admitting a transient gpg child in the same wake, then STOP/ACK before GS1). Each observation is a check-then-act race against a process that may exit, reap or be reaped at any instant. The law values were never the problem: the positive passed on INPUT66/67/69 with 14-18 s of margin and helper CPU around 1-1.4 s. The design was: it tries to prove, from outside, facts the kernel and an in-process check can enforce directly.

## Decision: enforce INSIDE the process image, verify ONCE at the end

1. **One audited exec shim, `dream-exec-shim`** (C, part of the qualified source and candidate prefix, compiled under the same strict flags and header audit). It is the ONLY thing posix_spawn starts for a helper, AND it is configured as git's gpg.program. Before execve it:
   a. validates its own argv/env against the EXACT 65126 allowlist for the requested form, including the gpg verify form and the signature temp-file argument. Any mismatch exits 81 before exec;
   b. for the gpg form, performs the GS1 custody checks in-process on the temp file it was given (nofollow, regular, 0600, euid, nlink 1, dev/ino/size/sha256, exclusive scratch) and records them to an owned, preopened log fd. This REPLACES the outside STOP/ACK/same-wake GS1 observation;
   c. sets RLIMIT_CPU (soft = hard = the per-process share, below) and RLIMIT_CORE = 0;
   d. closes every fd except the allowlisted pipes and the log fd, then execve()s the real tool by absolute path.
2. **Per-process CPU share.** Each process image the shim starts (at most 2 per group: git and gpg) gets RLIMIT_CPU = min(125 ms, remaining_job_budget / 2), rounded DOWN to whole seconds' resolution where the API forces it.
   - If the platform cannot express sub-second RLIMIT_CPU, the kernel limit is the 1 s floor and the 250 ms law stays enforced by the TERMINAL wait4 verdict (item 4 below). In that case the kernel limit is only a runaway backstop. Record this as an explicit assumption, measured by Q2.
3. **Group custody stays:** POSIX_SPAWN_SETPGROUP plus background QoS at birth (66280 item 1), the 66331 process_create bounds, a kqueue NOTE_FORK/NOTE_EXIT knote on the leader (counts forks; more than 1 fork, or any fork not ending in the shim, kills the group), and the 30 s deadline kill.
4. **Terminal verdict (unchanged law):** the leader's own wait4 (subtree roll-up, 1 us round-up) <= 250 ms per command and <= 3 s per job; ru_maxrss <= 16 MiB; 9x debt after reap; then a final census/group-gone and the GS2 post-reap temp-file check. Everything else is a refusal with retained excess.
5. **Removed as GATES** (they may stay as optional diagnostics, never refusals):
   - live RI sampling and the 50 ms gap rule;
   - per-member RI identity/monotonicity (engine_usage_identity);
   - same-wake verifier admission and the STOP-before-GS1 path;
   - members_max derived from sampling (now: fork count from the knote, at most 1);
   - per-member child-rollup proof (child CPU is in the leader's wait4; an escaped child is caught by the census).
6. **QoS gate:** the shim verifies its OWN base priority equals the throttled band (Darwin 4) before exec, which is inherited. A mismatch exits 81. The outside observation of priority is removed.
7. **Foreground pause** (66280 item 8): one SIGSTOP to the group. ACK = for each process from a fresh census, a stopped status or the terminal path through observe_or_terminal (already landed in 637cb234). USER growth across the pause must be 0 and SYSTEM growth at most 50 us (the Oracle ruling in the provisioner task). One SIGCONT. The 100 ms gate is unchanged.

## Why this is sound
- The kernel enforces CPU per process and cannot race; the shim validates argv/env/temp-file custody from INSIDE the image that is about to run, so no other process can intervene between check and exec (the scratch dir is exclusive per 65960).
- wait4 already is the accepted verdict (66236/66280).
- No live observation means none of the 27 race sites participate in a verdict.

## Assumptions to MEASURE before relying on them (Q2, real-process witnesses, no inference)
- XNU RLIMIT_CPU: the resolution actually honoured; the signal sequence at the soft and hard limits (SIGXCPU vs SIGKILL); whether system time counts.
- That git invokes gpg.program = shim with the expected argv form for verify-commit/verify-tag in the pinned git 2.54.0.
- That an RLIMIT_CPU set before execve survives exec into the hardened gpg binary.
Any assumption that fails is reported with its witness and this decision is revisited; nothing is silently relaxed.

## Verification
- Replace the Q2HelperTotalBudget model with a smaller HelperShimBudget model: states, the fork knote, kill on extra fork, the wait4 verdict, debt, pause. Keep a broken control per gate.
- Real controls:
  - shim argv mismatch -> 81;
  - temp-file custody violation -> 81;
  - CPU runaway killed by the rlimit, and the wait4 verdict refuses;
  - an extra fork kills the group;
  - an escaped setsid child is caught by the census;
  - a positive and a corrupt proof.


## CORRECTION (same day, measured): RLIMIT_CPU is NOT enforced on this macOS. Items 1c and 2 are WITHDRAWN.
Witnesses (Darwin 27.0.0, 2026-11-09, run by the Oracle):
- `sh -c 'ulimit -t 1; exec perl <4 s CPU loop>'` printed "survived" and exited 0;
- `sh -c 'ulimit -t 1; exec yes >/dev/null'` was still running after 4 s;
- python children with setrlimit(RLIMIT_CPU,(1,1)) after fork consumed 5.000 s of CPU and exited normally, with no SIGXCPU and no SIGKILL.
So no kernel per-process CPU cap is available. The replacement for items 1c/2 and 5 (live sampling):
- **Live CPU backstop, non-gating on failure:** the controller still reads the leader's (and any known child's) RI in its ordinary units at most every 50 ms. A SUCCESSFUL read showing user+system above the cap kills the group (resource_limit, with the excess retained).
- A FAILED or racy read is NEVER a refusal: no coverage_unknown, no identity/monotonicity checks, no gap rule. It is ignored, and the terminal wait4 decides. The 30 s deadline still bounds any runaway.
- The terminal verdict (item 4) is unchanged and remains the only CPU proof.
Everything else stands: the exec shim with in-image argv/env/allowlist validation and GS1 temp-file custody, the leader fork knote, the census, wait4 as the verdict, debt, pause, QoS self-check in the shim, and the removal of the observation-based GATES listed in item 5.
The HelperShimBudget model must include "live read failed/ignored" and "successful over-cap read -> kill" actions, plus a broken control where a failed read refuses (the old behaviour).


## AMENDMENT 2 (2026-11-09): fork/escape boundary (executor counterexamples, research #93377961366461, commit 29fd9039)
Real Darwin witnesses show:
- two forks can coalesce into ONE NOTE_FORK (data=0);
- a setsid orphan stays live after the leader's wait4 while the child and group censuses are empty.
So "more than 1 fork kills the group" (item 3) and "an escaped setsid child is caught by the census" are WITHDRAWN as enforceable claims.

Replacement:
1. **Premise (explicit, already law in 65654):** the fork/exec/escape shape of a helper command is bounded by the SUPPORTED-TOOL basis: the pinned, authenticated, source-built git 2.54.0 / gpg 2.5.24 with their recorded source-review reachability for the fixed command forms. Git forks only the shim (gpg.program -> gpg); gpg runs with no agent/daemon autostart. The verdict stays conditional_on_bound_source_review, and whole_command_lifetime_proof=false (unchanged). Any form or tool outside that basis is unsupported and refused.
2. **Best-effort detection (refuses when it fires; silence proves nothing):**
   - any NOTE_FORK observed on the leader beyond the expected shim/gpg exec chain;
   - a post-reap census finding a live process in the group or in the leader's child list;
   - the shim log showing an unexpected exec form.
   Any of these -> resource_limit/coverage_unknown with retained evidence. Record the observed fork-note count as a diagnostic, never as an exact count.
3. **Acceptance controls change accordingly:**
   - "extra fork kills the group" becomes "an extra fork that IS observed (an uncoalesced note or a census hit) refuses";
   - the setsid-escape witness is retained as a DOCUMENTED LIMITATION (negative evidence), not a pass criterion;
   - the shim must refuse any argv form that would make the pinned tools daemonize or autostart an agent (for example gpg without --no-autostart when it is required).
4. Optional hardening (NOT required for INPUT70; a separate future decision): a seatbelt profile restricting process-exec to the shim/gpg paths.
