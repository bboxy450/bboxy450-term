---
acceptance:
- Existing10GiB disk floor and5GiB private-target budget are preserved, with explicit normal-CI transient allocation and margin plus reason.
- Cold/warm preflight accounts only remaining target allocation, optional cache disposition is bounded, and peer artifacts/ownership remain protected.
- Low-capacity/failure behavior is named nonzero and unattended, with no central approval/queue, silent threshold reduction or operator cleanup prerequisite.
- A signed scoped budget line is delivered promptly to Rook/Oracle before enforcement, with exact checks and measurement gaps reported.
allowed_paths:
- .bboxy/specs/budgets.md
- .bboxy/invariants/e2e-runs-unattended.md
- .bboxy/runs/issue-77-disk-allocation.json
artifacts:
- '[[01M64J58ZFD8041RXBZ3BRT07W]]'
assignee: seat:01M649QP6AMVQWCG28X5MQP3JR
authorship:
  kind: seat
  process_generation: '1'
  public_key: fewHFqX1/aT03A8Ik46eNy0fXOwtv5Rt1gZmJu1Aids=
  signer: seat:01M649MS05FAD5CY9CS0Q07H76
created: 2026-10-29T18:33:44.128Z
created_by: seat:01M649MS05FAD5CY9CS0Q07H76
delegated_by: seat:01M649MS05FAD5CY9CS0Q07H76
id: 01M64F0YXKEHB5P8PQTT066HZB
number: 394
provenance:
  digest: sha256:3281938d8ef27e45b62e536aefacd6d245fe1af2e3dab124e5a4fe7b79a9ef48
  frontmatter_digest: sha256:5cfff45bbfa8ea832f1a8f8f4cf867f222d62ee5e89615459cb984e281e27d91
  log: .bboxy/log/2026-10-29.jsonl#1985
  sig: SO/Ns2OXfujZ3pQcVyDiHC2y6/vumR0w65OHjVYUlfhVLnjBh2mwdbjqa/pOTVydTqVKZ0KY8H+R6zAHjWOjBw==
  signer: seat:01M649MS05FAD5CY9CS0Q07H76
report:
  commit: 9aec6da534424caa33180bd52759e345d4b31f8d
  op_id: 01M64J15WE7PNTJ4N3Z3TCANCA
  reported_by: seat:01M649QP6AMVQWCG28X5MQP3JR
  summary: 'Normal-CI allocation recorded in signed 1b22ad70 and included in final frozen source. F10GiB + max(0,T5GiB-owned allocated target) + R2GiB runtime + M1GiB margin = cold18GiB/warm13GiB on one filesystem; separate-volume arithmetic, unchanged floors, cache-growth0 per-invocation off, named nonzero refusal, liveness/ownership and phase ordering are explicit. Rook/Oracle received exact signed lines promptly. Normal recovery defaults4 seats/8MiB replay/20MiB burst support the separate normal workload; combined normal runtime/artifact peak remains an explicit measurement gap. Enforcement and future capacity mutations are not claimed implemented. make test-task-contract on final signed 9aec6da534424caa33180bd52759e345d4b31f8d exits 0 (1 passed, 0 failed); target/final-frozen-law-task-contract.log SHA256 41fedc64e8057490d0067e31ae300cda51a0c65efbbb17a53c5adc91a1998b84. Nine original Markdown frontmatters (except updated), IDs/numbers/check fields and existing links are preserved, and new links resolve; git diff --check and approved Boxy signature verification exit 0. Clean worktree. The 4ddfa288 broad run exited make 2 / Cargo 101: sole failed test checkpoint_bounded_recovery, free 10539065344 < 10737418240 bytes at recovery.rs:329 (target/issue73-ci-local.log:1411, SHA256 416816a9fb7c560b336cf7a6209867be612c985f496bdcd4be6f3996c63b13df). It is superseded for the current SHA because law edits landed during it under Oracle sequencing authorization. No current-HEAD broad pass claimed. Retired only this worktree''s inactive target/debug after lsof found no holders, preserving logs and app/focused/recovery binaries with hashes in target/retained-law-evidence/manifest.json. Selected retired allocation 4517093376 bytes; observed free afterward15987892224 remains below the new cold18GiB allocation, so no unchanged full rebuild. No product code, live runtime, credentials, peer cleanup, merge/push or recruitment.'
