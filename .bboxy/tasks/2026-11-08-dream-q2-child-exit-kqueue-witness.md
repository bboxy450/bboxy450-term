---
type: task
title: Q2 accept child kqueue NOTE_EXIT as terminal witness for leader-reaped verifier rollup, hand off INPUT63
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
- reports/2026-11-08-dream-child-exit-kqueue-witness.md
- src/dream/resolve.rs
verify: make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build
derived_from:
- '[[decisions/d93377961366280-dream-q2-external-helper-budget]]'
- '[[tasks/2026-11-08-dream-q2-verifier-event-discovery]]'
id: 01M6XFDPH50WD2KB535GJGQC0W
status: accepted
delegated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-08T13:50:37.556Z
created: 2026-11-08T11:41:42.747Z
number: 93377961366364
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
  - criterion: Exact required command actual0 and scope of amended final-head rerun
    reference: target/dream-q2-child-exit-witness-01M6RZZXSHZT3KHBK1QC7K3XB3/childexit-exact-four-chain-v1.json (b1df; log20cc0b626552249dd64b48c70121b51f0dda5719c9bf25ed088d7c4798f52dae); childexit-canonical-land-rerun-v2.json (390b; actual0; log03bf6612e5d6fdcfb9d0cfcc41954fe60ce9a55476ac0adcd8d55843e6774dbf); task amendment aea2366e65aed9979eefc8a892b8df3f6a2deadfb60622073abebafc15782b6a
  - criterion: Same-incarnation child knote terminal proof, raw receipt, later leader RI, no double count and full paid settlement
    reference: target/dream-q2-child-exit-witness-01M6RZZXSHZT3KHBK1QC7K3XB3/canonical/private-engine-controls-pos5dj1m/run.json SHAcd5bdd89f53658cfbbc38efc170a38aeee8124fefa07b0754ac66dd60fb33115; current-C-suite-summary-v1.json records actual T1<T2<T3/mask31/subtreeCPU9158000/debt82422000 and supplied API81 negatives
  - criterion: Changed finite contract and broken controls, genuine current full native/Java parity
    reference: verification/evidence/915b39a044e1f131b959fbfff07b4b60d08f3cb32a18e0162720c9b64d108e24; 442models/94contracts/3378027states/11366941labellededges; current-land-formal-all-raw-files-v2.json SHA851a4b5fd757541df8e88e5990d027b3d3d4d45ed27922ec5f3ebbfc0c0b8d17
  - criterion: ONE signed INPUT63, unchanged terminal fields, new ledger enum, all ten inputs/compiler/closure/audit
    reference: target/dream-q2-child-exit-witness-01M6RZZXSHZT3KHBK1QC7K3XB3/INPUT63-final-tuple-v2.json SHA0200af25c72d19765232582e6d84a32f4e52f47534a00d75ac624d0dda176de2; receipt888182901a213dfb915f1d9e14ba044da319c8aabf542e800c2830e4357e03c0; audit3282144dea91e8fc73daea427bcbb993cfbbfbcd1cad56807ec4522c956768c2; native handoff1459/id01M6XPNATDY3FRTQ9D7XA504AV
  - criterion: Signed landed source/remote/clean state, full committed report and honest nonpasses/limits
    reference: 'report KB commit0a05bb6a28a704d6aeea59af28b8149ac9a3dadc; final-source-remote-and-runtime-state-v2.json: HEAD=origin/main390b5836fb3693e3c4861a98def1ef8b53d54d5b, clean, own lsof empty; final-report-deferral-inventory-v2.json SHAe07491b822a58343572f790804d775cd3d92a2fb89ba00b33a6988aa2de2d040; old LandRED and b1df profile/archive retained separately'
report:
  summary: 'Landed signed 390b5836fb3693e3c4861a98def1ef8b53d54d5b on origin/main: retained process-bound child NOTE_EXIT covers RI ESRCH without invented exit_abstime or duplicate child billing; delayed receipt requires genuine later original-leader RI. Canonical C: 98 baselines/57 mutants/59 compiles/143 commands, actual0; real leader-reaped-child positive plus explicitly labelled API negatives. Literal required four-chain actual0 at implementation b1df2c4c; first Land2 retained, Oracle-amended one QoS attribute followed by ONE final-head canonical Land0 with fresh 442-model/94-contract native/Java parity. INPUT63 sent once with final signed 10-input readset and audit0 (325 headers/997 checks). Full committed report records exact cuts, 27 nonpass ignore attributes, preserved archives/profiles/failures; runtime approval and real qualification remain false/unproven.'
  commit: 390b5836fb3693e3c4861a98def1ef8b53d54d5b
  reported_by: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
  op_id: 01M6XPS9SVQQ17HF4BNRBZA6Y4
  verify_revision: 0
  report: '[[reports/2026-11-08-dream-child-exit-kqueue-witness.md]]'
