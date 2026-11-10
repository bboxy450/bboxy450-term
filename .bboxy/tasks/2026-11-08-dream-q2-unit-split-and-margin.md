---
type: task
title: Q2 split the over-1ms helper sample unit and restore the 30 s margin by batching floors, hand off INPUT64
role: executor
worktree: /Users/feral/mydev/bboxy450-harness-dream-read-q2
allowed_paths:
- tests/fixtures/dream-read/proof-helper-probe.c
- tests/fixtures/dream-read/private-engine-controls.c
- tests/fixtures/dream-read/check-private-engine-controls.py
- tests/fixtures/dream-read/prepare-c-probe.py
- tests/fixtures/dream-read/check-c-preparation.py
- tests/formal/dream_read.rs
- tests/formal/dream_c1.rs
- tests/formal/models.rs
- verification/contracts.toml
- verification/corpus.toml
- verification/coverage.toml
- reports/2026-11-08-dream-unit-split-and-margin.md
- tests/dream_c1.rs
- tests/support/seat_socket.rs
- tests/seat_process.rs
- src/dream/read.rs
verify: make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build
derived_from:
- '[[decisions/d93377961365086-dream-native-quantum-and-debt]]'
- '[[tasks/2026-11-08-dream-q2-child-exit-kqueue-witness]]'
id: 01M6XQCS5XN0SEYDYHRMH4EAYN
status: accepted
delegated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-09T02:40:38.258Z
created: 2026-11-08T14:01:01.220Z
number: 93377961366374
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
updated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
authorship:
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  kind: seat
  public_key: DbWCjlMnSNUcBZMB2JrdfJ36/iQgSFRbtz78XE3WlhU=
  process_generation: '1'
assignee: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
verify_result:
  command: make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build
  exit_code: 0
  evidence:
  - criterion: Exact final signed-head verification and unchanged source
    reference: target/dream-q2-unit-split-margin-01M6RZZXSHZT3KHBK1QC7K3XB3/unitmargin-exact-four-chain-final-v8.json SHA256 f74b442026f537d8070ada3cb3c86996d00fd80f0a3dedcb64fa06557b401ef6; log315bbf7430a5066e352dd4c7224db87bd3f1d325fa324d77ee11e648f825da78 actual0
  - criterion: Unit composition, batching, same-wake custody, member peak, stopped cadence/SYSTEM/ACK bounds, and complete same-FD observer
    reference: target/dream-q2-unit-split-margin-01M6RZZXSHZT3KHBK1QC7K3XB3/canonical-C-input67-v5/run.json SHA256 d54f3d64de9e9d00c0773a61b12c76a098a61165deedcd814efee395a83c0818;113/72/74/174, explicit process/API scopes and intended81 assertions
  - criterion: Current full formal graph/verdict parity and immutable retention
    reference: verification/evidence/f5acd1e5688f0ff4161827e2bb7727dc3170d07513e5e1ce4586cb4f744e292e;442/94/1,3378027states/11366941labellededges/allgraph_equal; target/dream-q2-unit-split-margin-01M6RZZXSHZT3KHBK1QC7K3XB3/current-formal-all-raw-files-input67-v5.json SHA256 f59d825f9e191b53298518750951aefbaba3fc8fabd59e6a43b4f61d1c14b201
  - criterion: Canonical landing, signed commits, clean worktree and remote containment
    reference: target/dream-q2-unit-split-margin-01M6RZZXSHZT3KHBK1QC7K3XB3/unitmargin-canonical-make-land-input67-v4.json SHA256 bc4dc3e2cd14affc11b78409fc42bc5b37f9c39a2615fe280c68de1fbd379c83 actual0; landed-remote-and-signature-verification-input67-v4.json SHA25629f2351f9008ff15f752f108ab70460b009791cd85170b751d6a60a71693bdd1 containsHEAD0
  - criterion: ONE post-land INPUT67 immutable source/executable/compiler/header/10-input handoff, exact new internal fields
    reference: target/dream-q2-unit-split-margin-01M6RZZXSHZT3KHBK1QC7K3XB3/INPUT67-source-freeze-tuple-v2.json SHA25648c72b0865936fac5ec5fc43f890db3c4e97e3946b24e63e351cade4249030b4; receipt d6a854c8fd0e4f1784ddd56d72da9bf4f645bce126b66e417aa1ede920efac14; audit03224241f45e209717e782ae8e92c66756c162d4644cfac9fb8ed369fd78f956 actual0; native1557/01M6Z1XS14Q37RAC5JZKAYJ0KB
  - criterion: Accepted conditional margin and honest remaining qualification limits
    reference: target/dream-q2-unit-split-margin-01M6RZZXSHZT3KHBK1QC7K3XB3/unit-split-floor-projection-v2.json SHA256 fda1c98d16d2ca7b90662aa68bb1548efed8e79f1e2b728791b774285e84910f; Taska8e accepts conditional formula, no fabricatedG/fullcandidate/margin; report details all28nonpasses
  - criterion: Priority Rust metric repair landed separately with both-order/oldskip evidence
    reference: bc4f7fadabb3c7c73a67bba90c3abc2037e6b16b; default4/4 actual0 log81f1bdfcbe925f2472a8b13f92988dc176a1aa701e9a0ca74a66a79bee96f0be; oldskip101 log9f8e8f989b37f6e47fa02e5402f272fa66bb083e24dcab2c337b889e2252c0fe; original named failure UNESTABLISHED
  - criterion: Owned build cleanup after landing; immutable inputs and current graphs preserved; reserve retained
    reference: target/dream-q2-unit-split-margin-01M6RZZXSHZT3KHBK1QC7K3XB3/owned-completed-build-output-cleanup-result-input67-v4.json SHA2561ea31e6b976c86ebe48ef75f5ad05c96e9fb7d11973b46558bcf7f3299bc1131;actual0,23833journaled entries,5086kept stamps unchanged,free22536441856B; committed report090c71fa68da90d12d850649c3c8910a72fa75c8