role: architect
status: accepted
swarm_id: 01M649102FBS60QWNPZS5VAGF9
title: 'Issue77: baseline normal-CI disk allocations and optional-cache disposition'
type: task
updated: 2026-10-29T20:41:21.504Z
updated_by: seat:01M649MS05FAD5CY9CS0Q07H76
verify: make test-task-contract
verify_result:
  command: make test-task-contract
  evidence:
  - criterion: Existing10GiB disk floor and5GiB private-target budget are preserved, with explicit normal-CI transient allocation and margin plus reason.
    reference: .bboxy/specs/budgets.md:57 and :67 — preserves F10/T5 and allocates R2/M1 with source-based reason and explicit measurement limits.
  - criterion: Cold/warm preflight accounts only remaining target allocation, optional cache disposition is bounded, and peer artifacts/ownership remain protected.
    reference: .bboxy/specs/budgets.md:77 — remaining-target-only cold/warm and per-volume arithmetic; :103 per-invocation inherited cache off; owned/liveness rules prohibit peer deletion.
  - criterion: Low-capacity/failure behavior is named nonzero and unattended, with no central approval/queue, silent threshold reduction or operator cleanup prerequisite.
    reference: .bboxy/specs/budgets.md:111 — named nonzero low_capacity, phase/fixture checks, no operator prerequisite/global queue/threshold weakening; .bboxy/invariants/e2e-runs-unattended.md:77 preserves unattended/night semantics.
  - criterion: A signed scoped budget line is delivered promptly to Rook/Oracle before enforcement, with exact checks and measurement gaps reported.
    reference: .bboxy/runs/issue-77-disk-allocation.json:1 — signed allocation, current focused checks/record hashes, true superseded capacity failure and measurement gaps. Final source 9aec6da534424caa33180bd52759e345d4b31f8d signed/clean; no broad pass or enforcement claim.
  exit_code: 0
---
# Normal CI disk allocation, bounded supplement

ROLE: ARCHITECT — Nyx Copperwing. Same worktree /Users/feral/mydev/bboxy450-term-mcp-recovery-spec-20261029 and branch. Rook owns native task384, issue77, and has measured the exact missing budgets.md line. Read his current in-swarm measurement message01M64EHN3ARE291ASNPSPD5GTD and cache note01M64EK059REG7BTJC578JR45G, source and existing budgets. This scoped native task authorizes the existing budget record paths only; no code or cohort orchestration.

Existing allocations are floor10GiB and private target5GiB. Rook proposes preflight = floor + remaining target allocation + transient margin, rechecking floor before tests/invariants, optional cache off for full CI because a read-only client cannot reconfigure a shared RW daemon. No shared-cache/peer cleanup. The Oracle is retiring its own completed inactive triage build outputs, with exact logs/binaries preserved; that is cohort capacity release, not proof of the code fix.

Consumer proposal for your recorded allocation: normal functional CI retains floor10GiB + max(0,5GiB-existing owned target) +2GiB transient fixture allocation +1GiB explicit margin; cold total18GiB. These are proposed allocations, not measured exclusive consumption claims. Confirm against normal fixture bounds or provide the smallest credible alternative with reason; do not lower existing floor/counts/limits or infer swap/cache attribution from the old packet. The slow967-seat night remains its own workload/allocation. Recording the line is sufficient for Rook to implement within384's scope; ordinary work and messages continue meanwhile.

Define ownership/liveness and named nonzero low-capacity refusal, before/after-phase checks and cache-mode inheritance in the existing budget agreement. Preserve IDs/numbers and report the signed exact line promptly to Rook and Oracle. Do not touch code, credentials, runtime, installed app, other worktrees/caches/targets, merge/push or task lifecycles. Verification is make test-task-contract and needed record integrity checks; final report includes actual exits and any deferred broad-gate result, never a preclaimed pass.

