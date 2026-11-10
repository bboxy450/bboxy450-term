---
type: report
title: 'Dream term integration architecture: disk-policy review correction'
task: '[[tasks/2026-11-05-dream-term-integration-architecture]]'
author: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
created: 2026-11-05T10:29:41.469Z
commits: []
supersedes: '[[reports/2026-11-05-dream-term-integration-architecture]]'
verify:
  command: git diff --check
  exit_code: 0
id: 01M6NM3MRZWYBYGG72H08GRATA
updated: 2026-11-05T10:29:41.469Z
number: 93377961364364
created_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
updated_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
authorship:
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  kind: seat
  public_key: L62ggzXO9nS9icBZrd6nPN71BnmasbyzYyfTJE3eu0k=
  process_generation: '1'
provenance:
  digest: sha256:c42c2d79bbfe02050a7ae4d93b6f898ba20185db96fc46c5ee7c4123462b238b
  frontmatter_digest: sha256:9af524e06de1eca3b2e907b110c3d0b02137017df2c6b2ac62ab0d286441356e
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  sig: PqtLNv1gD2QpcPuHWV3B40p44y0vFJS1rJ5A65PrFbpEp7j90yMvwQ+yUtlSEaebQeQhcsQhKg6x6TE6apBpAg==
  log: .bboxy/log/2026-11-05.jsonl#134005
---
# Dream integration architecture: narrow disk-policy correction

Task: [[tasks/2026-11-05-dream-term-integration-architecture]].
Supersedes [[reports/2026-11-05-dream-term-integration-architecture]] for the corrected disk rule and current artifact digests. The original report remains append-only and unchanged.

## Change and source evidence

Changed only the proposed disk admission sentence and added a discrepancy note in [[specs/dream-term-integration]], Resource ceilings and allocation decisions.

- .bboxy/specs/budgets.md:58 — the older shared-machine table lists a 10 GiB free-space floor.
- LAND.md:58-60 — current recovery guidance reserves 128 MiB plus 1 MiB per seat (132 MiB for its default four-seat run), checks retained bytes against that recovery budget, and explicitly rejects a blanket 10 GiB free requirement.
- The Dream specification now uses its own explicit materialization allocation (the proposed 128 MiB inspection staging/output ceiling) together with the actual applicable owning subsystem's admission check. It creates no blanket 10 GiB product gate and does not transfer the LAND recovery reserve to Dream. Budget/admission failure defers only the affected materialization.

Neither LAND.md nor budgets.md was edited. Research, ICD, C1–C6 sequence, source reachability, R1–R11 matrix and other accepted design findings were not rewritten. No feature, source change or product test was added.

## Current artifact digests

| Artifact | Current record digest | Disposition |
|---|---|---|
| [[research/dream-term-integration]] | d46654dd0eab4e43fd3eb223eead3543371d696a900ae808c30110c4b4fcbc44 | Unchanged; fetched in this correction turn |
| [[specs/dream-term-integration]] | d0683ab597e68363f5ec5a1d6e2998158c4a33ec0fb3501a70e33e2e4ff67a18 | Corrected disk rule and source discrepancy only |
| [[interfaces/dream-term]] | d5a3987e8c364535bda29e3f3c4e3607ffe4cd3b113573033104ef958e4341de | Unchanged; fetched in this correction turn |

The update used knowledge_write with the fetched spec ref/etag and preserved frontmatter; it returned lossy=false. Complete paged knowledge_fetch readback matched the intended body. Reversing just the new admission sentence and the added discrepancy note reproduced the prior body exactly. This is text/write verification, not a product or model test.

## Verification and worktree evidence

| Command | Actual exit/result |
|---|---|
| nl -ba LAND.md followed by sed -n '48,68p' | 0; source lines above inspected |
| nl -ba .bboxy/specs/budgets.md followed by sed -n '50,65p' | 0; conflicting older row inspected |
| git diff --check | 0; empty output, run after the correction |
| git rev-parse HEAD | 0; ad243195daf74153756308346d01a56390c1261c |
| git status --short | 0; only the five untracked knowledge paths listed below |
| git -C crates/bboxy450-harness rev-parse HEAD | 0; a2ca8f393630081b6be583151293ae286dc2419e |
| git -C crates/bboxy450-harness status --short | 0; empty output |

Short status before creating this append-only correction report:

    ?? .bboxy/interfaces/dream-term.md
    ?? .bboxy/reports/2026-11-05-dream-term-integration-architecture.md
    ?? .bboxy/research/dream-term-integration.md
    ?? .bboxy/specs/dream-term-integration.md
    ?? .bboxy/tasks/2026-11-05-dream-term-integration-architecture.md

This report adds only reports/2026-11-05-dream-term-integration-architecture-review.md through the selected knowledge provider. No path was staged; no commit was created (commits=[]), and no submodule was moved. git diff --check checks tracked differences; the explicit body comparison separately checks the untracked spec correction. Host-authenticated KB provenance is the write evidence.

The broader architecture is supported by the original report's source audit and explicitly unrun future verification matrix. This narrow correction ran no builds, formal/Yarn/TLC checks, unit/integration/smoke/e2e tests, inference or installed-artifact inspection, as required by the amended brief. Existing allocation/provider/producer/trial-support dependencies remain as recorded; no additional decision or permission is requested.

## Acceptance mapping

1. Source-map criterion: unchanged [[research/dream-term-integration]] and original report; current artifact digest confirmed above.
2. Integration/resource criterion: corrected spec Resource ceilings table and discrepancy paragraph; LAND.md:58-60 and budgets.md:58 identify the actual conflicting sources and their scopes.
3. Verification-matrix criterion: unchanged spec R1–R11 matrix, with proposed future checks kept distinct from the readback and git diff --check actually run here.
4. Scope/report criterion: one existing permitted spec changed, this permitted append-only report created, original report untouched, no production/submodule changes or code commits; actual command results, revisions and digests recorded above.

Correction handoff timestamp from the UTC clock tool: 2026-11-05T10:28:40Z.
