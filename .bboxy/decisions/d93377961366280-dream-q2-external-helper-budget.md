---
type: decision
title: 'Q2 external helpers: OS-isolated background process with total budgets, not 1 ms slicing'
status: accepted
date: 2026-11-07
task: '[[tasks/2026-11-07-q2-external-helper-budget-model]]'
supersedes_in_part:
- '[[decisions/d93377961365126-dream-q2-helper-creation-and-lifetime-budget]]'
- '[[decisions/d93377961365489-dream-q2-member-window-accounting]]'
- '[[decisions/d93377961366236-dream-q2-measured-quantum-and-wall-ceiling]]'
id: 01M6VB39H27W4PDTDA4VY2F9KH
updated: 2026-11-07T15:47:38.463Z
created: 2026-11-07T15:47:38.463Z
number: 93377961366280
created_by: seat:01M6VASNV9NAZRZ5JHQ7ER9PPX
updated_by: seat:01M6VASNV9NAZRZ5JHQ7ER9PPX
authorship:
  signer: seat:01M6VASNV9NAZRZ5JHQ7ER9PPX
  kind: seat
  public_key: gX8ozXnq5IspMTpqtE9rHRFSbgoDxsJh5dGOJgtj2o8=
  process_generation: '1'
provenance:
  digest: sha256:e511696c0d95ea8ce7d2772bb5eb58b0d49e4730c2f7783e5f04b7df79427523
  frontmatter_digest: sha256:eb08343a19d1d479d23eb1da54b490a708f571a0f5398c2b05636b46c1d3ebf8
  signer: seat:01M6VASNV9NAZRZ5JHQ7ER9PPX
  sig: PM0W7dFAEM5JlfU6sDWmyGMJbc6uhxVrZ/ic7Rz4Z7EHplWivMvmSNFaPQ90FGOscMIOuae44Zp06/yYVcuwCg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150375
  event:
    id: 01M6VB39Y6F8W8JJ6AK2PQ5C7Y
    sequence: 150375
    hash: 69e8cc449c5e9f5cdc264431c450bcb311e6e468ce2e6f6929387c974054d3d5
---
# Q2 external helpers: OS-isolated background process with total budgets, not 1 ms slicing

Date 2026-11-07. Task [[tasks/2026-11-07-q2-external-helper-budget-model]]. Choice **(b)**, with three additions: one-shot foreground pause, helper CPU paid as 9× job debt, and an observed-QoS gate. The 1 ms law of [[decisions/d93377961365086-dream-native-quantum-and-debt]] is **unchanged** and still covers all work on our own threads.

## What the 1 ms law protects

65086 limits each unit of work that runs **on a thread of ours** (UI, mux, reactor, control lanes, the proof owner) to ≤1 ms. Its 9× rest debt holds each job to ≤10% amortized CPU. A Git or GPG process runs on none of our threads. Its CPU competes with the UI only through the kernel scheduler, and macOS already ranks it below user-interactive work when it runs at background QoS. The rounds of 65126 → 65489 → 66236 → fixed T → adaptive grants measured whether a utility-QoS controller can preempt another process inside 1 ms. That is a run-queue property. It is not a UI property.

## Evidence

| Fact | Source |
|---|---|
| The controller spent more CPU supervising than the helper spent working: C 3,104,291 ns vs A 2,360,628 ns; W 3,849,000 ns | run 01M6VAGHD4Y6MXHSASDMN15XE9 `components/positive/output/result.json` metrics |
| The 126 µs window became t0→STOP 1,087,000 ns because the utility controller was preempted; STOP→ACK was 107 µs | task history item 4; ledger 0ee1e452… `grant_scheduler` line |
| A pause/stop delivered in about 1.1 ms easily meets the existing **100 ms** pause gate. The mechanism works at that scale, but not at 1 ms. | same |
| Background QoS is enforced and inherited: `taskpolicy -c background` gives the process PRI 4 and its child (ps) PRI 4; the default is PRI 31 | `ps -o pri`, Darwin 27.0.0, 2026-11-07 |
| This machine has 4 P-cores and 6 E-cores | `sysctl hw.perflevel0/1.logicalcpu` = 4 / 6 |
| wait4 of a reaped leader rolls up its reaped children (BSD ruadd of p_cru); on run 2 ΣRI matched wait4 within 1.3 µs | 65126 wait4 reference; 66236 evidence table |

Option (a), raising the controller's timing thread to user-interactive during each window, rejected: it spends the UI's scheduling class on background work. It still guarantees nothing, because timers coalesce and P-cores are shared with the UI's own threads. It keeps every mechanism that consumed the last five rounds: grant scheduler, per-member windows, roster closure per grant, conservation T and N_reaped. Under (a), C > A already showed that slicing more than doubles the helper's CPU cost.

## Exact law for external helper processes