report:
  summary: 'Landed signed 1f1b30ddec05e5d8e092d91b92c161bd94d1cc38 on origin/main with the exact prescribed four-command chain and canonical make land both actual0, clean source, current 442-model/94-contract full native-Java parity and preserved profiles/observers/graphs. Complete report #93377961366422 is committed at native knowledge checkpoint090c71fa68da90d12d850649c3c8910a72fa75c8; sole post-land INPUT67 tuple48c72b08…9030b4 was handed off as1557, independent preparation audit0, owned completed-build cleanup0 leaves22,536,441,856bytes free. Current C113baselines/72intended mutants/74strict compiles/174commands passes; priority pipe-metric fixbc4f7fad was landed separately. All historical failures and28ignored attributes remain nonpasses; supplied API versus actual-process evidence is explicit, runtime_approved=false and real qualification/current private30s margin remain unproven.'
  commit: 1f1b30ddec05e5d8e092d91b92c161bd94d1cc38
  reported_by: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
  op_id: 01M6Z2V6WB5M706BAX4DE4C1QW
  verify_revision: 0
  report: '[[reports/2026-11-08-dream-unit-split-and-margin.md]]'
provenance:
  digest: sha256:de85e3adcb68718dfb4ae37d4f07d0393622335d3550d0f4cd1aa0d84aece5c2
  frontmatter_digest: sha256:b6d414f1181b13edcd53f3a5aba831624753a01ae79f64aba894faae7c4a8355
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  sig: y3fXYHAfKDaN6y1B5OCGzoRp9LTc1kcFgbldLJiOmVJXmhCedKSfAXRihGT/m5Nya0LVLaARZPrzLVFrFMpbDQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150896
  event:
    id: 01M6Z2VPGM17X26XNJDDBDBQN5
    sequence: 150896
    hash: fa58a0da4ff400ff4289a6306cb64d44279f744495159818192ab76af8d9cfac
---
ROLE: EXECUTOR

## Evidence (builder run 01M6XPVSR73HYNHFAB3ECZSZX8 on frozen INPUT63 390b5836)
- POSITIVE PASSED: actual wait4 exit 0, proof valid, 16/16 settled, final terminal passed with 0 errors.
  - The signature verifier was retained via fork_event; child_exit_witness=kqueue_note_exit; mask 31; GS1->GS2 used the same inode and hash.
  - Final paid debt 24.863725881 s = helper 1.376 s + ordinary raw 4.634 s + floors 18.854 s.
  - Reporting finished at 29.144 s of the 29.835 s admitted interval, leaving ONLY 690 ms of margin.
