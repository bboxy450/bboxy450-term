---
allowed_paths:
- .bboxy/decisions/
- .bboxy/specs/
- .bboxy/interfaces/
authorship:
  kind: seat
  process_generation: '1'
  public_key: 0X00kpb5zTGUeot1iS3xss6zajL5tcixiAgioL6WETw=
  signer: seat:01M878FXVG76NYQYEEEXAHQ108
created: 2026-11-09T12:15:47.503Z
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
delegated_by: seat:01M878FXVG76NYQYEEEXAHQ108
derived_from:
- '[[decisions/d93377961366280-dream-q2-external-helper-budget]]'
- '[[decisions/d93377961366331-dream-q2-process-create-wall-ceiling]]'
- '[[tasks/2026-11-05-dream-proof-tools-provisioner]]'
id: 01M703RTB5MD2D9V70AV66SPND
number: 93377961366456
reconciliation:
- by: seat:01M878FXVG76NYQYEEEXAHQ108
  disposition: implemented
  evidence:
  - decision d93377961366459-dream-external-helper-shim-enforcement.md — derived_from this task
  fresh_accepted: false
  kind: historical
  op_id: 01M878KGG3NYPYPC56XMEVTCBH
  previous_status: open
  reason: Decision written.
role: architect
status: done
swarm: 01M649102FBS60QWNPZS5VAGF9
title: 'Architect review: external-helper budget enforcement after INPUT54-69 (continuous observation vs kernel rlimits)'
type: task
updated: 2026-11-24T17:10:39.624Z
updated_by: seat:01M878FXVG76NYQYEEEXAHQ108
verify: review-only
provenance:
  digest: sha256:84938fafe24673af8225f1585c8b323d5627e948dc15601266fc1d9488f754f2
  frontmatter_digest: sha256:3769c14dda764172b2f7d9d81c62c0a12eb58d7972cb9f6e2040d0a936ffa1c3
  signer: seat:01M878FXVG76NYQYEEEXAHQ108
  sig: aVpsWgF/bN5naPb73ZYa/2tvnB/Bp9ikMknlvp040Pp7vPdpiEXHDOtXCxd5nNzX/0bTdt+2q0fLXFVnYgL3CA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1883
  event:
    id: 01M878KHD07YE8MSJS9YENA9TW
    sequence: 1883
    hash: d7b85bd8476953532fd626cea7e3c873a6ae7f4a9831e8374c53748eb164451d
---
ROLE: ARCHITECT

## Why this review
The private real git/gpg candidate qualification (task 2026-11-05-dream-proof-tools-provisioner, executed with Q2 under decisions 65086/65126/66280/66331 and the Oracle rulings in that task) has run INPUT54 -> INPUT69 over about 2 days. Each real run surfaced a NEW physical failure class in the live-observation design of the external helper:
- argv expansion (^{tree});
- process_create wall under load (-> 66331);
- a TASKALLINFO ESRCH race;
- a missed verifier between polls (-> kqueue NOTE_FORK);
- a leader-reaped child with no RI exit (-> knote NOTE_EXIT witness);
- an ordinary unit over 1 ms;
- the same-wake admission regression;
- members_max bookkeeping;
- the pause 50 ms gap during a stop (-> stopped-interval rule + SYSTEM <= 50 us);
- a STOP vs exit race;
- audit-token/BSD exit races (-> systematic observe_or_terminal at 27 sites);
- now (INPUT69, run 01M7035811Q2HTZ37K75WQ5K3C): "foreign or reversed live helper resource identity" (proof-helper-probe.c ~3220 engine_usage_identity: RI start changed or counters decreased), with the child never admitted. That is likely a stale member slot.
Converged so far: the POSITIVE passes on three inputs (66, 67, 69) with all 28 terms true and margins of 14-18 s. corrupt_proof passed on 66 and 67. pause_resume has never completed. The Oracle set a checkpoint: if INPUT69 failed on a non-exit-race class, the design goes back for review. It did.

## Question for the Architect
Is continuous per-member live observation (RI sampling every <=50 ms, 27 observation sites, per-member identity/admission/rollup custody, GS1/GS2 on a transient verifier) the right mechanism to enforce the external-helper budget? Or can the same law (250 ms per command / 3 s per job CPU, 9x debt, 30 s, 16 MiB RSS, group custody, no escaped effects) be enforced with far fewer moving parts?

## Candidate simplification to evaluate (Oracle suggestion, not a ruling)
- setrlimit(RLIMIT_CPU) (and RLIMIT_AS/RSS where enforceable on macOS) set on the helper leader in posix_spawn's pre-exec path or a tiny exec shim; it is inherited by gpg. The kernel then enforces the per-command CPU cap; SIGXCPU/SIGKILL is the refusal.
- The terminal bill stays the leader's own wait4 (subtree roll-up already accepted in 66236/66280).
- The process group plus the kqueue NOTE_EXIT/NOTE_FORK knotes stay for custody and escape detection.
- Live RI sampling becomes optional diagnostics, not a gate.
- Pause = SIGSTOP the group, gated only on the positive stop ACK and the wait4 bill.
Evaluate soundness (does RLIMIT_CPU on macOS cover the subtree and system time? granularity? escape via setsid/daemons?), what the law loses, and whether 65126/66280 need amending.

## Deliverable
A decision record (amend or supersede 66280), with:
- explicit invariants;
- what the finite model must cover;
- a minimal implementation outline Q2 can execute in ONE cut.
Until then the INPUT loop is paused. No product feature is blocked: Q1/Q3 Dream reads are landed and real lesson serving (Q3b) needs a publication pipeline first.