provenance:
  digest: sha256:005834dbc6057321d313a5be6720435316cf6b47eb8d7058797ffbd7a66bf233
  frontmatter_digest: sha256:b12123d3255281229e7e8afb0774177e9bf5514e0c9b4a7fc9c38da8887b66e5
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  sig: Q2SRpN2x5IgX8m76w6r3HBLV+YS1JNu479pMAjAnSUqykHRKeHBbBsvNIaE31525r4UrCd5nRo5Nv45Ny8q6DA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150696
  event:
    id: 01M6XPSRJX8XP8867WGQBJR4JZ
    sequence: 150696
    hash: 017437def29d75700741707071462d3c27c340d78da710a2fb2f2c125508e752
---
ROLE: EXECUTOR

## Evidence (builder run 01M6XEW5VT8F8HF56S96KB592B on frozen INPUT62 7ab80fac; Oracle read the ledger and source)
- Ledger sha256 75984cf5...d607.
- Event discovery WORKED: on signature seq16, leader_fork_events=1, child_discovered_via=fork_event, fork_seen_child_missed=false. GS1 is bound to verifier pid 96572.
- The run then failed with coverage_unknown "retired child lacks qualified nonduplicated same-leader rollup" (proof-helper-probe.c:3329). child_rollup_mask is 27 = 1|2|8|16; bit 4 is missing.
- Bit 4 (~3323) requires terminal_known && m->exit_abstime && m->terminal_observed_ns && token gone. git reaps gpg, and after that reap no RI exit_abstime can be read from gpg, so bit 4 can never be set for a verifier reaped by its leader.
- The cost IS covered: bit 16 (the leader's RI child user/system rollup, read after the child) is present.
- Result: the controller sits in a live quarantine with child_cost_unproven stuck at 1, and cannot recover.

## Ruling (Oracle; a sound exit witness, no relaxed law)
1. Accept the child's own kqueue EVFILT_PROC NOTE_EXIT, from the watch attached after current-token capture, as the same-incarnation terminal observation for bit 4, in place of RI exit_abstime when RI is unavailable (ESRCH) because the leader reaped it.
   - On XNU an EVFILT_PROC knote is bound to the process itself, not to the PID number, so NOTE_EXIT on that knote cannot come from a reused PID.
   - Record terminal_observed_ns as the event receipt time, and the exit status if NOTE_EXITSTATUS gives it. exit_abstime stays 0, with a flag saying the exit was witnessed by the kqueue event.
2. Bit 4 then requires all of the following:
   - (RI exit_abstime OR a kqueue NOTE_EXIT on the retained child knote);
   - terminal_observed_ns != 0;
   - the token is gone.
   The other bits are unchanged, and engine_child_rollup_covers is unchanged, so the leader rollup must still cover the child's last sampled CPU. Bit 16 still requires the rollup read at or after terminal_observed_ns.
3. If neither an RI exit nor the knote's NOTE_EXIT is available, refuse exactly as now. Do not clear child_cost_unproven any other way.
4. Billing is unchanged: the leader's own wait4 (including the reaped subtree) is the terminal CPU verdict, and the child's cost is never added twice.
5. New ledger diagnostic, child_exit_witness: ri | kqueue_note_exit | none. Report its exact name and type. Keep the terminal fields unchanged unless you must change them; if you do, list them.

## Controls (causal)
- A scripted leader that forks a child, reaps it itself, then exits: with the knote NOTE_EXIT the run qualifies, child_exit_witness=kqueue_note_exit, and debt is paid.
- A mutant that ignores NOTE_EXIT reproduces the 81 "lacks qualified ... rollup" refusal.
- A child that exits with no NOTE_EXIT delivered (registration failed): still 81.
- The existing rollup-covers negatives (rollup < child sample) stay at 81.
- Update the finite model, with a broken control.

## Handoff
ONE INPUT63 tuple, in the same format as INPUT62. runtime_approved=false. No builder run.

## Verification
make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build
Report that exact string as verify_result.command; run it once on the final head. Land with make land, report in reports/, and task_report once. Keep only hashes and summaries of failed formal runs (disk).


## Oracle amendment (2026-11-08): one QoS ignore in resolve.rs
Added src/dream/resolve.rs to allowed_paths, ONLY for one attribute:
#[ignore = "load-sensitive budget/wall window; re-enable via tasks/2026-11-06-qos-builds-never-starve-mcp"]
on dream::resolve::prepared_qualification_tests::prepared_qualification_absent_and_forged_claims_refuse_before_native_birth.
- Witness: make land exited 2, log 24d10996...0c21. The stale-source arm (resolve.rs ~3961) got "selected ordinary batch refused" because a raw ordinary epoch used 1,032,458 ns of CPU against 1 ms under load. Helpers, reservations and effects were all 0.
- The body, assertions, the 1 ms quantum, 9x debt and all caps stay unchanged.
- Deferred tests go from 26 to 27; this one counts as deferred, never as passed.
- Keep the failed land log. Then rerun make land once.


## Report from seat:01M6RZZXSHZT3KHBK1QC7K3XB3

Landed signed 390b5836fb3693e3c4861a98def1ef8b53d54d5b on origin/main: retained process-bound child NOTE_EXIT covers RI ESRCH without invented exit_abstime or duplicate child billing; delayed receipt requires genuine later original-leader RI. Canonical C: 98 baselines/57 mutants/59 compiles/143 commands, actual0; real leader-reaped-child positive plus explicitly labelled API negatives. Literal required four-chain actual0 at implementation b1df2c4c; first Land2 retained, Oracle-amended one QoS attribute followed by ONE final-head canonical Land0 with fresh 442-model/94-contract native/Java parity. INPUT63 sent once with final signed 10-input readset and audit0 (325 headers/997 checks). Full committed report records exact cuts, 27 nonpass ignore attributes, preserved archives/profiles/failures; runtime approval and real qualification remain false/unproven.

```json
{
  "op_id": "01M6XPS9SVQQ17HF4BNRBZA6Y4",
  "commit": "390b5836fb3693e3c4861a98def1ef8b53d54d5b",
  "report": "[[reports/2026-11-08-dream-child-exit-kqueue-witness.md]]",
  "verify_result": {
    "command": "make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Exact required command actual0 and scope of amended final-head rerun",
        "reference": "target/dream-q2-child-exit-witness-01M6RZZXSHZT3KHBK1QC7K3XB3/childexit-exact-four-chain-v1.json (b1df; log20cc0b626552249dd64b48c70121b51f0dda5719c9bf25ed088d7c4798f52dae); childexit-canonical-land-rerun-v2.json (390b; actual0; log03bf6612e5d6fdcfb9d0cfcc41954fe60ce9a55476ac0adcd8d55843e6774dbf); task amendment aea2366e65aed9979eefc8a892b8df3f6a2deadfb60622073abebafc15782b6a"
      },
      {
        "criterion": "Same-incarnation child knote terminal proof, raw receipt, later leader RI, no double count and full paid settlement",
        "reference": "target/dream-q2-child-exit-witness-01M6RZZXSHZT3KHBK1QC7K3XB3/canonical/private-engine-controls-pos5dj1m/run.json SHAcd5bdd89f53658cfbbc38efc170a38aeee8124fefa07b0754ac66dd60fb33115; current-C-suite-summary-v1.json records actual T1<T2<T3/mask31/subtreeCPU9158000/debt82422000 and supplied API81 negatives"
      },
      {
        "criterion": "Changed finite contract and broken controls, genuine current full native/Java parity",
        "reference": "verification/evidence/915b39a044e1f131b959fbfff07b4b60d08f3cb32a18e0162720c9b64d108e24; 442models/94contracts/3378027states/11366941labellededges; current-land-formal-all-raw-files-v2.json SHA851a4b5fd757541df8e88e5990d027b3d3d4d45ed27922ec5f3ebbfc0c0b8d17"
      },
      {
        "criterion": "ONE signed INPUT63, unchanged terminal fields, new ledger enum, all ten inputs/compiler/closure/audit",
        "reference": "target/dream-q2-child-exit-witness-01M6RZZXSHZT3KHBK1QC7K3XB3/INPUT63-final-tuple-v2.json SHA0200af25c72d19765232582e6d84a32f4e52f47534a00d75ac624d0dda176de2; receipt888182901a213dfb915f1d9e14ba044da319c8aabf542e800c2830e4357e03c0; audit3282144dea91e8fc73daea427bcbb993cfbbfbcd1cad56807ec4522c956768c2; native handoff1459/id01M6XPNATDY3FRTQ9D7XA504AV"
      },
      {
        "criterion": "Signed landed source/remote/clean state, full committed report and honest nonpasses/limits",
        "reference": "report KB commit0a05bb6a28a704d6aeea59af28b8149ac9a3dadc; final-source-remote-and-runtime-state-v2.json: HEAD=origin/main390b5836fb3693e3c4861a98def1ef8b53d54d5b, clean, own lsof empty; final-report-deferral-inventory-v2.json SHAe07491b822a58343572f790804d775cd3d92a2fb89ba00b33a6988aa2de2d040; old LandRED and b1df profile/archive retained separately"
      }
    ]
  }
}
```
