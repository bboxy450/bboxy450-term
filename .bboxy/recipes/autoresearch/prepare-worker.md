---
type: recipe
title: Prepare an experiment worker
derived_from:
- '[[proposals/bounded-autoresearch-loops]]'
- '[[decisions/d93377961365194-autoresearch-recipe-composition]]'
canonical_source:
  repository: https://github.com/bboxy450/bboxy450-term
  commit: fbc7a69a196b0b3d4e291006d1828276bb314530
  path: recipes/autoresearch/prepare-worker.md
  sha256: 46ea4c226b928478ee6d3e1bc28d7334867b8f8a0e74cc27e4fa09001b4ab960
parent: '[[recipes/autoresearch]]'
id: 01M6PNJTJ913BRMWJ5XWFYE5GW
updated: 2026-11-05T20:14:41.162Z
created: 2026-11-05T20:14:41.162Z
number: 93377961365195
created_by: seat:01M6P8KEN6RZDN72N74KW8QKJT
updated_by: seat:01M6P8KEN6RZDN72N74KW8QKJT
authorship:
  signer: seat:01M6P8KEN6RZDN72N74KW8QKJT
  kind: seat
  public_key: 7HNDKxdN28ZDEWeI5+lEyXlSHUJgBc6Gz4PH4AfOcH8=
  process_generation: '1'
provenance:
  digest: sha256:32c9ec504bf88a9b4f6a227d1b48f2daa8e4dde595048700fc2730b57f289565
  frontmatter_digest: sha256:9c62c0c8de5cf896ae7b3020e3767221b22eb3ad683e588d6f0627bdd4d4e67a
  signer: seat:01M6P8KEN6RZDN72N74KW8QKJT
  sig: nwQ1kjjhBFy5CCbNLZghmz5kppniT7nVMGzi8W0dLglcXLlyom6EXqe4E3rI8xnJgaxCzYzIq69828Z9A3nyCg==
  log: .bboxy/log/2026-11-05.jsonl#141634
---
Canonical source: [signed source recipe](https://github.com/bboxy450/bboxy450-term/blob/fbc7a69a196b0b3d4e291006d1828276bb314530/recipes/autoresearch/prepare-worker.md). Source SHA-256: 46ea4c226b928478ee6d3e1bc28d7334867b8f8a0e74cc27e4fa09001b4ab960.

This KB projection preserves the procedure and converts recipe/governance links for KB navigation. The source artifact is committed; native KB checkpoint state is separate.

<!-- Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md -->
# Prepare an experiment worker

Subrecipe of [[recipes/autoresearch|Autoresearch]]. Produce a verified environment
and owned lease from a recorded construction procedure. Existing workers can be
reused after the declared reset and compatibility checks.

## Inputs

The parent's governance/authority and remaining limits; a pinned environment
recipe with executable prepare/reset/check entrypoints; source/input manifests;
an admitted backend/target or provisioning recipe; required isolation/compute
capabilities; retention and cleanup rules.

## Steps

1. Resolve current applicable decisions and invariant checks through the parent
   governance bundle. Confirm this backend/action is in scope. Missing or ambiguous
   policy is a recorded admission gap, never guessed authority.
2. Freeze construction inputs: base image/snapshot, setup/IaC/Ansible definition,
   tool/runtime/provider versions, locks, selected configuration and the verification
   procedure. Retain exact bytes or retrievable immutable artifacts. A host name or
   a floating image tag is not a reconstruction definition.
3. Compile an explicit environment/configuration bundle using existing launch
   controls. Give a fresh worker its own admitted identity and private working/HOME
   paths. Pass approved credential references through the backend mechanism; do not
   copy seat capabilities, session databases, credentials or an entire HOME.
4. Acquire an existing worker or invoke the admitted CLI/native provisioner.
   Register operation/resource ownership before effects. Retain original IDs and
   actual resource incarnations, budget/expiry and cleanup disposition.
5. Stage and verify source/data/configuration hashes. Apply prepare/reset and
   readiness checks unattended. A shell command, Docker/Tart recipe, SSH worker,
   cloud provisioner or Kubernetes adapter can implement this step; supported
   guarantees must be explicit. Workspace separation is not OS confinement.
6. Record the resolved manifest: actual material versions and hashes, worker and
   harness/profile/tool configuration, OS/architecture and relevant hardware,
   verification results and external assumptions. Keep incidental lease/boot IDs
   separate from the material reproducibility fingerprint.
7. Bind the manifest and lease to the parent. Candidate/reference and evaluator
   workers must meet their declared compatibility rule before execution.

## Outputs and verification

Return exact recipe and resolved-manifest references, check command/exit/evidence,
worker/resource incarnation, supported capabilities and owned lease. A provisioning
CLI's success alone is insufficient. The adapter's acceptance fixture reconstructs
two clean workers and checks the declared material fingerprint and tiny workload;
reset/reuse has a separate witness. Stochastic outcomes retain their sampling rule.

## Failure and recovery

Retain setup failure, mismatch, missing inputs or unsupported mandatory capability;
do not silently repair a recipe under the same identity. Query uncertain original
provisioning operations before retrying. Cancel/release only owned resources after
actual settlement, retaining quarantine/expiry evidence when unavailable. This
worker's failure does not hold the UI, KB writer or another program.