- corrupt_proof (same invocation) FAILED: resource_limit at engine_checkpoint (proof-helper-probe.c ~2683). It charges thread CPU from private_cost.open_cpu to now; the actual unit was 1,020,958 ns against the 1 ms quantum. Ledger components/corrupt_proof/output/ledger.ndjson sha256 131ccb59...6feb, row 87. Child witness none, mask 0, unproven 1, quarantined.
- Load average was about 3-4 on 10 cores. The 1 ms quantum is LAW and stays.

## Required (code fixes; no law change)
1. Find the work in the unit that ended at that checkpoint (ledger row 87: which phase or sample). Split it so that every ordinary unit on the helper sample/census/kqueue path has a source-bounded worst case comfortably under 1 ms, at most about 500 µs of expected CPU.
   - Put kqueue servicing, census, RI reads and child-watch registration in separate units if needed, each closed and paid.
   - Report the before/after unit composition.
2. Restore the 30 s margin. Ordinary floors are 18.85 s of the 24.86 s total; a positive must leave at least 3 s of margin on a loaded machine. Reduce the floor count by batching more cheap fixed-bound checks into single ordinary units under the existing C14 pattern (each batch below 400 µs / 64 steps, full 9x rest paid, no check removed).
   - Give a source-derived projection of the new positive and corrupt_proof totals.
   - Neither change may move the 1 ms quantum, 9x debt, caps, 30 s, 66331 or any witness.
3. Controls: a mutant restoring the old combined unit must exceed the bound in the unit-composition test; a batching mutant that drops a check must fail. The existing positives and negatives stay green. Update the models if they represent unit composition.

## Handoff
ONE INPUT64 tuple, in the same format as INPUT63. Report any ledger or terminal field changes, with exact names and types. runtime_approved=false. No builder run.

## Verification
make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build
Report that exact string as verify_result.command; run it once on the final head. Land with make land, report in reports/, and task_report once.
Disk: reuse ONE build tree (for example target/harness-landing); do not create a new per-task canonical tree, and remove your build output after landing. Free space must stay above 8 GiB for the builder.


## Oracle amendment (2026-11-08): margin evidence disposition
The actual deduplicated G and the 33-pass path/batch/poll counts are not available from the failed ledger 131ccb59... The >=3 s margin requirement in item 2 is therefore satisfied by the clearly labelled SOURCE-CONDITIONAL projection E/unit-split-floor-projection-v2.json (sha256 fda1c98d...910f): N = 33(G+7) - sum(B) - K, with the examples G8 -> 4.515 s and G2 -> 2.733 s, NOT certified.
The actual loaded margin and the completed corrupt_proof total are left to the authorized builder's single run on INPUT64. If that run shows less than 3 s of margin, or a corrupt_proof overrun, that is the next defect; it is not a reason to reopen this task.
No fabricated counts.


## Oracle amendment (2026-11-08): INPUT64 regression, verifier image admission lost
Builder run 01M6XWN035QSQRFAB367J0AZG0 on frozen INPUT64 1ec942b2, ledger sha256 8fabd6b5...51d9. Signature seq16 leader 90581:
- fork_events=1, via fork_event, missed=false. Child 90584 was created, but image_admitted=0 and there was NO GS1.
- child_exit_witness=kqueue_note_exit, rollup_mask=30 (bit 1 missing), so the run failed with coverage_unknown "retired child lacks qualified nonduplicated same-leader rollup". The controller is held in quarantine.
- INPUT63 (combined census+GS1) admitted this same verifier. INPUT64's separate queue/census->member and member->GS1 phases most likely let gpg exec and exit before the GS1/image admission unit runs.
Required, in the same task: restore same-wake verifier admission. After the fork-event census, attach the child knote and run image admission + GS1 in the NEXT unit with no intervening wait or sleep, or keep them in one unit if its source-bounded worst case is at most 500 us. Each unit still stays under 1 ms.
Add a control: a scripted child that execs and exits within one census->GS1 gap must be admitted (positive); a mutant that inserts a wait between the units must reproduce bit 1 missing.
Report the new unit composition. The freeze of 1ec942b2 is VOID; send ONE replacement tuple, INPUT65, after the fix.
Debt at that point of the positive was 12.65 s, about half of INPUT63's 24.86 s at the same stage, and ordinary_max_ns was 660,041. The batching works.


