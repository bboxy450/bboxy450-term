---
type: task
title: Implement exec-shim external-helper enforcement (decision 66459 + correction), hand off INPUT70
role: executor
worktree: /Users/feral/mydev/bboxy450-harness-dream-read-q2
allowed_paths:
- tests/fixtures/dream-read/
- src/dream/read.rs
- src/dream/resolve.rs
- src/dream/c1.rs
- src/dream/read_wire.rs
- tests/formal/
- tests/dream_read.rs
- tests/dream_c1.rs
- verification/
- Makefile
- scripts/land.py
- ATTRIBUTION.md
- reports/2026-11-09-dream-exec-shim-enforcement.md
verify: make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build
derived_from:
- '[[decisions/d93377961366459-dream-external-helper-shim-enforcement]]'
- '[[tasks/2026-11-09-architect-review-external-helper-budget]]'
id: 01M70FJG4DC2F40F9QPJ94MHBX
status: accepted
delegated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-09T18:07:37.552Z
created: 2026-11-09T15:42:03.370Z
number: 93377961366460
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
updated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
authorship:
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  kind: seat
  public_key: DbWCjlMnSNUcBZMB2JrdfJ36/iQgSFRbtz78XE3WlhU=
  process_generation: '1'
assignee: seat:01M70FJGXV7W40W80200Z0TDN5
verify_result:
  command: make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build
  exit_code: 0
  evidence:
  - criterion: Exact final-head verification, signed landing and clean remote binding
    reference: target/dream-exec-shim-decision-witness/required-four-v2.json; canonical-land-v2.json; landed-receipt.json; landed-state.json; source-cut.json
  - criterion: Qualified shim source/executable/compiler/form allowlist and 326-header audit
    reference: target/dream-exec-shim-decision-witness/prepared-input70/dream-read-c-preparation/compile-u3u9hnyr/preparation-c24749c7481562f853f776dc0004d8197bd25efc3b6508890dfdcf94f228a56b.json
  - criterion: Terminal budget/debt/pause preservation and native argv/custody/fork/CPU controls
    reference: target/harness-landing/private-engine-controls-_vxxz7tm/run.json; target/harness-landing/private-engine-controls-901blrdc/run.json; target/harness-landing/exec-shim-controls/run-g30zove4/result.json; target/harness-landing/exec-shim-controls/run-r86jxpyp/result.json
  - criterion: Real pinned tool argv and in-process GS1, valid/corrupt signatures; conditional fork/escape limitations
    reference: target/dream-exec-shim-decision-witness/git-shim-crypto-pwa5v219/result.json; actual-git-child-env.json; result.jsonl; research/2026-11-09-dream-shim-fork-census-counterexamples.md
  - criterion: HelperShimBudget baseline plus 19 controls, all 446 native/Java full graph comparisons
    reference: verification/evidence/352f40b4c82aa1b50bca2dd39ba0c7871c1f3d8908040b4f44f31f7da3fc9687/evidence.json; target/dream-exec-shim-decision-witness/current-formal-all-raw-files.json
  - criterion: ONE INPUT70, every field/type change, 343-byte shim basis delta and runtime_approved=false
    reference: target/dream-exec-shim-decision-witness/INPUT70-source-freeze-tuple.json; field-changes.json; INPUT70-binding-audit.json
  - criterion: Evidence retained and only owned build outputs cleaned, disk above 8GiB
    reference: target/dream-exec-shim-decision-witness/cleanup-selective.json; profiles-final.json; current-formal-all-raw-files.json
report:
  summary: 'Dream Exec Shim implemented decision 66459, its Darwin correction and Amendment 2, and landed signed head 76045587db4a96612e2406b9bb08d4191c626253. The exact four-command verification and make land exited 0; all 446 native/Java model graphs and verdicts matched. ONE INPUT70 is /Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-exec-shim-decision-witness/INPUT70-source-freeze-tuple.json (SHA-256 3d42b6ecbdd2a3b5da9ac27a31a6cc7ed1c971300937616d567a8dd7dda0b7d4). Shim basis delta is 343 bytes; runtime_approved=false; no candidate/qualification builder run. Real pinned Git->shim->GPG valid/corrupt witnesses pass, with coalesced-fork and setsid-orphan limitations retained. Full report #93377961366477 is committed at native checkpoint c0019f43beecf85c662f968290c1d54c50b5e1a0. Native artifact_claim rejected both its committed report locator and checkpoint ID as artifact_not_committed; the fetched knowledge record independently confirms publication_state=committed. All code/evidence is retained, owned Cargo outputs cleaned, worktree clean.'
  commit: 76045587db4a96612e2406b9bb08d4191c626253
  reported_by: seat:01M70FJGXV7W40W80200Z0TDN5
  op_id: 01M70QVTVW3HJHK0KSFEQR4MEH
  verify_revision: 0
  report: '[[reports/2026-11-09-dream-exec-shim-enforcement.md]]'
