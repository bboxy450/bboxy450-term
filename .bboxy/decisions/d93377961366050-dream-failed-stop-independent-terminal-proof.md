---
type: decision
title: A failed helper STOP is not an acknowledgement; independently proved exit can settle it
derived_from:
- '[[interfaces/dream-proof-tool-qualification]]'
- '[[decisions/d93377961365489-dream-q2-member-window-accounting]]'
id: 01M6RWRRTQAY1RPTTXJXZT6VJ8
updated: 2026-11-06T16:58:44.771Z
created: 2026-11-06T16:58:44.771Z
number: 93377961366050
created_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
updated_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
authorship:
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  kind: seat
  public_key: UgGFd6fs0Ln9gJXlN2fw8kUmonpBkr4IBn/U5N0uOrY=
  process_generation: '1'
provenance:
  digest: sha256:8797e51f134f7c2be4b63d6429a11a5e7a15fd995543fa358d3509c81222b48c
  frontmatter_digest: sha256:28794a7342e3fbf9685334b50d67c61b6adce78cdff4c5dd4618f5d85f9af70c
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  sig: 0tsVCrZF9sbvTrT/IuB0Ynjei1ETFgzdpaCulKCEwnnwLA3u1Vlmt3IUloPEhzgZqpXa5PWz5vXIw505fjEwDg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/149616
  event:
    id: 01M6RWRS8ZPFQQF94KZ29A1JQD
    sequence: 149616
    hash: 0c2e8afa770304adae4cba320ffba61f8b87f7466e974696e9798026570a670a
---
## Evidence and scope
Q2 reported io-batching-transport.log SHA25630c11ee20253bedde7808f0436de9064e58dcca0236db130f608577072097b02, exit101 (1pass/2fail): consumed SIGSTOP returns EPERM for original PIDs51062/51375. Those runs remain FAILED. Their later settlement does not establish that the processes had exited at the failed call.
Root read current src/dream/c1.rs:1104-1128 and:3242: every failed request currently rejects immediately except ESRCH plus positive original unreaped WNOWAIT; the signal receipt is retained first. This is a policy choice, not independent exit proof.
Root fetched Apple's published [XNU kern_sig.c](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/kern/kern_sig.c), exact fetched SHA256343742eb42c5b584c529d348f9772ce03ffedd3be9d986badd60692459b6acdf. Its explicit-group filter excludes zombie entries and its empty eligible set may yield EPERM (raw lines1709-1718). This supports a possible race only: the running kernel/source identity and the cause of either retained failure are UNPROVEN. No OS-specific errno heuristic substitutes for observation.

## Decision
A consumed EPERM or ESRCH STOP failure may enter an explicit terminal-observation state. It is NEVER delivered/stopped/acknowledged, never closes a slot or unknown roster and never establishes exit at the failed-call instant. Retain result/errno/sequence/PID/PGID/incarnation/attempt before observation, including the first failure. Do not implement this as a bare errno acceptance in request_group_stop. Preserve a distinct pending-terminal outcome or equivalent sticky state.
No repeated STOP or CONT is admitted for that consumed grant, no new command/helper/proof/hash work or replacement effect follows while terminal proof is pending. Continue only bounded independent control, exact observation, charging and owned settlement. Existing identity-safe cancellation/escalation may dispose the original effect; no blind PGID signal or identity revival. Other signal errors retain refusal semantics.
Terminal classification requires independent positive WNOWAIT for the original still-unreaped leader; positive current-incarnation exit/RI/audit continuity for every retained member; causally closed roster with no unknown/exec/birth slot; complete late/retired/kernel/controller/pipe usage; actual exact wait4 and group settlement. One exited leader, EPERM itself, empty census, later successful cleanup, changed/reused PID or missing descendant does not satisfy this conjunction. Unknown proof remains verification_unproven/settlement_unknown with original custody. Preserve the conditional reviewed source/platform premise; this is no unseen-work theorem.
Cost endpoints remain conservative: every affected member window stays open from its original pre-CONT sample through its own positive terminal endpoint (unknown slots through complete roster closure), never through the earlier failed STOP. ALL contemporaneous control/pipe/observation CPU remains inside Q=max(sum windows,nonduplicated helper CPU)+controller/pipe CPU <=1ms. No backdating, splitting active corroboration into a free later unit, debt reset, or elapsed/clock/late-CPU omission. Post-stable-closure work and actual teardown retain the existing ordinary1ms and fresh9x debt law. Every other native/heap/control/foreground/30s/reap gate remains unchanged.
A real command may qualify only if that entire independently observed terminal/semantic/cost/debt/currentness/settlement conjunction passes. If the failed signal is followed by a resource, clock, coverage, signature or identity failure, that failure still decides. Historic failed samples are not reclassified or rerun unchanged.

## Implementation and proof
Q2 owns the SAME narrow Rust helper and canonical C controller paths in its existing task. Keep public wire/schema/version and runtime_approved=false unchanged. Extend existing state model with consumed failure, pending terminal, terminal proof and refused/quarantined states; no new scheduler/controller. Native/TLC state+labelled-edge+named-verdict parity and existing final task gates remain.
Required causal controls: EPERM while child remains runnable; positive original terminal after consumed failure with no second signal; live descendant/unknown slot; replaced PID/exec/token; falsely treating errno as ACK or failed-call time as endpoint; dropped controller/pipe/late CPU; over1ms Q; unknown/reversed clock; unpaid debt; failed actual reap. Broken guards must fail. Include a deterministic source-known owned process witness using the same admission/observation machinery, raw signal/identity/window/cost endpoints and actual reap; label injected signal results separately from actual kernel errno. Positive terminal classification must never certify that an injected or historical EPERM was caused by a kernel exit race.
Then run the one materially corrected source-known stdin/transport family, retaining old failures and numeric receipts. Real builder qualification still awaits Root's separate functional C source/executable/compiler input freeze; no real pair birth or runtime approval is granted by this decision.
