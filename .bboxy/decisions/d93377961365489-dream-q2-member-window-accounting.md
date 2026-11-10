---
type: decision
title: Charge Q2 helper member windows and only proven stopped corroboration
date: 2026-11-05
derived_from:
- '[[tasks/2026-11-05-dream-q2-member-window-disposition]]'
- '[[decisions/d93377961365126-dream-q2-helper-creation-and-lifetime-budget]]'
id: 01M6PWC6DKK09SAD167YQQCNME
updated: 2026-11-05T22:13:23.869Z
created: 2026-11-05T22:13:23.869Z
number: 93377961365489
created_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
updated_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
authorship:
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  kind: seat
  public_key: L62ggzXO9nS9icBZrd6nPN71BnmasbyzYyfTJE3eu0k=
  process_generation: '1'
provenance:
  digest: sha256:f99e80a49ea9e725549af7e08693ea99adbb9e992c48525e729e5a801d48dab9
  frontmatter_digest: sha256:ee199b792e69452cd93433f471fd8aff353f83fe65a801babc6993aec26a4f44
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  sig: p7xWYB1E/roiGB6bZ2Dc7ShygHsvF/331HxNailb9DRS1nMxIv1LR+hitF0hYqHM3hsXfLbNElYOVBH49trSAw==
  log: .bboxy/log/2026-11-05.jsonl#144838
---
# Q2 helper grants: per-member possible-runnable windows

Date: 2026-11-05. Narrow correction to the helper-lifetime accounting paragraph of [[decisions/d93377961365126-dream-q2-helper-creation-and-lifetime-budget]], inherited by [[specs/dreaming-mcp]] / [[interfaces/dreaming-mcp]]. It does not change C1 native quantum65086, public/private wire shapes, IR, versions, accepted components, tool allowlist or resource allocations. Task: [[tasks/2026-11-05-dream-q2-member-window-disposition]].

## Evidence and choice

Read-only Q2 snapshot at 22:09:48 UTC: HEAD `3d9fa84f196ea9b1b643939e7ae035a7dd0a6c9c` plus active author changes. `src/dream/c1.rs:1151–1182` multiplies elapsed time through finish_grant by expected_members; :1185–1598 samples custody/image/usage/RSS/closed roster; :1679–1775 includes that sample's CPU in the running grant. The frozen `target/dream-read-control-state-x6qosf3h/proof-helper-probe.c:445–551` already separates some stopped corroboration, but charges both slots through the last acknowledgement.

Retained status is **81**, stdout empty, clang.stderr empty. `stderr.log:3–4`: sample140 charge1,682,500ns, actual helper delta2,792ns, active controller636,500ns, late_kernel0. From its formula, 2*523,000ns +636,500ns fails1ms. Merely replacing2 with1 yields1,159,500ns and also fails. These are failed baseline data, not measurements of the design below.

Alternatives: retain the blanket reserved-count multiplier; reduce the multiplier/waive controller work; or charge conservative per-member windows and separate work only after a real stopped-roster boundary. Choose the last, conditionally on demonstrated coverage. Reserved capacity is not evidence that every slot stayed runnable until the last member stopped. Conversely zero CPU delta, sleep, waitpid or a last-seen blocked thread cannot close a window. No general native exemption or promise of OS preemption is introduced. Current tools remain **verification_unproven** until their exact provisioned capability and qualifying process witnesses establish this rule; a known timing/cap breach remains resource_limit.

## Exact accounting and ordering

One existing controller/group per job, ≤2 logical process slots, one runnable thread per provisioned process, ≤256 consumed creation reservations including implicit children. No second controller. Existing 30s/four-job/two-root/foreground/control/heap/reap bounds and START_SUSPENDED birth remain.