## Oracle amendment (2026-11-08): QoS ignore in tests/dream_c1.rs; same-wake admission is option A
1. Added tests/dream_c1.rs to allowed_paths, ONLY for the existing QoS reason attribute
   #[ignore = "load-sensitive budget/wall window; re-enable via tasks/2026-11-06-qos-builds-never-starve-mcp"]
   on real_worker_reuses_failure_retry_and_denial_fingerprints, exactly as in C1-one-QoS-attribute-PROPOSAL-NOT-APPLIED-v1.patch.
   - Witness: exact chain on 1ec942b2 exited 2, log 02a3e314...9354. One complete warm_normalize_engine Validate unit took 1,027,125 ns at c1.rs:4165; one charge; debt paid.
   - The body, assertions and caps stay unchanged. Ignored tests go from 27 to 28, all non-passes.
2. Same-wake verifier admission: OPTION A. A single bounded no-sleep ordinary span runs from discovery -> child knote -> EXEC/image admission -> the EXISTING custody STOP with a positive ACK. Then the full 9x rest is paid while the child is positively stopped, then GS1.
   - "No intervening wait" in the earlier amendment applies only to the window before the child is positively stopped.
   - If EXEC/ACK cannot fit the 1 ms quantum (nominal 500 us), refuse and retain exactly as now.
   - No new STOP, no deferral of closed-unit debt, no cap change.
   - Controls: the positive (a child that would exec and exit quickly is admitted because it is stopped before the rest); a mutant that rests before the STOP/ACK reproduces bit 1 missing.


## Oracle amendment (2026-11-08): a bounded kevent wait inside the open span is permitted
Option A's "no intervening wait" forbids paid 9x rest, usleep and any other blind time-passing before the positive STOP/ACK. It does NOT forbid a bounded native kevent wait on the retained child knote (NOTE_EXEC/NOTE_EXIT) inside the still-open ordinary span, because that wait observes the exact event we are waiting for.
Proposal C33 (E/C33-bounded-event-wait-PROPOSAL-NOT-APPLIED.json sha256 f3ece2e0...e5d, patch 6b4f65eb...8c49) is APPROVED, with these conditions:
- Each engine_events_poll wait is at most 1 ms, and at most the original deadline and the remaining RI 50 ms headroom. Overshoot refuses, with no clock reset.
- The span keeps the 400 us CPU precheck, at most 64 finite iterations, and the hard 1 ms CPU quantum (blocked time is not CPU; all CPU is still charged).
- NOTE_EXIT before EXEC/STOP/ACK refuses and retains, with the distinct detail "verifier exited before admission".
- The full 9x rest happens only after the positive STOP/ACK, and the existing STOP is unchanged.
- No usleep, no deferral of closed-unit debt, no cap change.
- Controls: a slow-exec child (CLONE for at least 750 us) is admitted through the event wait; a mutant that does a non-blocking poll only reproduces the 400 us refusal; an exit-before-exec child refuses with the distinct detail.


## Oracle amendment (2026-11-08): INPUT65 run, members_max bookkeeping on same-wake admission
Builder run 01M6Y3GQNVZDMH82PY9DNMKAT9 on frozen INPUT65 28c91dff, ledger sha256 db7e2fbb...ab20.
- Everything substantive held: proof valid, 16/16 settled, group gone, retired coverage true. The signature verifier was found via fork_event, image_admitted=1, child_exit_witness=kqueue_note_exit, mask 31, GS1->GS2 used the same inode and hash.
- Debt 11.890 s (helper 1.414 + raw 1.526 + floors 8.950). Margin 16.17 s. Max ordinary unit 749,833 ns. The positive still exited 81.
- Cause (verified in source): result->members_max is written only by the census (proof-helper-probe.c:3240) and by source admission (3524, =1). Same-wake admission of members[1] bypasses the census, so members_max stays 1, and the final-case conjunction (4117, members_max!=2) fails as coverage_unknown.
Required:
- Wherever a retained member slot is actually admitted (including the same-wake path), update members_max from the ACTUAL count of retained live members at that moment. Do not hard-code 2. Keep the census update.
- Serialize the individual final-case conjunction terms, which are now opaque, as diagnostics in the ledger: one bool per term plus the first failing term name. Then the next failure names itself. Report exact names and types.
- Controls: the same-wake positive reaches members_max=2 and passes the conjunction; a mutant that drops the admission-path update reproduces the coverage_unknown; a census-only path is unchanged.
Then send ONE INPUT66 tuple; the INPUT65 freeze is superseded. The corrupt_proof case still needs its first real run.