provenance:
  digest: sha256:aee15f3dd6c1bfa891f3ce639385fd7cb4096ada113656755834ab404ec5b5e4
  frontmatter_digest: sha256:da0a7adfff1c84f1a415fa5702fb314f3699ec95412ade7df1427e849da73bde
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  sig: wnuhmMbzOCTgTIJj4qiLCAXzv6KqzAVZvn+iEphWbomeoi154fNKeKtI9twnYD4ND5AsDe1s2MWjVeROolkNBQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151089
  event:
    id: 01M70QX25GN8QVG2DRPRZ6RN26
    sequence: 151089
    hash: 902cb132d9669229ca610e14b6ce3fcb4cedd203b612ad987027702afd5ceaac
---
ROLE: EXECUTOR

## Goal
Implement [[decisions/d93377961366459-dream-external-helper-shim-enforcement]], INCLUDING its same-day CORRECTION (RLIMIT_CPU is not enforced on Darwin 27; the live CPU read is non-gating on failure). Then hand off INPUT70 for the parked proof-tools builder. Read the whole decision first.

## Worktree
/Users/feral/mydev/bboxy450-harness-dream-read-q2 (the previous Q2 executor's tree). Start from harness origin/main (637cb234 or later): fetch and rebase or reset the BRANCH to origin/main only if the tree is clean. Keep everything under target/ (frozen inputs and evidence); do not delete other runs' evidence.

## Required (one cut)
1. A new `dream-exec-shim` C source next to proof-helper-probe.c (in the qualified source set, strict flags, header audit):
   - argv/env allowlist validation per form (65126), exiting 81 on mismatch;
   - for the gpg verify form, the GS1 temp-file custody checks done IN-PROCESS before exec;
   - a self QoS check (throttled band) before exec;
   - fd closing;
   - execve of the absolute tool path;
   - writes its checks to an owned preopened log fd.
   Configure git's gpg.program to the shim.
2. The probe/controller (proof-helper-probe.c, plus the Rust resolve/read/c1 code if they gate the same way):
   - spawn the shim (SETPGROUP + background QoS, 66331 bounds);
   - a leader NOTE_FORK/NOTE_EXIT knote, where more than 1 fork kills the group;
   - the terminal wait4 verdict, 9x debt, ru_maxrss <= 16 MiB, final census/group-gone and GS2;
   - live RI reads NON-GATING on failure, with a successful over-cap read killing the group;
   - REMOVE as gates: the 50 ms gap, engine_usage_identity monotonicity, same-wake admission/STOP-before-GS1, sampled members_max and the per-member child rollup proof (they may stay as diagnostics).
   - Pause is unchanged (SIGSTOP group, ACK via census plus observe_or_terminal, USER exactly 0 and SYSTEM <= 50 us, one SIGCONT).
3. Measure and record the decision's listed assumptions with real-process witnesses:
   - git 2.54.0 calls gpg.program=shim with the expected argv for verify;
   - the shim's in-process checks see the temp file before gpg runs.
4. Replace the Q2HelperTotalBudget model with HelperShimBudget (decision section Verification), with a broken control per gate including "a failed live read refuses".
5. Real controls:
   - shim argv mismatch -> 81;
   - temp-file custody violation -> 81;
   - an extra fork kills the group;
   - an escaped setsid child is caught by the census;
   - an over-cap CPU loop is killed by the live read (successful) and refused by wait4;
   - a positive and a corrupt proof.
6. Keep all 446+ models in native/Java parity; update coverage/contracts.

## Handoff
ONE INPUT70 tuple (same format as INPUT69), now including the shim binary and its source in the tuple. Report every terminal or ledger field change with exact names and types. runtime_approved=false. No builder run.

## Verification
make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build
Run it once on the final head, then land with make land, report in reports/, and task_report once.
QoS standing rule: an existing Dream timed test family may be registered in the harness serialized lane (scripts/land.py SERIALIZED_LIB_FAMILIES, mirrored from the Makefile) without asking.
Disk: one build tree (target/harness-landing); keep free disk above 8 GiB; clean your own build output after landing.


## Report from seat:01M70FJGXV7W40W80200Z0TDN5

Dream Exec Shim implemented decision 66459, its Darwin correction and Amendment 2, and landed signed head 76045587db4a96612e2406b9bb08d4191c626253. The exact four-command verification and make land exited 0; all 446 native/Java model graphs and verdicts matched. ONE INPUT70 is /Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-exec-shim-decision-witness/INPUT70-source-freeze-tuple.json (SHA-256 3d42b6ecbdd2a3b5da9ac27a31a6cc7ed1c971300937616d567a8dd7dda0b7d4). Shim basis delta is 343 bytes; runtime_approved=false; no candidate/qualification builder run. Real pinned Git->shim->GPG valid/corrupt witnesses pass, with coalesced-fork and setsid-orphan limitations retained. Full report #93377961366477 is committed at native checkpoint c0019f43beecf85c662f968290c1d54c50b5e1a0. Native artifact_claim rejected both its committed report locator and checkpoint ID as artifact_not_committed; the fetched knowledge record independently confirms publication_state=committed. All code/evidence is retained, owned Cargo outputs cleaned, worktree clean.

```json
{
  "op_id": "01M70QVTVW3HJHK0KSFEQR4MEH",
  "commit": "76045587db4a96612e2406b9bb08d4191c626253",
  "report": "[[reports/2026-11-09-dream-exec-shim-enforcement.md]]",
  "verify_result": {
    "command": "make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Exact final-head verification, signed landing and clean remote binding",
        "reference": "target/dream-exec-shim-decision-witness/required-four-v2.json; canonical-land-v2.json; landed-receipt.json; landed-state.json; source-cut.json"
      },
      {
        "criterion": "Qualified shim source/executable/compiler/form allowlist and 326-header audit",
        "reference": "target/dream-exec-shim-decision-witness/prepared-input70/dream-read-c-preparation/compile-u3u9hnyr/preparation-c24749c7481562f853f776dc0004d8197bd25efc3b6508890dfdcf94f228a56b.json"
      },
      {
        "criterion": "Terminal budget/debt/pause preservation and native argv/custody/fork/CPU controls",
        "reference": "target/harness-landing/private-engine-controls-_vxxz7tm/run.json; target/harness-landing/private-engine-controls-901blrdc/run.json; target/harness-landing/exec-shim-controls/run-g30zove4/result.json; target/harness-landing/exec-shim-controls/run-r86jxpyp/result.json"
      },
      {
        "criterion": "Real pinned tool argv and in-process GS1, valid/corrupt signatures; conditional fork/escape limitations",
        "reference": "target/dream-exec-shim-decision-witness/git-shim-crypto-pwa5v219/result.json; actual-git-child-env.json; result.jsonl; research/2026-11-09-dream-shim-fork-census-counterexamples.md"
      },
      {
        "criterion": "HelperShimBudget baseline plus 19 controls, all 446 native/Java full graph comparisons",
        "reference": "verification/evidence/352f40b4c82aa1b50bca2dd39ba0c7871c1f3d8908040b4f44f31f7da3fc9687/evidence.json; target/dream-exec-shim-decision-witness/current-formal-all-raw-files.json"
      },
      {
        "criterion": "ONE INPUT70, every field/type change, 343-byte shim basis delta and runtime_approved=false",
        "reference": "target/dream-exec-shim-decision-witness/INPUT70-source-freeze-tuple.json; field-changes.json; INPUT70-binding-audit.json"
      },
      {
        "criterion": "Evidence retained and only owned build outputs cleaned, disk above 8GiB",
        "reference": "target/dream-exec-shim-decision-witness/cleanup-selective.json; profiles-final.json; current-formal-all-raw-files.json"
      }
    ]
  }
}
```