| Quantity | Definition / owner |
|---|---|
| Reservation N | One direct helper or Git+one possible GPG, fixed before birth. Does not shrink a live uncertainty interval. |
| t0 | Checked monotonic sample immediately BEFORE the actual CONT call, after stopped attachment, prior debt, authority/activity/lease/currentness checks. CONT's whole delivery latency is covered. |
| Window w[i] | Possible executable interval for a reserved slot, initially OPEN at t0. Known members close only at a checked clock AFTER positive identity-bound stop/exit acknowledgement. No backdating to signal send, first sleeping observation, timestamp of an old snapshot or estimated actual stop. |
| Unknown slot | If Git may fork, reserve/open its second slot at t0 even before any child appears. Retain through discovery, fork/exec and stop latency. If no child exists, close only after the leader is demonstrably stopped and a complete closed roster excludes both live and unaccounted transient descendants. Otherwise keep unknown; absence in one census is insufficient. |
| W | Checked sum of lengths of possible-runnable window unions across ≤2 slots. Within one logical process, Git-clone→GPG exec is one continuous covered slot, not a free new incarnation or double-counted third process. |
| A | Observed direct+descendant CPU, with explicit Mach-unit conversion, monotonic counters and nonduplicated roll-up. Samples/reap reconcile W; they do not grant work. |
| C_active | ALL attributable controller and pipe CPU from grant admission through positive stopped AND closed-roster boundary, including partial/failed observations and actual clock/identity checks. Sum the fixed owners' CPU spans once, not just whichever thread closes the grant. |
| Q | At closure, max(W,A_active)+C_active; ≤1ms ordinary grant gate. Preserve W, A, C and inclusive maxima separately. Unexplained A_active>W invalidates coverage even if Q≤1ms; retain actual charge/debt and refuse trust. |
| C_stopped / L | Post-closure corroboration/pipe/retirement CPU and newly observed late kernel CPU. Separate finite ordinary paced units, each inclusive charge≤1ms, with fresh debt after observation. No prior stopped time pays a newly discovered charge. |

A positive per-member acknowledgement requires the original op/helper sequence/PID/PGID/birth/start-abstime and current audit-task/exec incarnation, expected image phase and provisioned one-thread predicate, explicit stopped state plus zero running threads, or exact retained exit/resource evidence. A successful STOP, ESRCH on an old audit token, numeric PID match, blocked state or zero delta alone is not acknowledgement. Reader endpoints are host post-call monotonic clocks; kernel snapshot fields are observations, not exact historical stop timestamps.

Tentative member endpoints may differ. They become usable only after the SAME grant closes its complete roster: recheck exact group membership, parent/child edges, identities and stopped/exit continuity; retain all original effects. No further CONT, uncontrolled signal owner or admission occurs during closure/corroboration. Unknown/changed/newborn members cannot disappear from accounting: extend their possible slot from t0 (or an earlier still-open uncertainty boundary), capture/stop the actual new incarnation and re-establish closure. A later exec invalidates an old-task stop receipt; a pending/unknown exec cannot inherit its endpoint. If continuity cannot be established, keep coverage unknown and settle/refuse. Do not retrospectively invent a tighter birth time from later discovery.

The supported tool/OS capability must justify the ≤2 processes/one runnable thread restriction across the whole command, including implicit Git fork/exec and fast-exited descendants. Runtime child/group censuses plus retained execution/exit/roll-up evidence corroborate it; samples alone cannot prove no unseen fork/thread/escape. Missing support returns verification_unproven. No broad process scan or new trace/controller service is selected.

**Closure is a causal state transition, not a label on sample().** t_close is sampled after the final mandatory stop/incarnation/roster predicates. Work needed to establish those predicates remains C_active. Only redundant or later metadata work (e.g. retained usage/RSS/report corroboration) may move after this boundary, and only if it is not needed to establish it. `PROC_PIDTASKALLINFO` is not an atomic stop/exec/roster seal. Keep expensive image/token discovery in the active interval when necessary. Fixed pre-grant prepared bindings can avoid repeat preparation; they cannot waive current identity/coverage.

After closure, finalize Q and pay ≥9*max(1ms,Q) rest. Later stopped units each pay ≥9*max(1ms,C_stopped+newly_attributed_L), after their own observation. Debt is additive/nonborrowed and survives failure/ABA; do not reset a deadline from now in a way that drops existing debt. No helper CONT, main proof/hash/parser or another expensive job-local admission while closure, corroboration or any debt is unfinished. Independent bounded Status/Cancel/pause continues; foreground revokes immediately, with existing hard acknowledgement gates.

Late CPU while stopped must retain same incarnation and stable user-work counters with an explained kernel-only delta; record it, charge once as fresh ordinary work/debt and revalidate the stopped seal. Unexplained user growth/unknown clock/unit/coverage refuses and preserves custody. Actual CPU over W cannot be silently capped, subtracted or called idle. Use baseline/interval ledgers to avoid counting direct+rolled descendants twice; report conservative charges separately from observed CPU. All later cleanup/reap and diagnostic cutoffs remain in terminal accounting.

Failure during CONT/STOP/clock/census retains original operation/effect, handle/PID-start/audit incarnation and every open window. No assumed no-effect or replacement retry. Stop/revoke, paced retirement, actual reap/group settlement or owned quarantine follows existing65126; an open window may produce a retained over-budget failure, never an available lesson. Unknown clock is not zero. Retirement cannot discard endpoints or release signaling rights before its identity-safe handoff. Output trust requires successful proof plus complete accounting and actual settlement.

