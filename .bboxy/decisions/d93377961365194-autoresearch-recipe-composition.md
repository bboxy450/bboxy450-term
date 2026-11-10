---
type: decision
title: Autoresearch composes pinned recipes under decisions and executable invariants
status: decided
id: 01M6PNJ1JGKTRKEWKG6DR031X4
updated: 2026-11-05T20:14:15.464Z
created: 2026-11-05T20:14:15.464Z
number: 93377961365194
created_by: seat:01M6P8KEN6RZDN72N74KW8QKJT
updated_by: seat:01M6P8KEN6RZDN72N74KW8QKJT
authorship:
  signer: seat:01M6P8KEN6RZDN72N74KW8QKJT
  kind: seat
  public_key: 7HNDKxdN28ZDEWeI5+lEyXlSHUJgBc6Gz4PH4AfOcH8=
  process_generation: '1'
provenance:
  digest: sha256:4866662e6fe5cb2df65e911df2745e0abe76a2a681a0f642dc57f4ae3092bb33
  frontmatter_digest: sha256:6bef61f8c32d5b5b17bf25ad788291ad91ad8bffa94c0e95c6a510cd04ced45d
  signer: seat:01M6P8KEN6RZDN72N74KW8QKJT
  sig: Siot4WOl3fCXFeQTS0nb7nHsYknDMCp1jqi6Np9QmDidt2nauSrkiaXPmQk/4pZM6ZJtQBbO0LMW8yzu2ifSDw==
  log: .bboxy/log/2026-11-05.jsonl#141632
---
# Autoresearch recipe composition

The operator selected recipe composition, including subrecipes, and requested a designed, committed and pushed recipe that uses decisions and invariants (2026-11-05). This records that design choice, not authorization to launch a particular experiment, provision resources, spend inference budget or publish its results.

## Choice

Use an ordinary top-level recipe with six reusable recipe children: prepare worker, propose experiment, execute, evaluate, retain findings and choose next. Keep the system contract in spec; pin applicable decisions, their supersession, executable invariants and recipe closure for each experiment/run. Decisions retain the rationale and chosen rules; invariant checks retain command, actual exit and evidence. A recipe link grants no authority and cannot weaken an invariant or reset the parent's budget.

## Why

Existing shell/CLI dispatch, harness sessions, SSH/worker transport, operation identities, KB types and provenance supply the foundations. One experiment/evidence contract can compose local commands, environment/provisioning recipes and future native adapters. A runbook is an operational recipe. Introducing another KB type or a separate workflow/dispatch system is unnecessary for this procedure.

## Alternatives

A flat prompt loses reusable child contracts and explicit evidence bindings. A new workflow language or orchestration store duplicates current pieces. A recipe alone treated as execution proof conflates instructions with actual effects; require run receipts, independent evaluation and bounded continuation instead.

## Constraints

Use [[decisions/d28-structured-task-verification]], [[decisions/d30-artifact-provenance-scope]], [[decisions/d365-mandatory-formal-verification]], [[invariants/never-block-swarm]], [[invariants/no-chokepoints]], [[invariants/kill-only-own-pids]] and [[invariants/signed-main]] where applicable, together with current operator instructions/grants. Resolve and freeze actual applicable revisions at admission, including known lookup/verification gaps. Late results cannot transfer to a changed policy, worker or comparison. Missing required proof is not acceptance.

The current native lookup of decision 365 returned number_conflict by both path and exact discovered ID; its tracked source copy and the operator's supplied policy were inspected for this documentation. This is a recorded lookup limitation, not a substitute native policy pin. No research invocation is admitted by this decision.

## Delivered source

[Signed source recipe and six children](https://github.com/bboxy450/bboxy450-term/blob/fbc7a69a196b0b3d4e291006d1828276bb314530/recipes/autoresearch.md), commit fbc7a69a196b0b3d4e291006d1828276bb314530, pushed to main through make land. The source is a procedure/design; executable backend conformance, strict isolation and native automated composition remain separately verified implementation work. Private host KB records were not staged in source Git.

Related design: [[proposals/bounded-autoresearch-loops]]. Native recipe entry: [[recipes/autoresearch]].