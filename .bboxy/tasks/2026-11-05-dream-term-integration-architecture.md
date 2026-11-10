---
type: task
title: Ground Dream and swarm Dream integration into bboxy450-term
allowed_paths:
- research/dream-term-integration.md
- specs/dream-term-integration.md
- interfaces/dream-term.md
- reports/2026-11-05-dream-term-integration-architecture.md
- reports/2026-11-05-dream-term-integration-architecture-review.md
verify: git diff --check
id: 01M6NJF1W3MP1PNY7EBNMH7H3H
status: accepted
delegated_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-05T10:30:43.416Z
created: 2026-11-05T10:00:57.308Z
number: 93377961364352
created_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
updated_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
authorship:
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  kind: seat
  public_key: UgGFd6fs0Ln9gJXlN2fw8kUmonpBkr4IBn/U5N0uOrY=
  process_generation: '1'
role: architect
assignee: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
verify_result:
  command: git diff --check
  exit_code: 0
  evidence:
  - criterion: Source map names actual producer/consumer code, current revisions, personal and swarm Dream behavior and missing seams, with file:line citations.
    reference: '[[research/dream-term-integration]] unchanged at d46654dd0eab4e43fd3eb223eead3543371d696a900ae808c30110c4b4fcbc44; prior source audit retained in [[reports/2026-11-05-dream-term-integration-architecture]].'
  - criterion: Integration plan defines a minimal vertical slice and ordered cuts, interfaces/owners, scoped authority/provenance, lifecycle/failure behavior and resource limits.
    reference: '[[specs/dream-term-integration]] resource table/discrepancy note corrected, digest d0683ab597e68363f5ec5a1d6e2998158c4a33ec0fb3501a70e33e2e4ff67a18; LAND.md:58-60 versus .bboxy/specs/budgets.md:58. Other C1–C6/ICD behavior unchanged.'
  - criterion: Verification matrix connects every proposed stateful requirement to model/negative control and unattended implementation witness; distinguishes proposed checks from checks actually run.
    reference: '[[specs/dream-term-integration]] R1–R11 matrix unchanged; [[reports/2026-11-05-dream-term-integration-architecture-review]] records only readback and Git checks actually run; no product/formal runs.'
  - criterion: No production/source or submodule changes; report records actual read/verification commands and exits, revisions, artifact locators, unresolved decisions and deliberate exclusions.
    reference: '[[reports/2026-11-05-dream-term-integration-architecture-review]] supersedes original handoff, preserves original report, records current digests, term/harness HEAD and short status, git diff --check exit 0, commits=[] and narrow exclusions.'
report:
  summary: Applied the requested narrow disk-policy correction only. The spec now admits Dream materialization against its explicit job budget and the actual applicable owner's admission check; it records the older budgets.md:58 versus current LAND.md:58-60 discrepancy, removes the blanket 10 GiB product gate, and does not apply LAND's recovery reserve automatically to Dream. Complete readback confirmed only the requested sentence/note changed; current research/spec/ICD digests are in the append-only superseding review report. git diff --check exited 0; term remains ad243195 and embedded harness a2ca8f39, with no production/submodule edits, commits or product tests. Original report remains unchanged.
  commit: null
  reported_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  op_id: 01M6NM4AN1EGZHPATJMMK0RMC3
  verify_revision: 0
  report: '[[reports/2026-11-05-dream-term-integration-architecture-review.md]]'
rejection:
  reason: 'One narrow correction before acceptance: remove the blanket 10 GiB free-space product gate, resolve the old budgets.md versus current LAND.md discrepancy as specified in the amended brief, and submit the permitted append-only correction report/current digests. Source reachability, C1–C6 sequence, ICD and verification matrix otherwise reviewed.'
  by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
provenance:
  digest: sha256:7f7d7980609640105dddeaf630994535d772e12c42b6ba973ae6c115d3c6af32
  frontmatter_digest: sha256:1692d17b32cd888c6fb08f88b3a9d3a94b8a183c190c1443e93ba93f0e446fbe
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  sig: P8XPlxpDZ6VNOJEVFXkQIPsB1B84C3ge85krLoO5sVrwJVRSd7x5rCtRPIJnyNSKOXi48PX8QD+I/6kBo/obCA==
  log: .bboxy/log/2026-11-05.jsonl#134016
---
ROLE: ARCHITECT

## Goal
The operator asks how to integrate the already implemented bboxy Dream and swarm Dream, using IR logs, into bboxy450-term. Orient yourself, ground the design in the actual implementation, and produce an integration plan. This is architecture/research only.

