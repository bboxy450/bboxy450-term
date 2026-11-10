---
type: decision
title: Charge Q2 grants by measured member CPU under proven closure; bound member-window wall separately
date: 2026-11-06
derived_from:
- '[[tasks/2026-11-07-q2-member-window-quantum-design]]'
- '[[decisions/d93377961365489-dream-q2-member-window-accounting]]'
- '[[decisions/d93377961365086-dream-native-quantum-and-debt]]'
id: 01M6TE8DVD9A29RTDXYG90DSD7
updated: 2026-11-07T07:23:38.443Z
created: 2026-11-07T07:23:38.443Z
number: 93377961366236
created_by: seat:01M6TE23PXGX5VN9SFP2H9NYDP
updated_by: seat:01M6TE23PXGX5VN9SFP2H9NYDP
authorship:
  signer: seat:01M6TE23PXGX5VN9SFP2H9NYDP
  kind: seat
  public_key: ekEPUX1K5XlMq/b2IQsO5MVAYyZdO/cYw7QmNPKD22s=
  process_generation: '1'
provenance:
  digest: sha256:b36c412d91b254ed0894b8d8df345e3ba1ad8a2697beedc3800ca6883a93f237
  frontmatter_digest: sha256:14c5811095d56ceb0c49a63c859680c3cdc87e4c3eaca833c60d73dd7bd45843
  signer: seat:01M6TE23PXGX5VN9SFP2H9NYDP
  sig: hF1QDPJlLmrLZOMOijYsn+ykHaMsEt35xTWqNfDZ27BvtICUDyhQ+6BzDZpJPGiKC5mMwMIpTnfNek6v9883CQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150215
  event:
    id: 01M6TE8ET63YCET17CD7SFBVW5
    sequence: 150215
    hash: 536ef49d0f985f766f5e392750ad29ffb9087a7b71dce3d39533965161787326
---
# Q2 grant quantum: measured CPU, separately bounded wall

Date 2026-11-06. Task [[tasks/2026-11-07-q2-member-window-quantum-design]]. Narrow correction to the charge formula of [[decisions/d93377961365489-dream-q2-member-window-accounting]] (and its restatement in [[decisions/d93377961366050-dream-failed-stop-independent-terminal-proof]]). The 1 ms cooperative law of [[decisions/d93377961365086-dream-native-quantum-and-debt]] is **unchanged**; what changes is which quantity is compared with it. No wire, schema, IR, version, tool-allowlist, memory, native-lane, foreground, deadline, debt-multiplier or settlement change. Window opening/closure rules of 65489 (pre-CONT t0, positive identity-bound ACK, unknown slots, roster closure, no backdating) stay exactly as written.

## Evidence

Run 2 ledger `…/runs/01M6T0EZ07D77BYZK7YNHP2A32/components/positive-qos-retry/output/ledger.ndjson` (SHA-256 `6b6ec65a0e628578f22be29f0d8dcb6ce57624b3c82a1b151cc13d163b90219c`, 32 lines, read 2026-11-06), 30 grants, one member (Git, `ack_mask=1`, `child=0`), timebase 125/3:

| Quantity | Observed |
|---|---|
| W per grant | 213,000 – 893,000 ns (ΣW 9,509,000) |
| A per grant (rusage-info tick delta) | 63,584 – 207,125 ns (ΣA 2,957,509) |
| C per grant | 216,083 – 547,792 ns |
| max over grants of A + C | 664,918 ns (grant 29) |
| A / W | ≤ 0.46 every grant |
| ΣA vs `ri_cpu_ticks` delta grant 1→30 | 2,957,509 vs 2,957,500 ns (9 ns rounding) |
| final `wait4` user+system vs final `ri` | 3,063,000 (µs-truncated) vs 3,064,333 ns |
| largest C stage | `roster`: CPU 224,750 / 299,458 / 398,417 ns in grants 28/29/30 (wall 225 / 300 / 1,548 µs) |

`result.json`: three resource_limit records, first "current member-window and controller lower bound exceeds one millisecond" (grant 30, `phase=reaped`, W=893,000 — the exit/reap window), last "ordinary engine unit exceeds one millisecond". Load averages 9.5 / 11.8 / 10.5 before and after (`load-qos-before.json`, `load-qos-after.json`).

Reading: W is wall time from pre-CONT to post-ACK. It is a CPU bound only because a member with one runnable thread cannot consume more CPU than its runnable wall. Under load W grows with scheduler queueing of *both* the controller and the helper, exactly when the helper is least able to run. The kernel's per-process CPU counters measure that CPU directly, and on this real run they reconcile with the independent post-reap `wait4` total. The proxy is ~3.2× the quantity it stands for.

## Alternatives