1. **Birth.** The prepared posix_spawn (65086 `process_create` tag, 2 ms CPU / 25 ms wall, unchanged) sets `POSIX_SPAWN_SETPGROUP` and `posix_spawnattr_set_qos_class_np(QOS_CLASS_BACKGROUND)` atomically. START_SUSPENDED is permitted but no longer required by any budget. PID/start/PGID/effect are retained before any verdict (65086 effect law).
2. **No per-grant slicing.** The helper runs continuously until exit, kill or foreground pause. No CONT/STOP grants, member windows, per-grant Q, W ≤ 25 ms, roster closure per grant or adaptive grant sizing.
3. **QoS gate (hard).** Before the first pipe service, the controller observes each live member's base priority. It must equal the throttled band recorded by the capability probe for this OS (Darwin: 4). Git's GPG child inherits the clamp and is checked when it is observed. A mismatch kills the group → `resource_limit`.
4. **CPU caps (hard).** Per command: ≤ **250 ms** user+system, including reaped descendants. Per job, all helpers: ≤ **3 s**. The 3 s equals 10% of the 30 s deadline, consistent with the 65086 ≤10% allocation, and four jobs stay ≤40%. Live: the controller reads each known member's RI user+system in its ordinary pipe/control units, at least every 50 ms wall. If a read passes a cap, SIGKILL the group. Terminal: the leader's own wait4 (µs fields rounded up by 1 µs, subtree roll-up) is the verdict. wait4 > cap → `resource_limit` even when no live read saw the overrun. The excess is retained, never clamped.
5. **Debt.** After the leader is reaped, the job pays rest ≥ 9 × max(1 ms, wait4 CPU) before its next expensive span, whether helper or main work. Same no-banking/no-reset rules as 65086.
6. **Wall.** The existing 30 s job deadline only. On the deadline, kill and settle.
7. **Memory.** The 65126 limits are unchanged: helper-group sampled RSS ≤ 16 MiB and the mixed main+group ≤ 96 MiB admission. Add a terminal hard check: the wait4 `ru_maxrss` of each process the controller reaps ≤ 16 MiB.
8. **Foreground.** A pause, not slicing. One SIGSTOP to the group. A positive identity-bound stopped ACK must arrive inside the existing 100 ms pause gate. RI user time must not grow between the ACK and the resume. Resume with one SIGCONT after current-activity/authority admission. No CPU charge for the paused interval.
9. **Controller.** Stays at nice+10/utility; no elevation. Its pipe/sampling/settlement work is ordinary ≤1 ms units under 65086, with debt. The proof owner waits for the helper result and runs no concurrent expensive work for the same job. The commands are sequential anyway.
10. **Settlement.** The 65126/66050/66074 rules are unchanged: retire, kill, actual own reap, group-gone, or owned quarantine. The final group kill now also follows 66050 (ruling below).

## Clause disposition

| Decision | Stays | Changes |
|---|---|---|
| 65086 | All of it: ≤1 ms units on our threads, native tags, 9× debt, effect retention, gates | None. Scope clarified: an external process's CPU is not a unit on our thread. It is governed by the total-budget law above. |
| 65126 | Allowlist and argv/env/config, provisioning, IO caps, 256 reservations, ≤2 processes, memory shares, whole-tree accounting via wait4, effect/settlement/quarantine, error mapping, no reap-then-signal | §"No free attachment interval": initial ≤250 µs grants, stop-before-1 ms, n×wall charge and per-grant rest → replaced by items 2, 4, 5 and 8. START_SUSPENDED is no longer mandatory (item 1). |
| 65489 | Positive identity-bound stop ACK (now only for foreground pause); census only for settlement/group-gone | Superseded as a budget law for external helpers: W, A_active, C_active, Q and per-grant roster closure removed. |
| 66236 | A_measured definition (nonduplicated self plus reaped roll-up, µs round-up) as the definition of the terminal CPU figure | Superseded: basis, per-grant Q ≤ 1 ms, W ≤ 25 ms, conservation gate, Oracle clarifications 1/2, T = 150 µs and N_reaped are retired. Nothing per-grant is left to conserve, and wait4 is the verdict. |
| 65654 | Unchanged. Escaped or out-of-group CPU is still invisible to wait4, so the qualified tool basis is still required. | — |
| 66050, 66074 | Unchanged | 66050 now explicitly governs the final group kill too. |

## Ruling: the settlement_unknown in run 01M6VAGHD4Y6MXHSASDMN15XE9

**The signal failure was an expected exit race. Recording it as an unresolved terminal error is a probe bug.**