## Scope
Seat: Sable Quill (01M6NJ9RS9EXJ7M5AK1AY1B3JR), existing Architect beside Rook Ashwhisker.
Working directory: /Users/feral/mydev/bboxy450-term. Read shared/project AGENTS.md, .bboxy/vision.md, relevant specs/decisions/interfaces, and both repository Git states.
Read-only source access in this repository, its embedded crates/bboxy450-harness, and a canonical sibling harness checkout if necessary. Head observed: term 59b06c30, embedded harness a2ca8f393630081b6be583151293ae286dc2419e. Recheck before citing.
Write only the allowed knowledge paths through knowledge_write. Respect the selected provider's storage and versioning workflow; knowledge records may be committed to Git. Keep secrets and raw transcripts out of Git. No KB commit is required for this planning assignment. No production edits, submodule updates, builds, implementation assignments, merge, publish, credentials, or operator desktop control.
First call orient, then knowledge_orient; discover tools/schemas. Use task_report for the final handoff. Do not recruit.
Existing seams worth investigating: harness src/dream/{mod,graph,recall,receipt,projection,synth,join,lower,flush,trust}.rs, IR types/store/readers, harness CLI/ACP and term native/ACP adapters, session discovery/history, mux seat/swarm ownership and knowledge provider.

## Required outcome
Write research/dream-term-integration.md with precise file:line evidence and specs/dream-term-integration.md with a staged integration plan. Add interfaces/dream-term.md if the boundary warrants a separate ICD.
Distinguish shipped/currently reachable APIs from library-only capabilities and missing orchestration. Trace personal Dream and swarm Dream from source IR ingestion through synthesis/trial/adoption/publication/Recall, including scope, authorization, provenance and output custody. Explicitly test the operator's premise against current sources; do not invent a swarm scheduler or assume harness feature existence means term exposes it.
Define the smallest end-to-end slice, ordered later stages, exact likely component/file owners, user/operator/agent control surfaces, and out-of-scope behavior. Cover manual versus automatic triggers, per-seat/swarm/repo/account boundaries, tool/adapter/version compatibility, non-blocking operation, duplicate runs, cancellation/restart and generation/identity A→B→A, bounded CPU/disk/inference, failure reporting and no automatic authority promotion from memories.
Include an ICD table (participants, shapes, owners, ordering, lifecycle, errors, versioning), concrete resource allocations or clearly marked decisions still needed, plus verification matrix: TLA+/Yarn/TLC model and broken controls where stateful, real API/process witnesses, unit/integration/unattended smoke/e2e, and canonical Make targets. List exact first implementation cut and dependencies. Preserve grounding uncertainties honestly.

## Acceptance criteria
- [ ] Source map names actual producer/consumer code, current revisions, personal and swarm Dream behavior and missing seams, with file:line citations.
- [ ] Integration plan defines a minimal vertical slice and ordered cuts, interfaces/owners, scoped authority/provenance, lifecycle/failure behavior and resource limits.
- [ ] Verification matrix connects every proposed stateful requirement to model/negative control and unattended implementation witness; distinguishes proposed checks from checks actually run.
- [ ] No production/source or submodule changes; report records actual read/verification commands and exits, revisions, artifact locators, unresolved decisions and deliberate exclusions.

## Verification commands
Run git diff --check from the term checkout (record actual exit); inspect git status --short and rev-parse HEAD for term and embedded harness. This planning task does not require product builds/formal runs; identify the appropriate future gates, do not claim they ran.

## Handoff
Create append-only reports/2026-11-05-dream-term-integration-architecture.md through knowledge_write with task link, commits: [], verify command/exit_code, evidence per acceptance line and gaps. Then task_report with one paragraph and that locator. Silence while working except an all-work blocker.

## Operator clarification, 2026-11-05
The operator explicitly rejects the blanket claim that .bboxy is private/uncommitted. Knowledge records may live in Git, and another machine may select an external KB plugin. Rook is correcting the shipped seat guidance in a separate tiny commit. Ground the integration contract in the selected provider's actual custody, provenance and versioning capabilities; do not design around a prohibition on committing .bboxy. The committed-record-only Dream resolver still needs an honest compatibility account for external providers.


## Report from seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR

Grounded Dream integration in term 59b06c30 (closing ad243195, guidance-only change) and embedded harness a2ca8f39. Saved research, C1–C6 plan and D1–D9 ICD: current source exposes Dream primitives but lacks full personal/swarm orchestration; synthesis CLI and trial adapter remain scripted. Proposed first delivery is bounded closed-IR inspection/status with zero inference, followed by separately gated trial/flush/publication/seal/Recall, production adapter and authorized swarm/provider expansion. R1–R11 map scope, ABA, duplicates, settlement, budgets and authority barriers to models, broken controls and unattended witnesses. git diff --check exited 0; no production/submodule edits, commits, builds or formal/product tests. Full commands, artifact digests and unresolved dependencies are in the report.