## Implementation boundary and verification

Q2 branch only: `src/dream/c1.rs` HelperGroup/sample/finish_grant/helper_tick owns fixed slot windows and staged closure/corroboration; `read.rs` helper_poll/retire/usage/detach merges exact pipe/retirement/reap attribution and residual debt; `resolve.rs` keeps original suspended birth/tool/custody and bounded receipts. Reuse the existing controller/proof owner. `tests/fixtures/dream-read/proof-helper-probe.c` must witness the same phase boundaries and source-bound capability; `tests/dream_read.rs` retains raw clocks/member ledger, provision receipt, successful prepared publication and corrupt-proof failures. No wire/schema/helper-version bump or new entitlement. Oracle must amend the live Q2 task before author implements this rule.

Finite model: one group, two slots, attempts0..2 abstracting256, epochs A1/B2/A3, bounded monotonic clock/cost0..3 and additive debt0..27; slot absent/possible/running/ackStopped/ackExited/unknown, incarnation/fork-exec phase, tentative endpoint, rosterClosed, workOwner, actualCPU, active/post CPU, coverage, foreground/deadline and trust. Initially no helper/effect, no debt, untrusted. Actions Reserve/CreateStopped/Attach/Grant/OpenUnknown/ForkExec/StopRequest/AckMember/CloseRoster/Charge/StoppedCorroborate/Reconcile/Rest/Revoke/Retire/Reap/Restart/Trust. Only internal control/settlement is weakly fair; arbitrary syscall completion/success is not assumed. Unknown dependencies end in refusal/owned quarantine, not fabricated progress.

| Property / causal broken control | Required real unattended witness |
|---|---|
| Window covers every possible runnable interval; unequal stop delays | Delay just one member's positive acknowledgement; compare raw per-member endpoints and W against blanket last-stop charge; no backdating. |
| Unknown fork/exec/member/retired coverage stays charged | Birth after initial census, late exec task, short-lived/escaped/extra member, stale retired token; catch or Unproven, no trusted output. |
| Blocked/sleep/zero delta is not stopped | CPU-burning GPG while Git waits; false-stop mutant must violate the named coverage invariant and fail actual fixture. |
| C_active includes work before closure; stopped work pays fresh debt | Move one census/required identity check after a fake seal; missing-charge/moved-controller controls fail. Valid post-seal heavy corroboration remains≤1ms/unit and paced. |
| A/late kernel complete, no debt reset/overlap | ActualCPU>W, late kernel after prior rest, double roll-up/omitted pipe, debt reset or CONT/main overlap mutants; retain excess and fail, fresh post-observation debt. |
| Custody through failure and A→B→A | Failed/late stop, missing/reversed clock, stale incarnation, cancel/revoke/panic during run/closure/rest/retire; actual reap or quarantine, no retry/trust. |

Run existing branch targets `make test-dream-mcp` and `make test-formal-parity` with source/model/tool hashes, Yarn/native versus pinned Java TLC full finite-graph/verdict parity and named causal controls. No build or suite ran for this decision. Acceptance still needs actual provisioned Git→GPG qualifying publication completion, corrupt-proof negative, four independent qualifying jobs, control/foreground/heap/30s/real-reap gates and complete per-unit receipts. A component capability probe is not managed-worker or MCP availability proof.

## Source limits and documentation

Frozen probe SHA256 `6315a34b672cdb597fb4a5a073d59a406dba3849b9b0c67fcb09787c4f7cae59`; c1.rs snapshot `05d54fba1c1be5f5cee07215098cecc24117081614e310b62fa5aae90289d150`. `resolve.rs:270–334` checks a32KiB source/OS/tool/capability receipt; `tests/dream_read.rs:284–318,515–520` resolves an explicit Xcode Git and absolute Homebrew GPG and binds hashes/builds/source/probe/OS. This failed retained attempt has no completed stdout/capability receipt proving availability; no exact new tool version is asserted here.

Primary documentation fetched2026-11-05: [Apple proc_taskinfo fields](https://raw.githubusercontent.com/apple-oss-distributions/xnu/main/bsd/sys/proc_info.h) :119–142 exposes status/thread observations; [Apple PROC_PIDTASKALLINFO dispatch](https://raw.githubusercontent.com/apple-oss-distributions/xnu/main/bsd/kern/proc_info.c) :2164–2174 fills BSD and task data separately. Inference: one returned struct is not a transactional stop/exec/roster barrier. Public main source is explanatory, not proof of this machine's exact kernel/tool build; supported receipts/process evidence remain required. No arbitrary OS timing/native-memory guarantee or qualifying positive is claimed.