## Report from seat:01M649QP6AMVQWCG28X5MQP3JR

Normal-CI allocation recorded in signed 1b22ad70 and included in final frozen source. F10GiB + max(0,T5GiB-owned allocated target) + R2GiB runtime + M1GiB margin = cold18GiB/warm13GiB on one filesystem; separate-volume arithmetic, unchanged floors, cache-growth0 per-invocation off, named nonzero refusal, liveness/ownership and phase ordering are explicit. Rook/Oracle received exact signed lines promptly. Normal recovery defaults4 seats/8MiB replay/20MiB burst support the separate normal workload; combined normal runtime/artifact peak remains an explicit measurement gap. Enforcement and future capacity mutations are not claimed implemented. make test-task-contract on final signed 9aec6da534424caa33180bd52759e345d4b31f8d exits 0 (1 passed, 0 failed); target/final-frozen-law-task-contract.log SHA256 41fedc64e8057490d0067e31ae300cda51a0c65efbbb17a53c5adc91a1998b84. Nine original Markdown frontmatters (except updated), IDs/numbers/check fields and existing links are preserved, and new links resolve; git diff --check and approved Boxy signature verification exit 0. Clean worktree. The 4ddfa288 broad run exited make 2 / Cargo 101: sole failed test checkpoint_bounded_recovery, free 10539065344 < 10737418240 bytes at recovery.rs:329 (target/issue73-ci-local.log:1411, SHA256 416816a9fb7c560b336cf7a6209867be612c985f496bdcd4be6f3996c63b13df). It is superseded for the current SHA because law edits landed during it under Oracle sequencing authorization. No current-HEAD broad pass claimed. Retired only this worktree's inactive target/debug after lsof found no holders, preserving logs and app/focused/recovery binaries with hashes in target/retained-law-evidence/manifest.json. Selected retired allocation 4517093376 bytes; observed free afterward15987892224 remains below the new cold18GiB allocation, so no unchanged full rebuild. No product code, live runtime, credentials, peer cleanup, merge/push or recruitment.

```json
{
  "commit": "9aec6da534424caa33180bd52759e345d4b31f8d",
  "op_id": "01M64J15WE7PNTJ4N3Z3TCANCA",
  "verify_result": {
    "command": "make test-task-contract",
    "evidence": [
      {
        "criterion": "Existing10GiB disk floor and5GiB private-target budget are preserved, with explicit normal-CI transient allocation and margin plus reason.",
        "reference": ".bboxy/specs/budgets.md:57 and :67 — preserves F10/T5 and allocates R2/M1 with source-based reason and explicit measurement limits."
      },
      {
        "criterion": "Cold/warm preflight accounts only remaining target allocation, optional cache disposition is bounded, and peer artifacts/ownership remain protected.",
        "reference": ".bboxy/specs/budgets.md:77 — remaining-target-only cold/warm and per-volume arithmetic; :103 per-invocation inherited cache off; owned/liveness rules prohibit peer deletion."
      },
      {
        "criterion": "Low-capacity/failure behavior is named nonzero and unattended, with no central approval/queue, silent threshold reduction or operator cleanup prerequisite.",
        "reference": ".bboxy/specs/budgets.md:111 — named nonzero low_capacity, phase/fixture checks, no operator prerequisite/global queue/threshold weakening; .bboxy/invariants/e2e-runs-unattended.md:77 preserves unattended/night semantics."
      },
      {
        "criterion": "A signed scoped budget line is delivered promptly to Rook/Oracle before enforcement, with exact checks and measurement gaps reported.",
        "reference": ".bboxy/runs/issue-77-disk-allocation.json:1 — signed allocation, current focused checks/record hashes, true superseded capacity failure and measurement gaps. Final source 9aec6da534424caa33180bd52759e345d4b31f8d signed/clean; no broad pass or enforcement claim."
      }
    ],
    "exit_code": 0
  }
}
```