- `proof-helper-probe.c` (SHA e23f68d3…, the run's `probe.source_sha256`) :4383-4389: `known_exited` sampled the leader as not done. Then `kill(-pgid, SIGKILL)` returned an error other than 0 or ESRCH, and the probe recorded `settlement_unknown` at once. The errno is stored only in `owner->group_kill_result` (:2474, :4387) and is emitted nowhere.
- The same result records `settlement {reaped:1, group_gone:true, retired_coverage:true}`, and `wait4.json` shows an actual reap. The unknown was resolved but never retracted.
- Local witness on Darwin 27.0.0: a group whose only member is an unreaped zombie gives `kill(-pgid) = EPERM`; after the reap it gives ESRCH. 66050 already traced this to XNU `kern_sig.c`, whose group filter excludes zombies. The leader exited between the sample and the kill.
- Required fix, under any budget model: retain the raw errno. EPERM/ESRCH on the final kill enters 66050's pending-terminal state. Own wait4 plus group-gone resolves it to settled, with a retained `group_kill_raced_exit` observation. Only when that proof fails does it stay `settlement_unknown` with 66074 quarantine.
- Run 4 stays **FAILED** (`resource_limit`: closure refused, and an ordinary engine unit > 1 ms under 65086). Nothing is reclassified.

## Verification

Finite model `Q2HelperTotalBudget` (replaces the two-slot window model for external helpers). Yarn/native vs pinned Java TLC parity as usual.

Variables:
- helper ∈ {absent, running, paused, killed, exited, reaped, quarantined}
- qos ∈ {bg, other}
- cpu 0..4 (cap 2) and jobCpu 0..5 (cap 4), abstract units
- debt 0..36
- reservations 0..2
- foreground, killResult ∈ {none, ok, eperm}, groupGone, settled, unknownRecorded, trusted

Initially absent, zero counters, untrusted.

Actions: Spawn (atomic qos = bg), Burn (only while running), Sample/KillOverCap, Foreground/StopAck, Resume, Exit, FinalKill (eperm after Exit), Reap (wait4 = cpu; debt += 9·cpu), ObserveGroupGone, Settle, Deadline, Trust, AdmitNext.

Weak fairness covers controller/settlement actions only; helper progress is not assumed.

Invariants:
- `BackgroundOnly`
- `TrustWithinCap`: trusted ⇒ reaped ∧ wait4 ≤ cap ∧ jobCpu ≤ jobCap
- `NoBurnWhilePaused`
- `DebtBeforeNext`
- `SettlementHonest`: settled ⇒ reaped ∧ groupGone; a terminal unknownRecorded ⇒ ¬(reaped ∧ groupGone)

Liveness: Deadline ⇒ ◇(settled ∨ quarantined).

| Broken control | Must violate | Real unattended witness (`make test-dream-mcp`, private identities) |
|---|---|---|
| Spawn without the QoS attribute | BackgroundOnly | Observed PRI 31, not 4 → refused. Positive: Git and its GPG child observed at 4 |
| Trust from live samples only (no wait4) | TrustWithinCap | Scoped test-hook helper burns past 250 ms after its last sample → terminal wait4 refuses |
| Drop the reaped-child roll-up | TrustWithinCap | Git→GPG with a GPG burner just over the cap → refused; Git's wait4 ≥ GPG burn |
| Disable the live kill | (terminal gate still holds) | Over-cap helper → wait4 > cap retained, `resource_limit` |
| Skip helper debt | DebtBeforeNext | Next span admitted < 9× wait4 CPU after reap → fail |
| No STOP on foreground / resume during foreground | NoBurnWhilePaused | Foreground mid-command: ACK ≤ 100 ms, RI user unchanged until resume |
| Treat an EPERM final kill as settled without reap/group-gone | SettlementHonest | Hook keeps a live member while kill reports EPERM → `settlement_unknown` + 66074 quarantine |
| Leave settlement_unknown after reap + group-gone | SettlementHonest | Natural-exit race → settled, `group_kill_raced_exit` with errno, no unknown |

Validation (existing hard gates, unchanged): four jobs, each with a helper burning at its cap. The ≥240-frame p99 CPU ≤ 4 ms and input-to-GPU ≤ 16.7 ms gates, plus control p99 ≤ 100 ms / max ≤ 250 ms, must hold, with load averages recorded. Positive: the real provisioned pair completes a qualifying publication on a fresh source-bound tuple, and a corrupt proof fails. No historical run is reclassified.

## Cost and ownership

Removed from Q2: the grant scheduler, per-member windows, per-grant roster closure, adaptive sizing and the T/N_reaped conservation. Added: one spawn attribute, one priority read, RI cap reads in existing units, one wait4 comparison and one debt entry. The Q2 author implements this under an Oracle-amended brief: `src/dream/c1.rs` HelperGroup, `read.rs` reap/usage, `resolve.rs` spawn attributes, `tests/fixtures/dream-read/proof-helper-probe.c` and `tests/dream_read.rs`. The allocations (250 ms, 3 s, 50 ms sampling) must be revisited with real-helper distributions after qualification. Any change goes through a decision.