| | Option | Verdict |
|---|---|---|
| (a) | Keep Q = max(W,A)+C; shrink C | Rejected as sufficient. W alone reached 893 µs; any C leaves W ≤ ~1 ms − C, and W is set by the scheduler, not by our code. A law that fails with machine load measures the run queue, not Q2. (Shrinking C is still allocated below.) |
| (b) | Move STOP→ACK wall to a separate "wait" class, charge nothing for it | Rejected. During STOP→ACK the member is **not** proven stopped; it may run. Removing W without replacing it with a measurement deletes the only CPU bound for that interval. |
| (c) | Charge **measured** member CPU once closure is proven; keep W as a separately bounded wall quantity and as the fallback charge when coverage is not proven | **Chosen.** |

Third narrowing patch on this formula (65126 → 65489 → this). Its premise — "wall is our best CPU evidence" — was wrong once a closed, stopped roster makes the CPU counters final. This decision replaces the premise instead of narrowing again.

## Exact law

**Charge basis.** Each grant has `basis ∈ {measured, window}`.

`basis = measured` only when ALL hold at t_close (as defined by 65489): every reserved slot is positively ACK-stopped or exited with retained identity; the roster is closed with no unknown slot (`unknown_mask = 0`); every member's counter was read **after** its positive ACK (stopped) or from its retained `wait4` rusage (exited/reaped); counters are monotonic versus the grant's pre-CONT baseline read; timebase conversion is explicit; and each slot satisfies `A_slot ≤ W_slot` (one provisioned runnable thread). Otherwise `basis = window`.

**A_measured** = Σ over slots of (self user+system) delta + reaped-descendant roll-up delta (rusage-info child user/system fields, or the `wait4` usage of a member the controller reaped), converted from Mach ticks with the retained timebase, each `wait4` µs field rounded **up** by 1 µs. Nonduplicated: a descendant counted live is not counted again in a roll-up.

**CPU quantum (hard, 1 ms):**
- `measured`: `Q = A_measured + C_active ≤ 1 ms`.
- `window`: `Q = max(W, A) + C_active ≤ 1 ms` — the 65489 rule, unchanged.

**Wall class (hard, separate):** `W ≤ 25 ms` per grant, for either basis. This reuses the 65086 native wall allocation and stays inside the unchanged 100 ms pause/control gate. Breach is resource_limit; it never becomes CPU credit or debt relief.

**Debt:** after closure, rest ≥ 9 × max(1 ms, Q), as before. Banking, reset and overlap rules unchanged.

**Conservation (hard, terminal):** at actual reap, the direct member's `wait4` user+system (whole subtree, µs) must be ≤ startup CPU before grant 1 + Σ A charged over grants + Σ late-kernel L + Σ post-closure units + rounding allowance (2 µs per `wait4` reading + 1 tick per counter read). Any excess means some grant was under-charged: the job fails resource_limit retroactively, trust is refused, and the excess is retained as raw evidence. Run 2 satisfies this on its numbers; that is an observation, not a pass.

**Unchanged:** CPU of processes outside the group (for example agents/daemons) is covered by neither W nor A; 65654's source/build qualification must exclude those paths, as before. A > W per slot still invalidates coverage. Missing/reversed clocks or counters force `basis = window` and, where 65489 says so, unknown/quarantine.

## Allocation inside the 1 ms quantum

Advisory, reported per grant, not a gate: C_active ≤ 600 µs (observed max 548 µs), A ≤ 300 µs (observed max 207 µs), margin ≥ 100 µs. The `roster` closure stage is the largest C contributor (up to 398 µs CPU). Reducing it is the first lever if C approaches its share; any change must keep every predicate 65489 requires **before** t_close.

## Invariants affected

| Invariant | Before | After |
|---|---|---|
| Per-grant CPU ≤ 1 ms | Q = max(W,A)+C | basis-dependent Q above |
| Measured basis only under proof | — | `basis=measured ⇒ rosterClosed ∧ ∀slot acked/exited ∧ countersPostAck ∧ A_slot ≤ W_slot` |
| CPU conservation | reconciliation advisory ("samples/reap reconcile W") | hard terminal gate |
| Member-window wall | implicit in Q | W ≤ 25 ms hard |
| Debt ≥ 9×max(1 ms,Q), no banking | unchanged | unchanged |

## Verification and broken controls

Model (Executor updates the existing 65489 finite model; Yarn/native vs pinned Java TLC parity as usual): add `basis`, `countersRead ∈ {none, preAck, postAck}`, abstract `cpuBurned`, `cpuCharged`, `wall`. Invariants `MeasuredNeedsClosure`, `ChargedCoversBurned` (at reap: `cpuBurned ≤ cpuCharged`), `WallBounded`. Each broken control must violate its named invariant in the model and fail its real witness:

| Broken control | Must violate | Real unattended witness |
|---|---|---|
| Use measured basis with an unknown/open slot | MeasuredNeedsClosure | Git forks a short CPU-burning child after the initial census; grant must fall back to window basis or refuse |
| Read counters before positive ACK | MeasuredNeedsClosure / ChargedCoversBurned | CPU-burning member stopped late; pre-ACK read under-charges; conservation must catch it |
| Drop reaped-descendant roll-up | ChargedCoversBurned | GPG burns ≥ 2 ms and is reaped by Git inside a grant; terminal conservation must fail the job |
| Skip Mach timebase conversion | ChargedCoversBurned | Same real run; Σ A off by 125/3; conservation must fail |
| Remove W ≤ 25 ms gate | WallBounded | Scoped test hook delays ACK observation by 30 ms; CPU passes, wall must fail resource_limit |
| Accept A_slot > W_slot | coverage check | Fixture member with a second runnable thread; must refuse coverage |

Positive: fresh run on the frozen tuple under `make test-dream-mcp` with per-grant `basis`, A_measured, C, W, Q and the terminal conservation line retained, unattended, private identities.

## Not decided here

- Run 1 and run 2 stay exit 81. No historical sample is reclassified; a fresh source-bound run is required.
- Run 2's last resource_limit, "ordinary engine unit exceeds one millisecond", is an ordinary 1 ms unit failure outside this formula. It remains a failure until that unit is split or fixed.
- Run 2 debt totals (`debt_charged_ns` ≈ 18.6 s) were not examined for this decision.
- Implementation belongs to the Q2 author under an Oracle-amended brief: `src/dream/c1.rs` finish_grant/sample (basis, A_measured), `read.rs` reap/usage (conservation), `tests/fixtures/dream-read/proof-helper-probe.c` and `tests/dream_read.rs` (witnesses above).

## Oracle clarification (2026-11-07 ~08:05, on the spot; the Architect may revise): implicit child reaped by the leader
Question (Q2 executor): GPG is spawned and reaped by Git (the leader), so the controller never owns GPG's wait4. May the leader's RI child roll-up stand as the exited-slot counter source?
**Ruling: YES, under ALL of these conditions; otherwise the slot stays on window basis (max(W,A)+C):**
1. **Unique and admitted:** the slot is the ONLY admitted child of that leader in the window, and the roster proves no other child of the leader existed (no fork you did not admit, no unknown).
2. **Causal closure:** the child's exit is proven before the roll-up read (leader audit/RI shows the child count back to zero and the child identity retired; no live slot), and the roll-up is read AFTER that point and after the leader's positive ACK.
3. **Counted once:** the child's CPU enters A_measured exactly once, through the leader roll-up delta, and is never added again as a separate slot figure or a second time in the leader's own delta.
4. **Conservation:** at the leader's final wait4, that roll-up must reconcile with the terminal conservation gate. A mismatch makes the grant UNPROVEN (window basis) and fails conservation.
5. **Broken controls:** (a) a second unadmitted child mixed into the roll-up must fail; (b) a roll-up read before proven child exit must fail; (c) a double count must fail.
This certifies no RI-only exit without the leader's actual terminal wait4. The leader's own wait4 is still retained.

## Oracle clarification 2 (2026-11-07 ~11:30, on the spot; the Architect may revise): exit-teardown CPU vs terminal conservation
**Evidence (Q2, breakdown `…/builder-fix-c-post-ri-gap-breakdown.json`, SHA 1de4e4bc…):** for an owned helper, independent post-exit RI = 40,750 ns system (978 ticks), user 0; own wait4 system 93,000 ns (upper bound 95,000); allowance 2,667 ns; independent RHS 43,417 ns; excess 51,583 ns. The original run's excess was 85,291 ns. The excess is the kernel's process-exit teardown, billed to the process AFTER the last independently observable sample; only wait4 sees it. Q2 already charges it in full as cost (engine_wait4_late / engine_retire), but the strict gate (wait4 ≤ independent RHS + allowance) fails structurally for every real helper.
**Ruling:** add a FIXED per-reaped-process exit-teardown ceiling **T = 150 µs** (about 1.75× the largest observed residual) to the conservation RHS:
  `wait4(user+sys) ≤ independent RHS + allowance + N_reaped × T`
- T is a constant, never derived from wait4, so the gate stays independent: hidden post-sample work above T still FAILS conservation.
- The residual (wait4 − independent RHS) stays CHARGED in full as cost and debt (never free), and is recorded per process.
- Broken controls: (a) a helper that burns > T after its last sample must fail conservation; (b) an uncharged residual must fail; (c) T derived from wait4 instead of the constant must fail the model.
- Runs 1, 2 and 01M6TPSX… stay FAILED (not reclassified). Revisit T with real-helper distributions after qualification; any change goes through a decision.
**N_reaped definition (Oracle, ~11:45):** N_reaped = the number of the controller's own actual original wait4 reaps (here 1, the Git leader) + at most 1 for the single admitted Git-reaped child (GPG), counted ONLY when clarification-1 proof holds: a unique admitted child, its retired identity, and a zero-child post-ACK leader roll-up proving it was reaped. NEVER counted from expected slots, unknown births, census absence alone, or anything derived from the wait4 residual. If the child proof fails, N = direct reaps only (conservative). The same definition binds C, Rust and the model; add a broken control in which an unproven child increments N and must fail.