## Oracle amendment (2026-11-08): INPUT66 passed positive + corrupt_proof; pause_resume 50 ms gap during a positive stop
Builder run 01M6Y8VDN0ZJ8XMYC5D04V0FGJ on frozen INPUT66 7449a048:
- positive: wait4 exit 0, stage settled, control passed, 0 errors. final_case_conjunction: all 28 terms true, first_failing_term none. Debt 11.43 s, margin 16.59 s, max ordinary unit 520 us.
- corrupt_proof: exit 0, settled, expected rejection, all 28 terms true. Debt 11.22 s, margin 16.99 s.
- pause_resume: exit 81, resource_limit "RI start age 103.627 ms > 50 ms". Ledger sha256 37f55723...890a. Line 2 has max_sample_gap_ns=103,627,000. Line 3 shows the pause was ACKed, the user time was stable (ack_user_raw == observed_user_raw), user_excess_ns 0 and pause_user_pass 1.

Oracle interpretation of 66280 item 4: the live RI sampling interval (<=50 ms) exists so a running helper cannot overrun its CPU cap unseen.
- A member that is POSITIVELY STOPPED (stop ACK observed, user time stable) cannot accrue CPU, so its stopped interval is excluded from the 50 ms gap measure.
- Conditions, all required:
  (a) an RI sample at the stop ACK, and a fresh RI sample before or at resume (the gap is measured separately on each running side);
  (b) user and system time unchanged across the stopped interval (the existing pause_user check, plus system time);
  (c) each running-side gap is still <=50 ms;
  (d) the interval counts as stopped only from the observed ACK to the resume signal, never from a merely sent signal.
- First confirm from this run's timestamps that the 103.6 ms gap lies within, or straddles, the ACKed stop. If running-side time alone exceeded 50 ms, that is a genuine scheduling defect: fix the sampling instead, and do not apply this exclusion.
- Serialize stopped_interval_ns and running_gap_max_ns (u64) in the existing helper_total_budget row; report the names. Controls: a stop longer than 50 ms with stable user time passes; a running gap over 50 ms still refuses; a fake stop (no ACK) still refuses.
Then send ONE INPUT67 tuple. The cases after pause_resume still need their first real runs.


## Oracle amendment (2026-11-08): unrelated GUI profile-switch failure in the verify chain
The exact four-chain on fd28caeb exited 2 (log 8fc119e4...3b66). The cause is the tests/support/seat_socket.rs:607 GUI profile switch: it timed out at old_session_closed_wait 2 s because no SessionClosed arrived. Q2's diff (C probe, fixtures) does not touch this path.
Classify it ONCE, as a declared diagnostic rather than a gate retry:
1. Run ONLY that failing test, once, on an unchanged checkout of harness main f848c03f, using a separate temporary worktree that you delete afterwards.
   - If it FAILS on f848c03f, it is pre-existing. Add `#[ignore = "KNOWN-DEFECT: fails on base f848c03f; fix via tasks/2026-11-08-gui-profile-switch-session-closed"]` to that one test (tests/support/seat_socket.rs, tests/seat_process.rs or tests/dream_c1.rs are now allowed ONLY for that attribute). Count it as a deferral, keep the 2 s and the assertions unchanged, then rerun the exact four-chain once on the final head.
   - If it PASSES on f848c03f, run it once on fd28caeb. If it passes there too, record it as an unestablished transient with both logs, then rerun the exact four-chain once. If it fails only on fd28caeb, STOP and report: it is then attributable to this cut.
2. Either way the Oracle files tasks/2026-11-08-gui-profile-switch-session-closed for the lifecycle owner. Do not edit src/frontends/gui/bridge.rs here.