```json
{
  "op_id": "01M6NKTXC4ZAJ1XDPCB64QZKQJ",
  "commit": null,
  "report": "[[reports/2026-11-05-dream-term-integration-architecture.md]]",
  "verify_result": {
    "command": "git diff --check",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Source map names actual producer/consumer code, current revisions, personal and swarm Dream behavior and missing seams, with file:line citations.",
        "reference": "[[research/dream-term-integration]] — source tables and Source reachability versus promised workflow; harness src/dream/mod.rs:32-45, synth.rs:877-942, trial.rs:675-684, gateway/host.rs:297-331."
      },
      {
        "criterion": "Integration plan defines a minimal vertical slice and ordered cuts, interfaces/owners, scoped authority/provenance, lifecycle/failure behavior and resource limits.",
        "reference": "[[specs/dream-term-integration]] — C1–C6, R1–R11 and resource table; [[interfaces/dream-term]] — D1–D9, identity, provider custody, cancellation/restart and versioning."
      },
      {
        "criterion": "Verification matrix connects every proposed stateful requirement to model/negative control and unattended implementation witness; distinguishes proposed checks from checks actually run.",
        "reference": "[[specs/dream-term-integration]] — Verification design and matrix (R1–R11), explicit bounds/initial state/actions/fairness and future gates; [[reports/2026-11-05-dream-term-integration-architecture]] — Checks not run."
      },
      {
        "criterion": "No production/source or submodule changes; report records actual read/verification commands and exits, revisions, artifact locators, unresolved decisions and deliberate exclusions.",
        "reference": "[[reports/2026-11-05-dream-term-integration-architecture]] — actual git diff --check exit 0, empty tracked diffs/harness status, 96-command register with failures, source revisions, four allowed KB records, commits=[] and exclusions."
      }
    ]
  }
}
```


## Narrow review correction
Correct only the proposed disk admission rule: the spec's resource table says to respect an existing 10 GiB free-space floor, but current LAND.md:58-60 explicitly says recovery uses 128 MiB + 1 MiB/seat and does not require a blanket 10 GiB free. The older .bboxy/specs/budgets.md:58 conflicts. Record that discrepancy and use the Dream job's explicit materialization budget plus the actual applicable owner admission check; do not create a blanket 10 GiB product gate or imply LAND's recovery reserve applies automatically to Dream. No rewrite, broader audit, new feature or product tests. Re-run git diff --check; create reports/2026-11-05-dream-term-integration-architecture-review.md with supersedes linking the first report and current artifact digests, then task_report again. Original report stays append-only. Other findings accepted on source review.


## Report from seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR

Applied the requested narrow disk-policy correction only. The spec now admits Dream materialization against its explicit job budget and the actual applicable owner's admission check; it records the older budgets.md:58 versus current LAND.md:58-60 discrepancy, removes the blanket 10 GiB product gate, and does not apply LAND's recovery reserve automatically to Dream. Complete readback confirmed only the requested sentence/note changed; current research/spec/ICD digests are in the append-only superseding review report. git diff --check exited 0; term remains ad243195 and embedded harness a2ca8f39, with no production/submodule edits, commits or product tests. Original report remains unchanged.

```json
{
  "op_id": "01M6NM4AN1EGZHPATJMMK0RMC3",
  "commit": null,
  "report": "[[reports/2026-11-05-dream-term-integration-architecture-review.md]]",
  "verify_result": {
    "command": "git diff --check",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Source map names actual producer/consumer code, current revisions, personal and swarm Dream behavior and missing seams, with file:line citations.",
        "reference": "[[research/dream-term-integration]] unchanged at d46654dd0eab4e43fd3eb223eead3543371d696a900ae808c30110c4b4fcbc44; prior source audit retained in [[reports/2026-11-05-dream-term-integration-architecture]]."
      },
      {
        "criterion": "Integration plan defines a minimal vertical slice and ordered cuts, interfaces/owners, scoped authority/provenance, lifecycle/failure behavior and resource limits.",
        "reference": "[[specs/dream-term-integration]] resource table/discrepancy note corrected, digest d0683ab597e68363f5ec5a1d6e2998158c4a33ec0fb3501a70e33e2e4ff67a18; LAND.md:58-60 versus .bboxy/specs/budgets.md:58. Other C1–C6/ICD behavior unchanged."
      },
      {
        "criterion": "Verification matrix connects every proposed stateful requirement to model/negative control and unattended implementation witness; distinguishes proposed checks from checks actually run.",
        "reference": "[[specs/dream-term-integration]] R1–R11 matrix unchanged; [[reports/2026-11-05-dream-term-integration-architecture-review]] records only readback and Git checks actually run; no product/formal runs."
      },
      {
        "criterion": "No production/source or submodule changes; report records actual read/verification commands and exits, revisions, artifact locators, unresolved decisions and deliberate exclusions.",
        "reference": "[[reports/2026-11-05-dream-term-integration-architecture-review]] supersedes original handoff, preserves original report, records current digests, term/harness HEAD and short status, git diff --check exit 0, commits=[] and narrow exclusions."
      }
    ]
  }
}
```
