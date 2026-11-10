---
type: recipe
title: Execute an admitted experiment
derived_from:
- '[[proposals/bounded-autoresearch-loops]]'
- '[[decisions/d93377961365194-autoresearch-recipe-composition]]'
canonical_source:
  repository: https://github.com/bboxy450/bboxy450-term
  commit: fbc7a69a196b0b3d4e291006d1828276bb314530
  path: recipes/autoresearch/execute.md
  sha256: d5e31ca817b90771de45a3a2b48d88ff22ab18b9234baf2b7c44eb141f7c4c44
parent: '[[recipes/autoresearch]]'
id: 01M6PNJWN69RYWCDJBYAW6RJB5
updated: 2026-11-05T20:14:43.928Z
created: 2026-11-05T20:14:43.928Z
number: 93377961365197
created_by: seat:01M6P8KEN6RZDN72N74KW8QKJT
updated_by: seat:01M6P8KEN6RZDN72N74KW8QKJT
authorship:
  signer: seat:01M6P8KEN6RZDN72N74KW8QKJT
  kind: seat
  public_key: 7HNDKxdN28ZDEWeI5+lEyXlSHUJgBc6Gz4PH4AfOcH8=
  process_generation: '1'
provenance:
  digest: sha256:15d4fd9aafc264efcc847f5d2e2cb8b195bcaccb8f5c85169913475ef01a8526
  frontmatter_digest: sha256:5005e63eb882fadfc606618df19398e7e2a9aa8b383065f605fa5f1d4e05f10f
  signer: seat:01M6P8KEN6RZDN72N74KW8QKJT
  sig: eCzeDKiC221bXJVhmneZaJHmy8YCZayjW5epJ4PxASvW471WUJCL7xemNWnX+wCSQW3IC27oNSSLctHDqEkuAA==
  log: .bboxy/log/2026-11-05.jsonl#141638
---
Canonical source: [signed source recipe](https://github.com/bboxy450/bboxy450-term/blob/fbc7a69a196b0b3d4e291006d1828276bb314530/recipes/autoresearch/execute.md). Source SHA-256: d5e31ca817b90771de45a3a2b48d88ff22ab18b9234baf2b7c44eb141f7c4c44.

This KB projection preserves the procedure and converts recipe/governance links for KB navigation. The source artifact is committed; native KB checkpoint state is separate.

<!-- Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md -->
# Execute an admitted experiment

Subrecipe of [[recipes/autoresearch|Autoresearch]]. Invoke existing command dispatch
or a harness session and retain source-bound evidence of its actual outcome.

## Inputs

A frozen admitted plan, current governance/authorization and parent budget,
verified environment/lease, exact candidate/reference/input/evaluator artifacts,
an approved executor/profile and private output location.

## Steps

1. Revalidate the captured policy/worker incarnation, mandatory checks and remaining
   budget before new effects. A recipe link cannot widen authority or reset a quota.
2. Persist attempt/parent/step and original operation IDs before launch. Register
   child/resource ownership and cleanup scope. Prepare fresh owned output state;
   preserve earlier attempts and the live project.
3. Invoke the admitted project command with literal arguments, working directory,
   explicit environment and limits through the existing supervisor/process owner.
   A shell, Python script, container/VM CLI or remote worker is an executor choice.
   Use the existing harness engine when iterative reasoning/editing is required.
4. For interactive work, record actions, checkpoint identities, concise research
   notes, probe inputs/results and outstanding operations. A new conversation
   preserves the run and cumulative budget. File checkpoints or declared replay
   restore state; missing kernel memory is not implicitly recovered.
5. Measure declared usage and output/storage limits. The existing process preview
   cap is not a total spill quota. Record which limits are enforced, observed or
   unavailable; refuse unsupported mandatory guarantees. Preserve unknown cost.
6. Observe exit/signal/cancellation, actual child/IO settlement and artifact closure.
   For SSH/cloud, retain and query the remote workload ID/incarnation. Local
   transport exit or a detached pane does not establish remote completion.
7. Freeze a receipt and inventory with plan/source/input/environment/checkpoint
   bindings, executor/session identity, admitted argv/profile, clocks, termination,
   usage, outputs, exclusions and verification basis. Keep private logs out of Git;
   numerical artifacts remain byte-exact, separate from redacted text previews.

## Outputs and verification

Return the original run/operation ID, receipt, immutable output inventory and actual
settlement state. A model's claim, terminal prompt, forged score in stdout or API
acknowledgement is not execution evidence. An opaque script's outer receipt does
not prove internal substeps that were not instrumented.

## Failure and recovery

Query unknown original effects before retrying. Preserve crashes and partial runs.
Cancellation stops new admissions and targets only recorded owned children or
resources; keep ownership until settlement, or record quarantine and recovery.
Do not reset a shared checkout, extend a spent attempt, launch paid fallback,
copy another seat's identity or replay a command merely because its reply was lost.
GUI detach preserves work. Expiry follows the declared backend lifecycle.