## Oracle amendment (2026-11-08): USER-only mutants become API controls; bounded SYSTEM accrual during a positive stop
1. Approved: the two existing USER-only terminal/live causal mutants become explicitly labelled PRODUCTION-API controls with supplied immutable ACK and USER growth (SYSTEM fixed). They call the real live/terminal guards and assert raw growth, sticky refusal and full known cost. Label them native_process=false and never call them physical. The real physical modes 9/10 baseline, the raw SYSTEM refusal and every other assertion stay.
2. CORRECTION to the earlier "user and system time unchanged across the stopped interval" condition. C39's real witness: original 86873 in SSTOP with runnable 0 kept USER 35356->35356 but SYSTEM went 42377->42483 (+106 raw ticks, about 4.4 us). That is kernel bookkeeping for the stop itself. Exact SYSTEM equality would refuse a genuine pause.
   New rule for a positively stopped interval:
   - USER must be EXACTLY equal;
   - SYSTEM growth must be at most 50 us (in ns, converted from raw ticks with the retained timebase) and is FULLY charged. It is already inside the leader's wait4 terminal CPU; record it as stopped_system_ns (u64) and never subtract it;
   - SYSTEM growth over 50 us, or ANY USER growth, refuses as now.
   Controls:
   - API: USER +1 tick refuses; SYSTEM +50 us passes; SYSTEM +50 us + 1 ns refuses.
   - Physical: the existing real stop with about 4.4 us of SYSTEM passes.
   This is the only tolerance. The caps, the 50 ms running gaps, 9x and settlement are unchanged.
Then run one full private C, the final signed exact four-chain and canonical land, and send INPUT67 with stopped_system_ns listed among the new ledger fields.


## Oracle amendment (2026-11-08): FIRST PRIORITY, a harness main lib test fails on every run
The operator (term main 0d507f51, signed Maksim) pinned the term to divergent harness 520823e6 instead of harness main because harness main fails dream::read::helper_tests::retirement_application_rest_starts_after_actual_span_despite_saved_idle (src/dream/read.rs:3323) "alone on every run". This blocks every Dream consumer (Q3) from pinning harness main.
- Added src/dream/read.rs to allowed_paths for this fix ONLY.
- Reproduce it on current harness main (6409ebc3) with exactly `cargo test --lib dream::read::helper_tests::retirement_application_rest_starts_after_actual_span_despite_saved_idle`, alone. Find the cause: test expectation vs code, introduced by the unit-split/batching or same-wake work. Fix it without weakening the law: the rest must still start after the actual span, and saved idle grants no credit.
- Land that fix ON ITS OWN, before continuing INPUT67, with its own signed commit and make land. Then report the harness main SHA that contains the fix, 6409ebc3 and the INPUT65/66 lineage, in one short message.


## Report from seat:01M6RZZXSHZT3KHBK1QC7K3XB3

Landed signed 1f1b30ddec05e5d8e092d91b92c161bd94d1cc38 on origin/main with the exact prescribed four-command chain and canonical make land both actual0, clean source, current 442-model/94-contract full native-Java parity and preserved profiles/observers/graphs. Complete report #93377961366422 is committed at native knowledge checkpoint090c71fa68da90d12d850649c3c8910a72fa75c8; sole post-land INPUT67 tuple48c72b08…9030b4 was handed off as1557, independent preparation audit0, owned completed-build cleanup0 leaves22,536,441,856bytes free. Current C113baselines/72intended mutants/74strict compiles/174commands passes; priority pipe-metric fixbc4f7fad was landed separately. All historical failures and28ignored attributes remain nonpasses; supplied API versus actual-process evidence is explicit, runtime_approved=false and real qualification/current private30s margin remain unproven.

