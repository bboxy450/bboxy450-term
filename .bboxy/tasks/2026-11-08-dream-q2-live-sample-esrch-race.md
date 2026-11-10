---
type: task
title: Q2 fix live-sample ESRCH race (zombie between terminal check and TASKALLINFO) and hand off INPUT61
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
- reports/2026-11-08-dream-live-sample-esrch-race.md
verify: make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build
derived_from:
- '[[decisions/d93377961366280-dream-q2-external-helper-budget]]'
- '[[tasks/2026-11-08-dream-q2-process-create-wall-ceiling]]'
id: 01M6X2S2JVX45W5RX5PEYVJRJ8
status: accepted
delegated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-08T09:43:31.791Z
created: 2026-11-08T08:00:44.061Z
number: 93377961366347
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
  - criterion: Both live-sample read races, exact CURRENT original custody and real terminal proof; errno preservation; strict negative classes
    reference: reports/2026-11-08-dream-live-sample-esrch-race.md; source9bacbb0483ab328798af2bc98225bdf8f50f930e6d2ecda6779cec3afec75042; canonical C receipt bf043efc833c01a5dd38b050da537a74fbf056d65ddd348ca005c64dd13c3d2a,79/48/50/110 actual0
  - criterion: Changed finite model and missing-terminal-recheck broken control; fresh full native/reference parity
    reference: formal freeze63e715d1d6c0fd343a90183f602af1f9785b3f2ba32f7efe8dc63d5501c141f0; archive6e455ce97ad6292c1310d16ef3fa115c8f8a5735817ac00cb30a8e66d9e542e9,434models93contracts,3374874states11363095labellededges; rawmanifestba3cab1de50492ec349b37ae8a968ee6abb7797f993638d46fd7356d66cf6619
  - criterion: ONE required exact verification string ONCE on final signed head, actual0
    reference: target/dream-q2-live-sample-esrch-01M6RZZXSHZT3KHBK1QC7K3XB3/esrch-exact-four-chain-v1.json; logSHA2e07a0bbb9f6166b96be871f807596bed66948357edff62d1621e2a2f54be104
  - criterion: Canonical make land0; main contains signed HEAD and worktree clean
    reference: esrch-canonical-land-v1.log SHA1a151ba0c9914ff242f0913b8ddea90e0655c3dde4c48413ea5233ce2e53707b; remote-verification-detail-v1.json and report
  - criterion: ONE INPUT61 signed preparation/10inputs/compiler/audit; exact unchanged field names/types; no builder
    reference: stored1434/id01M6X8FZM6B3JKH8QVA0JK2N16; tupleff1c31cf724ecae33c874cd65ad25e869f3f6843a0d8eb4ca5814900d8689106; receipteb5f1696cba83e117d354c040b50f0950d5f0fd7847f398255d0a699f73dd987; audit533b16290b9296355c7d2017631eb4fe4a2046263b77dfdd54a20d67e2243790,997checks325headers0
  - criterion: Full committed new report; prior accepted records/failures preserved; limits and deferrals honest
    reference: 'reports/2026-11-08-dream-live-sample-esrch-race.md #93377961366349; KBcdc2d461e7d05584ce913f2debb1f0430f57fba8;26unchanged historicalignorednonpasses; runtime_approved=false,fullprivate30sfit/realqualification unproven'
report:
  summary: Landed signed f6dffea0ff8038240de796e237362577e58fe843 on main. Both first and closing TASKALLINFO ESRCH races now use the unchanged actual same-incarnation terminal proof only under exact CURRENT unreaped original custody; errno survives diagnostics and every non-ESRCH/identity/no-exit refusal remains strict. Current canonical C79/48mutants/50compiles/110commands0, affected model tests0, full434-model native/Java parity0, exact required four-chain ONCE0, literal make land0 and fresh remote/clean/signature checks0. INPUT61 stored1434 with signed10-input/compiler/997-check audit tuple; terminal/ledger fields unchanged, runtimefalse/no builder,26 historical deferrals and whole-private30s/qualification gaps remain unproven. Full report committed at cdc2d461e7d05584ce913f2debb1f0430f57fba8; old partial mutant-compile failure and previous builder81 preserved.
  commit: f6dffea0ff8038240de796e237362577e58fe843
  reported_by: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
  op_id: 01M6X8MT6ZC1E26XCBXHXSDT72
  verify_revision: 0
  report: '[[reports/2026-11-08-dream-live-sample-esrch-race.md]]'
provenance:
  digest: sha256:21241a2f517b6d4f37facd0d0ff2f52f4ac5c56ed71b7f67037682a5dccbe680
  frontmatter_digest: sha256:735eebf0c686529204462695763390db08c53a90ce4f1e2e0a94c4ee26b77206
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  sig: CTRJOTgkcZL8hV+QXCOtA5yAPYCCyx++nbMS5jw3j/EdTD2ddn1rG5iGpbkBUEjWWb2xdDEmHH2ic3fSrLjkCA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150620
  event:
    id: 01M6X8NA9DPG6R1PC9R4TCTDNJ
    sequence: 150620
    hash: 48d4b23a94cec8df78a6a497667c930d128cf0ad7921e3dde4b6fc8a9b6f3aea
---
ROLE: EXECUTOR

## Defect (verified by the Oracle in INPUT60 source 1602aff7, proof-helper-probe.c)
Builder run 01M6X25J5JYW6CS88P06H0QYNF ended with exit 81, coverage_unknown "live helper observation unavailable". Its stderr has `observe_failed op=taskall pid=78272 result=0 errno=3`. Ledger sha256 931699ad...4c2.
- engine_sample_member (~2605) checks engine_member_terminal, THEN calls control_observe (proc_pidinfo PROC_PIDTASKALLINFO, ~102).
- A helper that exits between those two calls is a zombie. TASKALLINFO then fails with ESRCH, and line ~2608 reports coverage_unknown.
- That is a check-then-read race, not an unobserved effect.
- The final settlement in that run was 8/8, with own wait4 and the group gone.

