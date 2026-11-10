---
type: recipe
title: Autoresearch
derived_from:
- '[[proposals/bounded-autoresearch-loops]]'
- '[[decisions/d93377961365194-autoresearch-recipe-composition]]'
canonical_source:
  repository: https://github.com/bboxy450/bboxy450-term
  commit: fbc7a69a196b0b3d4e291006d1828276bb314530
  path: recipes/autoresearch.md
  sha256: 849578c0125dec0395e567c448a98363b5ea69040d2a1287e87c4259a6e91098
children:
- '[[recipes/autoresearch/prepare-worker]]'
- '[[recipes/autoresearch/propose-experiment]]'
- '[[recipes/autoresearch/execute]]'
- '[[recipes/autoresearch/evaluate]]'
- '[[recipes/autoresearch/retain-findings]]'
- '[[recipes/autoresearch/choose-next]]'
id: 01M6PNK1WXA2CMD36QVT6YNSWR
updated: 2026-11-05T20:14:49.289Z
created: 2026-11-05T20:14:49.289Z
number: 93377961365201
created_by: seat:01M6P8KEN6RZDN72N74KW8QKJT
updated_by: seat:01M6P8KEN6RZDN72N74KW8QKJT
authorship:
  signer: seat:01M6P8KEN6RZDN72N74KW8QKJT
  kind: seat
  public_key: 7HNDKxdN28ZDEWeI5+lEyXlSHUJgBc6Gz4PH4AfOcH8=
  process_generation: '1'
provenance:
  digest: sha256:52dafc41a39f4bfe208a99c552fc671157c88dd15189b2d5c173c051f0d9976c
  frontmatter_digest: sha256:591820faa104a9293ef35a45a763c656f393d7a055c5d238ea97f91db04b9ac5
  signer: seat:01M6P8KEN6RZDN72N74KW8QKJT
  sig: HvXb+UOZDNtnT/PWELE+fl4kv1bH7iqtbYRw21EnT5b1sxYuaQGPHRPOoGoNZweckOIEVdqIQafVcTVbhnp7Dw==
  log: .bboxy/log/2026-11-05.jsonl#141646