```json
{
  "op_id": "01M6Z2V6WB5M706BAX4DE4C1QW",
  "commit": "1f1b30ddec05e5d8e092d91b92c161bd94d1cc38",
  "report": "[[reports/2026-11-08-dream-unit-split-and-margin.md]]",
  "verify_result": {
    "command": "make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Exact final signed-head verification and unchanged source",
        "reference": "target/dream-q2-unit-split-margin-01M6RZZXSHZT3KHBK1QC7K3XB3/unitmargin-exact-four-chain-final-v8.json SHA256 f74b442026f537d8070ada3cb3c86996d00fd80f0a3dedcb64fa06557b401ef6; log315bbf7430a5066e352dd4c7224db87bd3f1d325fa324d77ee11e648f825da78 actual0"
      },
      {
        "criterion": "Unit composition, batching, same-wake custody, member peak, stopped cadence/SYSTEM/ACK bounds, and complete same-FD observer",
        "reference": "target/dream-q2-unit-split-margin-01M6RZZXSHZT3KHBK1QC7K3XB3/canonical-C-input67-v5/run.json SHA256 d54f3d64de9e9d00c0773a61b12c76a098a61165deedcd814efee395a83c0818;113/72/74/174, explicit process/API scopes and intended81 assertions"
      },
      {
        "criterion": "Current full formal graph/verdict parity and immutable retention",
        "reference": "verification/evidence/f5acd1e5688f0ff4161827e2bb7727dc3170d07513e5e1ce4586cb4f744e292e;442/94/1,3378027states/11366941labellededges/allgraph_equal; target/dream-q2-unit-split-margin-01M6RZZXSHZT3KHBK1QC7K3XB3/current-formal-all-raw-files-input67-v5.json SHA256 f59d825f9e191b53298518750951aefbaba3fc8fabd59e6a43b4f61d1c14b201"
      },
      {
        "criterion": "Canonical landing, signed commits, clean worktree and remote containment",
        "reference": "target/dream-q2-unit-split-margin-01M6RZZXSHZT3KHBK1QC7K3XB3/unitmargin-canonical-make-land-input67-v4.json SHA256 bc4dc3e2cd14affc11b78409fc42bc5b37f9c39a2615fe280c68de1fbd379c83 actual0; landed-remote-and-signature-verification-input67-v4.json SHA25629f2351f9008ff15f752f108ab70460b009791cd85170b751d6a60a71693bdd1 containsHEAD0"
      },
      {
        "criterion": "ONE post-land INPUT67 immutable source/executable/compiler/header/10-input handoff, exact new internal fields",
        "reference": "target/dream-q2-unit-split-margin-01M6RZZXSHZT3KHBK1QC7K3XB3/INPUT67-source-freeze-tuple-v2.json SHA25648c72b0865936fac5ec5fc43f890db3c4e97e3946b24e63e351cade4249030b4; receipt d6a854c8fd0e4f1784ddd56d72da9bf4f645bce126b66e417aa1ede920efac14; audit03224241f45e209717e782ae8e92c66756c162d4644cfac9fb8ed369fd78f956 actual0; native1557/01M6Z1XS14Q37RAC5JZKAYJ0KB"
      },
      {
        "criterion": "Accepted conditional margin and honest remaining qualification limits",
        "reference": "target/dream-q2-unit-split-margin-01M6RZZXSHZT3KHBK1QC7K3XB3/unit-split-floor-projection-v2.json SHA256 fda1c98d16d2ca7b90662aa68bb1548efed8e79f1e2b728791b774285e84910f; Taska8e accepts conditional formula, no fabricatedG/fullcandidate/margin; report details all28nonpasses"
      },
      {
        "criterion": "Priority Rust metric repair landed separately with both-order/oldskip evidence",
        "reference": "bc4f7fadabb3c7c73a67bba90c3abc2037e6b16b; default4/4 actual0 log81f1bdfcbe925f2472a8b13f92988dc176a1aa701e9a0ca74a66a79bee96f0be; oldskip101 log9f8e8f989b37f6e47fa02e5402f272fa66bb083e24dcab2c337b889e2252c0fe; original named failure UNESTABLISHED"
      },
      {
        "criterion": "Owned build cleanup after landing; immutable inputs and current graphs preserved; reserve retained",
        "reference": "target/dream-q2-unit-split-margin-01M6RZZXSHZT3KHBK1QC7K3XB3/owned-completed-build-output-cleanup-result-input67-v4.json SHA2561ea31e6b976c86ebe48ef75f5ad05c96e9fb7d11973b46558bcf7f3299bc1131;actual0,23833journaled entries,5086kept stamps unchanged,free22536441856B; committed report090c71fa68da90d12d850649c3c8910a72fa75c8"
      }
    ]
  }
}
```