## Required fix
- In engine_sample_member, when control_observe fails with ESRCH for an owned, not-yet-reaped member, re-run the existing engine_member_terminal check. Only an actual same-incarnation exit counts: the same birth and token, SZOMB or the existing rusage exit test. If it holds, return the terminal path (1). Otherwise keep coverage_unknown.
- Any non-ESRCH errno, or a terminal check that fails, keeps coverage_unknown.
- No other observation, identity, cap, debt or settlement change.
- control_observe must keep errno intact for the caller, or return it explicitly.
- Unchanged: the 66331 bound, the strict ^{tree} peel, the fields and all law.

## Controls (causal)
- An injected exit between the terminal check and the TASKALLINFO read settles via the terminal path (positive).
- ESRCH where the re-check shows no actual exit (a foreign birth, or the token mismatching) still gives coverage_unknown (81).
- A non-ESRCH errno gives 81.
- A mutant without the re-check must fail the race positive.
- Update the finite model if it represents the live sample, with a broken control.

## Handoff
ONE INPUT61 tuple, in the same format as INPUT60: signed commit, receipt, probe, ten inputs, compiler and audit. State whether any terminal fields changed, with exact names and types. runtime_approved=false. No builder run.

## Verification
make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build
Report that exact string as verify_result.command; run it once on the final head. Land with make land, report in reports/, and task_report once.


## Report from seat:01M6RZZXSHZT3KHBK1QC7K3XB3

Landed signed f6dffea0ff8038240de796e237362577e58fe843 on main. Both first and closing TASKALLINFO ESRCH races now use the unchanged actual same-incarnation terminal proof only under exact CURRENT unreaped original custody; errno survives diagnostics and every non-ESRCH/identity/no-exit refusal remains strict. Current canonical C79/48mutants/50compiles/110commands0, affected model tests0, full434-model native/Java parity0, exact required four-chain ONCE0, literal make land0 and fresh remote/clean/signature checks0. INPUT61 stored1434 with signed10-input/compiler/997-check audit tuple; terminal/ledger fields unchanged, runtimefalse/no builder,26 historical deferrals and whole-private30s/qualification gaps remain unproven. Full report committed at cdc2d461e7d05584ce913f2debb1f0430f57fba8; old partial mutant-compile failure and previous builder81 preserved.

```json
{
  "op_id": "01M6X8MT6ZC1E26XCBXHXSDT72",
  "commit": "f6dffea0ff8038240de796e237362577e58fe843",
  "report": "[[reports/2026-11-08-dream-live-sample-esrch-race.md]]",
  "verify_result": {
    "command": "make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Both live-sample read races, exact CURRENT original custody and real terminal proof; errno preservation; strict negative classes",
        "reference": "reports/2026-11-08-dream-live-sample-esrch-race.md; source9bacbb0483ab328798af2bc98225bdf8f50f930e6d2ecda6779cec3afec75042; canonical C receipt bf043efc833c01a5dd38b050da537a74fbf056d65ddd348ca005c64dd13c3d2a,79/48/50/110 actual0"
      },
      {
        "criterion": "Changed finite model and missing-terminal-recheck broken control; fresh full native/reference parity",
        "reference": "formal freeze63e715d1d6c0fd343a90183f602af1f9785b3f2ba32f7efe8dc63d5501c141f0; archive6e455ce97ad6292c1310d16ef3fa115c8f8a5735817ac00cb30a8e66d9e542e9,434models93contracts,3374874states11363095labellededges; rawmanifestba3cab1de50492ec349b37ae8a968ee6abb7797f993638d46fd7356d66cf6619"
      },
      {
        "criterion": "ONE required exact verification string ONCE on final signed head, actual0",
        "reference": "target/dream-q2-live-sample-esrch-01M6RZZXSHZT3KHBK1QC7K3XB3/esrch-exact-four-chain-v1.json; logSHA2e07a0bbb9f6166b96be871f807596bed66948357edff62d1621e2a2f54be104"
      },
      {
        "criterion": "Canonical make land0; main contains signed HEAD and worktree clean",
        "reference": "esrch-canonical-land-v1.log SHA1a151ba0c9914ff242f0913b8ddea90e0655c3dde4c48413ea5233ce2e53707b; remote-verification-detail-v1.json and report"
      },
      {
        "criterion": "ONE INPUT61 signed preparation/10inputs/compiler/audit; exact unchanged field names/types; no builder",
        "reference": "stored1434/id01M6X8FZM6B3JKH8QVA0JK2N16; tupleff1c31cf724ecae33c874cd65ad25e869f3f6843a0d8eb4ca5814900d8689106; receipteb5f1696cba83e117d354c040b50f0950d5f0fd7847f398255d0a699f73dd987; audit533b16290b9296355c7d2017631eb4fe4a2046263b77dfdd54a20d67e2243790,997checks325headers0"
      },
      {
        "criterion": "Full committed new report; prior accepted records/failures preserved; limits and deferrals honest",
        "reference": "reports/2026-11-08-dream-live-sample-esrch-race.md #93377961366349; KBcdc2d461e7d05584ce913f2debb1f0430f57fba8;26unchanged historicalignorednonpasses; runtime_approved=false,fullprivate30sfit/realqualification unproven"
      }
    ]
  }
}
```