---
Canonical source: [signed source recipe](https://github.com/bboxy450/bboxy450-term/blob/fbc7a69a196b0b3d4e291006d1828276bb314530/recipes/autoresearch.md). Source SHA-256: 849578c0125dec0395e567c448a98363b5ea69040d2a1287e87c4259a6e91098.

This KB projection preserves the procedure and converts recipe/governance links for KB navigation. The source artifact is committed; native KB checkpoint state is separate.

<!-- Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md -->
# Autoresearch

A composable, bounded research procedure for an admitted harness or driver using
existing command dispatch, workers and KB tools. It supports investigation,
construction, intervention and replication. A useful result can be an explanation,
a counterexample, no improvement or an improved candidate.

This is an agent-followable recipe, not an installed workflow interpreter or a
claim that all execution backends exist. Instantiate the inputs and verify the
chosen adapter before running. Publishing this recipe launches no experiments,
provisions no resources and grants no spending, execution or publication authority.

## Design decision

Compose ordinary `recipe` records and source procedures. Use a `spec` for the
system's observable contract, decisions for adopted choices and their rationale,
invariants for executable obligations, and experiments for concrete bounded tests.
A runbook is an operational recipe. Keep one execution/evidence contract across
CLI, harness, SSH, container, VM, cloud and future native adapters.

The first executor is an existing project command. A harness supplies reasoning
and editing when needed. Reuse the supervisor/process owners, session logs,
operation journal, KB and provenance. Add experiment-specific bindings and
backend recipes rather than a second dispatcher or mandatory sandbox service.

## Governing records

These repository records are navigation anchors. At admission, resolve the current
applicable decisions, supersession chain, instructions and invariant revisions
from the selected KB/provider. Pin exact supported revisions or retained
content-addressed snapshots in a governance bundle. A filename, current etag or
copied instruction alone is not an immutable policy record or a permission grant.
Current operator instructions and authenticated grants retain their precedence.

| Record | Obligation in this recipe |
|---|---|
| [[decisions/d28-structured-task-verification|Structured verification, decision 28]] | Keep command, actual exit and evidence per criterion. Task completion and scientific success are separate. |
| [[decisions/d30-artifact-provenance-scope|Attribution and provenance, decision 30]] | Retain the producer and verification basis; unknown ownership and incomplete proof remain explicit. |
| [[decisions/d365-mandatory-formal-verification|Formal verification, decision 365]] | Stateful implementation changes carry models, broken controls and API/process witnesses under the repository's canonical checks. |
| [[invariants/never-block-swarm|Never block the swarm]], [[invariants/e2e-runs-unattended|unattended checks]] | No test or iteration requires an operator prompt, desktop action or borrowed identity. |
| [[invariants/no-chokepoints|No chokepoints]], [[invariants/ui-never-waits|responsive UI]] | Work off live input/UI owners; keep independent programs progressing. No experiment holds a KB writer lease across execution. |
| [[invariants/kill-only-own-pids|Owned cleanup]] | Record owned process/resource identities before effects. Cancel and clean up only those instances. |
| [[invariants/signed-main|Signed source]] | Source publication follows current approved identity, signing and [landing](https://github.com/bboxy450/bboxy450-term/blob/fbc7a69a196b0b3d4e291006d1828276bb314530/LAND.md) rules. A research verdict does not authorize a merge or deployment. |

Decisions explain which choices apply and why; invariants provide the checks that
must hold. Neither becomes a scientific result. If governing records conflict,
are ambiguous or have no required executable witness, retain a named admission
gap for that program. Do not guess, weaken a check or stall unrelated work.

For each applicable invariant, bind its assertion, command/adapter check, cwd,
expected exit, timing, evidence and coverage limits. Reuse source-bound checks
when valid. Run only the checks required by the changed source/protocol and the
declared experiment. Repository source gates are not automatically meaningful in
another domain: map the obligation to an appropriate domain/API witness.

## Inputs

An instantiated program must supply:

- Purpose/question, governing spec and decisions, invariant checks and current
  authorization/workspace/provider context.
- This recipe and the selected subrecipes at exact revisions, plus their input
  and output bindings. Checked child references form a DAG; iteration is explicit.
- Admitted source/data artifacts and their intended use: exploration/development,
  controls and withheld confirmation. Record known exclusions and missing inputs.
- A versioned worker/environment recipe, executable preparation/reset/verification
  entrypoints, required backend capabilities and an admitted target or provisioner.
- A fixed evaluator/confirmation procedure and interpretation/selection rule.
  Inspection-only studies declare predictions and how they will be checked.
- Numeric attempt, elapsed-time, concurrency, token/cost and storage ceilings,
  per-step limits, supported enforcement and retry/lease expiry rules. Unknown
  usage is never free. No implicit paid-provider fallback is allowed.
- A private run/artifact location, retention policy, parent run/operation IDs and
  review/stop conditions. No credentials or raw harness transcripts enter source Git.

No generic default scientific objective, infrastructure account or unlimited
budget is supplied by this recipe. Missing mandatory inputs make that invocation
not admitted; independent admitted programs can continue.

## Composition

| Order | Subrecipe | Consumes | Produces |
|---|---|---|---|
| Once, then verify/reset as required | [[recipes/autoresearch/prepare-worker|Prepare worker]] | Governance, environment definition, authorized target and limits | Resolved environment manifest and owned lease |
| Each iteration | [[recipes/autoresearch/propose-experiment|Propose experiment]] | Question, retained findings, remaining budget and environment | Frozen bounded experiment plan |
| Each attempt/arm | [[recipes/autoresearch/execute|Execute]] | Plan, inputs, current lease and executor | Run receipt and artifact inventory |
| After actual settlement | [[recipes/autoresearch/evaluate|Evaluate]] | Matching receipts, fixed evaluator and confirmation inputs | Scoped verdict and evidence |
| Every disposition, including failure | [[recipes/autoresearch/retain-findings|Retain findings]] | Plan, receipts, verdict and applicability limits | KB research/report/artifact links and review state |
| After disposition | [[recipes/autoresearch/choose-next|Choose next]] | Evidence, governance changes and remaining budget | Next bounded experiment, protocol revision or stop |

The same subrecipe may be called by several parents. Child invocations have their
own IDs and receipts, but share the parent's cumulative budget and effect scope.
A child can tighten limits; it cannot reset a quota or gain authority through a
link. Record parent run, step/call and child run/receipt IDs. Pin the invoked
recipe closure for each run; later edits affect new admissions, not old evidence.

Start by following these steps in the existing harness and invoking pinned
project driver commands for fixed procedures. A shell script may implement a
subrecipe. Its outer receipt does not prove uninstrumented internal substeps.
Do not infer completion from a terminal prompt, model message or transport exit.

## Records and evidence

Use the existing KB types; do not create a parallel task or memory store.

| Record | Required binding |
|---|---|
| Governance bundle, retained as an artifact | Active decision/spec/invariant/recipe revisions, applicability, check commands and outcomes, original context and known gaps |
| Environment recipe (`recipe`) | Exact base/setup/provisioning/configuration and dependency inputs; verification/reset procedure; capability and resource requirements |
| Environment manifest (`artifact`) | What actually ran: resolved material versions/hashes, runtime/hardware, configuration and verification; external assumptions |
| Environment lease, in private runtime evidence | Actual worker/resource incarnation, owner, manifest, expiry and cleanup state |
| Experiment | Question, parent/recipe/governance references, candidates or predictions, input/partition use, fixed evaluator, bounds and planned disposition |
| Run receipt (`artifact`) | Original attempt/operation and parent IDs; executor/session/worker identity; admitted command/profile; source/input/environment hashes; clocks; exit/signal/cancellation; settlement; usage or unknowns; output inventory |
| Evaluation (`artifact`, summarized in `report`/`research`) | Matching plan/run/candidate/environment/evaluator identities, metrics or checked predictions, controls, invalid/excluded cases, uncertainty and verdict |
| Findings (`research`) | Observation versus inference, supported claims, failures, applicability/revisit conditions and exact evidence links |
| Work/review (`task`/`report`) | Assignment and verified completion evidence. A negative experiment may still be correctly completed work. |

The experiment document type supplies a definition record; it does not by itself
launch work or validate these fields. Discover actual KB/backend support first.
Use the provider's supported immutable evidence contract; never fabricate a Git
commit or trust pin for a draft/non-Git revision. Text logs may be redacted; retain
byte-exact numerical artifacts through an appropriate binary collector.

## Continuation, checks and recovery

Before the first run, record finite state, owners, bounds and atomic actions for
any new controller/backend implementation. Define safety, progress and fairness,
then map them to the formal checks and implementation witnesses required by
decision 365. Pure parsing or numerical calculations retain specialized tests.

Before each new protected effect, validate current authorization, applicable
governance revision, recipe/worker incarnation, remaining budget and required
checks. A changed decision or invariant requires a new policy/comparison revision
where it affects the protocol. A late completion cannot authorize work under an
old revision, including A -> B -> A. Already admitted work settles against its
original resource/context and remains recorded.

Keep original operation IDs for uncertain effects. Query before retrying; do not
repeat an unknown model call, command, provisioning request, write or publication.
Cancellation stops new admissions and targets owned children/resources. Local
SSH exit does not prove remote termination. Retain ownership through actual
settlement; quarantine unavailable cleanup without blocking unrelated programs.

Messages notify. Durable run/operation records drive continuation. Event notices
and peer wake circuits are not the iteration clock. GUI detach preserves work;
explicit cancellation follows the selected backend's proved lifecycle.

Stop this invocation when its budget/deadline expires, its question is dispositioned,
required evidence/capabilities are unavailable, or its declared review/novelty rule
fires. Preserve why and the useful next condition. Do not extend a spent attempt,
relax an invariant, turn an inspected holdout back into unseen data or launch an
unauthorized fallback. Independent work remains available.

## Acceptance of an executable adapter

A private, unattended fixture must demonstrate: baseline/worse/crash/improvement;
an inspection prediction and a one-shot no-feedback path; two clean environment
reconstructions; invalid inputs and exhausted budgets; duplicate/delayed completion,
cancel/restart and worker/policy ABA; evidence retention; scoped cleanup and no
stale promotion. Pair stateful contracts with broken controls and real process/API
witnesses. A declared unsupported capability is an honest outcome, not a passing
execution. This source recipe is a design/procedure, not that runtime witness.

## Origin

Project-owned procedure, with conceptual inspiration from
[Karpathy autoresearch](https://github.com/karpathy/autoresearch),
[Winnow](https://github.com/Gonzih/winnow) and
[Weight-Space Programming](https://rikiyat.github.io/blog/weight-space-programming/).
No external implementation or experiment result is copied or claimed reproduced.
Keep the [project attribution policy](https://github.com/bboxy450/bboxy450-term/blob/fbc7a69a196b0b3d4e291006d1828276bb314530/ATTRIBUTION.md) with extracts.
