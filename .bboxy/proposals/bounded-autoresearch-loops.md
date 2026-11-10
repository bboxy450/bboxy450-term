---
type: proposal
title: 'Bounded autonomous research over existing dispatch: experiments, environments and retained learning'
id: 01M6PDT4VXPWQM27S6888T1H8B
updated: 2026-11-05T20:17:52.429Z
created: 2026-11-05T17:58:52.445Z
number: 93377961365079
created_by: seat:01M6P8KEN6RZDN72N74KW8QKJT
updated_by: seat:01M6P8KEN6RZDN72N74KW8QKJT
authorship:
  signer: seat:01M6P8KEN6RZDN72N74KW8QKJT
  kind: seat
  public_key: 7HNDKxdN28ZDEWeI5+lEyXlSHUJgBc6Gz4PH4AfOcH8=
  process_generation: '1'
provenance:
  digest: sha256:b0f178c6065d55f2d8a99fec23624ee62217e56be76187f481c5ad201df554a7
  frontmatter_digest: sha256:305513a5fdffb344a89a1e5a63bbabe19e7eaeb04a96ee46baf221b97e77a09a
  signer: seat:01M6P8KEN6RZDN72N74KW8QKJT
  sig: IyeOdhqm+H2Fvvy+HrxDpB672b0ju0tImI4jtQo9yWRvjpi6yCh8pchXHVJnvbqIpAA1xNhCnvB70unSdGFdDw==
  log: .bboxy/log/2026-11-05.jsonl#141853
---
# Bounded autoresearch loops in bboxy450-term

## Status and scope

Proposal for the work beyond registering the experiment document type. This document does not enable execution, scheduling, budget spending, candidate promotion or Dream adoption. The operator authorized the small type addition and requested the remaining design here; implementation of this proposal is separate.

Goal: let an authorized research program investigate questions, propose interventions, execute and evaluate bounded experiments, retain evidence and continue safely after detach or restart without operator intervention. Support executable code, prompts, datasets and model/tensor artifacts. Findings may be explanatory, negative, inconclusive or quantitative improvements. Winnow, a fixed-metric code optimizer and an interactive weight-space investigation are domain adapters to the same lifecycle. The research agenda may evolve; each executed comparison keeps its own frozen protocol.

## Published recipe procedure

The operator requested the designed recipe be committed and pushed, using decisions and invariants. [Canonical source recipe and six children](https://github.com/bboxy450/bboxy450-term/blob/fbc7a69a196b0b3d4e291006d1828276bb314530/recipes/autoresearch.md) landed on main in signed commit fbc7a69a196b0b3d4e291006d1828276bb314530. Native entry [[recipes/autoresearch]] composes the six KB subrecipes and links exact source-file hashes. The design choice is recorded in [[decisions/d93377961365194-autoresearch-recipe-composition]]. Verification and tool limits: [[notes/2026-11-05-autoresearch-recipe-delivery]].

This delivers an agent-followable procedure and its immutable source artifact. Native executable composition, strict backend conformance and the other runtime extensions below remain proposal work. Private KB records were written/read back through the native tools and were not added to source Git; their native checkpoint state remains separate.

## Existing pieces and their exact extensions

Source inspection on 2026-11-05: term de089ec26f9519dfcf32c6b387d2d578af9ea9ec and its clean pinned harness a2ca8f393630081b6be583151293ae286dc2419e. The following are implemented source capabilities, not a claim that every library API is exposed by the running MCP service or independently verified by this review.

| Piece we have | Reuse | Specific addition |
|---|---|---|
| Term shell/CLI dispatch and environment selection | [supervisor/protocol.rs:90](https://github.com/bboxy450/bboxy450-term/blob/de089ec26f9519dfcf32c6b387d2d578af9ea9ec/crates/bbt-supervisor/src/protocol.rs#L90) already carries arbitrary program/argv/cwd, explicit environment overrides, inherited-variable removal and an exact-environment bare launch mode. [mcp/seats.rs:22](https://github.com/bboxy450/bboxy450-term/blob/de089ec26f9519dfcf32c6b387d2d578af9ea9ec/crates/bbt-mux/src/mcp/seats.rs#L22) includes zsh among approved seat kinds. | Put the experiment envelope and completion/result wrapper around existing dispatch. Reuse the existing environment mechanism through an admitted experiment launch profile; bare mode's current plugin-login purpose is not a permission bypass. Keep existing caller/grant boundaries. No new general-purpose spawner is required. |
| Existing remote and plugin transport | [adapters/launch.rs:225](https://github.com/bboxy450/bboxy450-term/blob/de089ec26f9519dfcf32c6b387d2d578af9ea9ec/crates/bbt-mux/src/adapters/launch.rs#L225), [seat_state.rs:77](https://github.com/bboxy450/bboxy450-term/blob/de089ec26f9519dfcf32c6b387d2d578af9ea9ec/crates/bbt-proto/src/seat_state.rs#L77) and [ui-model/transport.rs:45](https://github.com/bboxy450/bboxy450-term/blob/de089ec26f9519dfcf32c6b387d2d578af9ea9ec/crates/bbt-ui-model/src/transport.rs#L45) already construct remote command/cwd/environment launches; plugin transport plans retain their producer authority. The operator confirms existing SSH/cloud workers and arbitrary CLI dispatch as available execution reach. | Select those existing workers/transports as experiment backends. Add remote run identity, staging/collection manifests, query/cancel and completion/settlement evidence. A transport process exiting does not identify remote job completion. Specific Docker/Tart/IaC/Kubernetes recipes below are proposed adapters, not claims of installed native integrations. |
| Harness process capture and ownership | [process.rs:25,409,532](https://github.com/bboxy450/bboxy450-harness/blob/a2ca8f393630081b6be583151293ae286dc2419e/src/process.rs#L25): literal program/argv/cwd, wall deadline, owned process groups, cancellation, capture and separately observed settlement. [tools.rs:337](https://github.com/bboxy450/bboxy450-harness/blob/a2ca8f393630081b6be583151293ae286dc2419e/src/tools.rs#L337) already exposes bash and literal exec. | Reuse this owner when execution is inside a harness or a headless worker. Bind experiment/run IDs and structured receipts. This particular Request has no environment map and only strips BBOXY_* variables at spawn; use the existing exact-environment launch path or explicitly adapt this path to the same policy instead of duplicating environment semantics. |
| Harness agent runtime | [gateway/host.rs:57,137](https://github.com/bboxy450/bboxy450-harness/blob/a2ca8f393630081b6be583151293ae286dc2419e/src/gateway/host.rs#L57) creates/opens correlated sessions; engine and headless frontend support continued work, tool policy and retained IR. | Bind session/epoch/profile and permitted operations to an experiment. Reuse this runtime for reasoning/editing when needed; deterministic experiments need no model session. Connect its actual usage to the program budget and retain handoff/checkpoint IDs. |
| Session forks and private trial workspaces | [store/fork.rs:7](https://github.com/bboxy450/bboxy450-harness/blob/a2ca8f393630081b6be583151293ae286dc2419e/src/store/fork.rs#L7) materializes a self-contained IR prefix and blobs; Dream trial builds private repository copies (trial.rs:1614-1710). | Reuse/extract workspace preparation with explicit source/input/environment manifests. An IR fork is not a snapshot of repository files, datasets or Python memory. The current trial snapshot refuses submodules; a general adapter must materialize declared dependencies or refuse before launch. |
| Blob storage | [store.rs:25,50](https://github.com/bboxy450/bboxy450-harness/blob/a2ca8f393630081b6be583151293ae286dc2419e/src/store.rs#L25) hashes/stores/verifies blobs; [store/intake.rs:139](https://github.com/bboxy450/bboxy450-harness/blob/a2ca8f393630081b6be583151293ae286dc2419e/src/store/intake.rs#L139) preserves exact binary bytes. | Reuse these identities for small outputs. Large checkpoint collection needs streaming byte-exact hashing, per-run retention/output limits and manifests. Text-redacting put and whole-file Vec intake are not a suitable large-weight collector by themselves. |
| Native KB and task system | [knowledge/types.rs](https://github.com/bboxy450/bboxy450-term/blob/de089ec26f9519dfcf32c6b387d2d578af9ea9ec/crates/bbt-mux/src/knowledge/types.rs) already provides proposal/spec/research/task/report/artifact; work items provide ownership, handoffs and acceptance. The small experiment registration is recorded below as unlanded. | Add experiment-specific plan validation and a view of runtime receipts/verdicts. Keep program intent in spec, findings in research, outputs in artifact and work/review handoffs in task/report. Task acceptance remains separate from scientific success. Keep KB writes short; do not hold a knowledge writer lease for the duration of an experiment. |
| Operation journal and background IO patterns | [mcp/journal.rs:97](https://github.com/bboxy450/bboxy450-term/blob/de089ec26f9519dfcf32c6b387d2d578af9ea9ec/crates/bbt-mux/src/mcp/journal.rs#L97) binds original operations to admitted roots; [logic/knowledge_effects.rs:125](https://github.com/bboxy450/bboxy450-term/blob/de089ec26f9519dfcf32c6b387d2d578af9ea9ec/crates/bbt-mux/src/logic/knowledge_effects.rs#L125) separates owner-side admission from IO completion. | A thin durable research-job owner correlating program revision, attempt, child/worker incarnation, outstanding operations and next action. Reuse admission/reconciliation patterns; do not run long experiments inside the KB effect queue or claim KB operations already supervise experiment processes. |
| Messaging and events | Existing authenticated messages, receipts and attention policy carry handoffs and notifications. | Add experiment/run IDs and completion/review event references. Continuation is persisted job state and due triggers, independent of model-message wake circuits. Delivery never substitutes for a receipt. No replacement broker is required. |
| Provenance and committed KB revisions | Signed records/events and lineage already exist; [knowledge/commit.rs](https://github.com/bboxy450/bboxy450-term/blob/de089ec26f9519dfcf32c6b387d2d578af9ea9ec/crates/bbt-mux/src/knowledge/commit.rs) creates explicit native checkpoints. [provenance/queries.rs:799](https://github.com/bboxy450/bboxy450-term/blob/de089ec26f9519dfcf32c6b387d2d578af9ea9ec/crates/bbt-mux/src/provenance/queries.rs#L799) exposes the limits of integrity checks. | Bind plan, input inventory, candidate/reference, evaluator, run receipt and verdict to exact revisions/hashes. The producing runner/evaluator must attest its observations; a signature alone does not prove external bytes or scientific validity. Add the selected-provider immutable-evidence adapter needed by Dream rather than invent a second KB store or fabricate allowed-main pins. |
| Usage observation and budgets | Harness [protocol/mod.rs:865-900](https://github.com/bboxy450/bboxy450-harness/blob/a2ca8f393630081b6be583151293ae286dc2419e/src/protocol/mod.rs#L865) distinguishes usage scopes/unknown fields; term usage_read observes account/machine usage. | Per-program admission/stop accounting and backend-specific enforcement. Existing usage displays are not reservations or hard limits. Request/result caps and unknown-cost handling must flow through the runner/provider adapter; current turn requests include max_tokens: None (engine/turn_loop.rs:409). |
| Dream | Existing D0, synthesis, trial, publication, audit and Recall implementations provide the learning path. trial.rs:794/829/1747/1896 currently constrain actual trials to scripted fixtures and a fixed verifier. | Feed reviewed experiment evidence through the existing source/trust/receipt boundary. Add production trial/evaluator adapters where Dream adoption needs them; do not reinterpret an arbitrary successful experiment as a passing memory trial. Experiment execution can ship before automatic memory adoption. |

Related active work: [[specs/dream-term-integration]], [[interfaces/dream-term]], [[specs/dreaming-mcp]], [[interfaces/dreaming-mcp]]. This proposal consumes their boundaries and does not amend their briefs.

Winnow's retained-data workflow and [Karpathy's autoresearch program](https://github.com/karpathy/autoresearch/blob/master/program.md) motivate the common loop. Project-specific capture, comparison and evaluation logic stays in domain adapters. The weight-space source pins and their limitations are recorded below. This proposal is independently written conceptual adaptation, not copied implementation.

## Recipes compose the system

Use the existing recipe type as the composition unit. A top-level autoresearch recipe can invoke subrecipes for worker preparation, proposal generation, bounded execution, evaluation, evidence retention and the next-step decision. Subrecipes are ordinary recipe records, reusable by different parent recipes. A runbook is an operational/recovery recipe; another document type or orchestration system is unnecessary.

The system spec states purpose, invariants, interfaces and acceptance conditions. The recipe describes how to operate that system using those contracts and the available tools. An experiment binds a concrete question, protocol, recipe revision, environment and inputs to bounded attempts. A run is an execution instance; artifact/report/research records retain its outputs, verdict and findings. A task assigns implementation or review work and does not become the scientific result.

Illustrative composition names below are proposed steps, not links to records already created:

~~~text
autoresearch recipe
├─ prepare-worker recipe → verified environment manifest + lease
└─ repeat while the admitted budget/stop rule permits
   ├─ propose-experiment recipe → question + bounded experiment plan
   ├─ execute recipe → run receipt + artifacts
   ├─ evaluate recipe → evidence-bound verdict
   ├─ retain-finding recipe → reviewed research/report/artifact links
   └─ choose-next-step recipe → next question, changed premise or stop
~~~

Each recipe declares inputs, outputs, preconditions, allowed effects/tool/backend requirements, resource ceilings, failure/recovery behavior and the evidence that counts as successful completion. A parent maps a child's outputs into subsequent inputs and declares dependencies or permitted parallel branches. It may use a command/script, a harness-guided procedure or both. Deterministic provisioning and scoring steps do not need separate reasoning agents.

Parent/child links organize the KB. Executable composition additionally needs explicit call/input/output bindings; a wiki link alone does not execute the child or prove its completion. Freeze the invoked recipe closure and its source/artifact references for each experiment/run. Record parent run ID, step/call ID and child run/receipt ID so provenance can explain which recipe version produced which result. Later recipe edits apply to newly admitted invocations, never rewrite old evidence.

Composition must preserve the parent's cumulative budget, effect scope and authority. A child can tighten limits but cannot reset the token/compute allowance or gain permissions from a recipe link. Use a checked acyclic subrecipe call graph with explicit bounded iteration; data-dependent research choices select the next admitted experiment within those bounds. Child failure, unknown execution or unsupported capability follows the parent's declared disposition and remains visible.

Start by letting the existing harness follow the top-level recipe and invoke pinned driver commands for fixed procedures. The thin experiment/job contract supplies admission, budget and evidence/recovery binding. No general Markdown workflow interpreter is required. If a script is opaque, its outer execution receipt does not establish every internal substep; instrument the declared child admissions/receipts or mark that trace unavailable.

Existing KB recipe records and references already represent these procedures. Still to add for executable composition: validated recipe input/output and invocation bindings, exact revision/closure pinning, parent-child run/receipt correlation and cumulative budget propagation in the same experiment runtime. These are contracts over existing dispatch, KB and provenance rather than new foundational services.

## Proposed lifecycle and ownership

program spec → question → inspection/hypothesis → frozen experiment → bounded actions/probes → independent validation → retained finding/lesson → next question

Optimization is one path through this loop. A study can finish with a tested explanation, a failed prediction or no intervention selected; it need not advance an incumbent. The graph grows as evidence suggests the next investigation, rather than requiring every future experiment to be known at admission.

Term owns program admission, durable lifecycle, trigger identity, per-program resource allocation, visibility and restart reconciliation. Harness owns model execution, confined candidate operations and reusable Dream algorithms. A domain adapter owns command construction, data prerequisites, metric extraction and comparison validity. Keep work off UI/input owners; independent programs/shards progress independently. Reuse existing process and job ownership rather than introduce a global runner or queue.

A task can be correctly completed when its candidate loses or its scientifically valid result is inconclusive. Task acceptance and candidate selection are separate outcomes.

## Experiment contract

A future validated executable experiment definition binds:

- governing program/spec revision, question and changed premise;
- exact artifact/source identities, editable scope and fixed evaluator or validation procedure; a candidate and declared reference for intervention comparisons, or committed predictions and a confirmation procedure for inspection studies;
- input inventory hashes, partition use, seed/environment/runtime and dependency identities;
- a pinned environment recipe/build definition and the resolved environment manifest for every worker/evaluator used, with declared compatibility and reconstruction limits;
- runner/profile capabilities and allowed filesystem/network effects;
- metric schema, direction, units, controls, validity conditions and comparison/selection rule;
- maximum wall time, CPU/GPU share, memory, disk, tokens/cost, concurrent attempts and retry count;
- deadlines and stopping rules, including exhausted budget, repeated refusal, unavailable inputs or unconsumed review backlog.

Changing the evaluator or inspected input partition creates a new comparison definition. A Markdown claim cannot create a runtime transition or spending authority. Detailed validation and lifecycle semantics are future implementation; registering the document type alone adds writing/discovery guidance.

## Execution backends and what sandbox means

Use the existing shell/CLI dispatcher as the first experiment executor. A project's Make target, executable or Python script can perform the numerical work. The same dispatch reach can invoke SSH on an existing worker, a container/VM CLI, a cloud job command or a provisioning CLI. Add a thin experiment wrapper and backend recipe to that path rather than build another execution engine. The wrapper binds admitted argv/cwd/environment, limits, run identity and result collection to the existing supervisor or harness process owner. Agent-produced text is not automatically a launch instruction.

Use a harness session where the experiment requires iterative reasoning, editing or tool exchanges; it can call the same CLI dispatcher. Local CLI, SSH/cloud worker and embedded-runtime execution share the experiment contract and receipt format. An embedded runtime is an optional executor adapter. A deterministic command/scorer needs no model session. A dedicated shell seat may display work, but structured receipts supply completion evidence independently of terminal prompts or screenshots.

### Separate guarantees

| Concern | Meaning | Initial implementation and limit |
|---|---|---|
| Execution | Launch work and observe actual completion/cancellation. | Reuse owned processes and, optionally, harness sessions. A subprocess or harness is an executor. |
| Workspace isolation | Keep attempt state separate from the live project and other attempts. | Owned fresh work/output roots, private temporary HOME, explicit environment and pinned source/input copies or admitted immutable references. A private cwd or directory is not an OS access boundary. |
| Confinement | Enforce which files, services, network and compute the code may access. | Current Dream descriptor capabilities constrain advertised read/write tools (tools.rs:656-658,754); ordinary exec can perform further effects inside its process. A harness tool allowlist does not restrict arbitrary subprocess syscalls. Strict protocols need a supported, tested OS/container/VM or external execution-service profile. |
| Reproducibility | Explain the observed result and rerun the declared procedure. | Pin code, data, evaluator, dependencies/interpreter, configuration, environment, seeds and sampling; record hardware/runtime and known nondeterminism; preserve original outputs. A container alone establishes neither these pins nor deterministic results. |
| Scientific validation | Determine what the evidence establishes. | A declared scorer, appropriate controls/partitions and an evidence-bound verdict. Process exit, confinement and signed logs do not establish a scientific claim. |

Use the precise name workspace-only for the first trusted-command profile. It is useful for repository-owned, cooperative experiments and makes no host filesystem/network security claim. Do not market it as a hard sandbox. If a protocol requires enforced no-network, inaccessible held-out data or a protected evaluator, admission checks the backend's proved capabilities and returns unsupported when they are absent. No operator prompt or silent weakening is a runtime fallback.

This leaves sandbox as an optional confinement backend, not a mandatory new subsystem. A shell command may itself launch a supported confinement tool, and a harness can operate within such a backend. Record the actual guarantee and backend identity in the plan/receipt; execution style alone does not select the guarantee. Do not copy the external post's Linux bubblewrap mechanism into macOS and call it equivalent.

### Thin request/result contract

The host validates the experiment's governing spec/revision, admitted actor/root, executor selection, source and input references, effect profile, per-step/program limits and evaluator before starting. For command execution, the host owns program/argv construction from the approved adapter. For harness execution, it owns profile/session policy and the action schemas. The model may propose parameters/actions within that contract; recorded prose cannot grant execution authority.

Prepare an owned isolated workspace with a fresh attempt ID. The runner records requested and observed executor identity with its verification basis, actual admitted argv/profile/session, source/evaluator/input/config hashes, environment manifest with permitted names/redacted bindings, clocks, exit/signal/cancellation, settlement, output inventories and measured or explicitly unknown usage. Preserve failures and spent attempts. A preflight hash alone is not proof of the bytes eventually executed; retain that distinction when a backend cannot prove stronger identity.

The current process cap bounds the in-memory preview, while the normal streaming path can spill output to disk (process.rs:887-904). Add a separate total output/disk allowance and an explicit overrun outcome; do not report the existing preview cap as a complete storage quota. CPU, RSS, GPU and network limits likewise advertise enforced, observed or unavailable semantics honestly. A requested mandatory limit must be supported before admission.

A trusted deterministic scorer can be another ordinary command. It validates the matching run receipt and candidate/input/evaluator identities, then emits a bounded structured result. Candidate stdout is observation data, not the authoritative metric channel. Freeze the evaluator and run it under the required access profile; source/hash checks detect drift but do not create protection against arbitrary same-user code. Quantitative records carry metric name/direction/unit, value or unknown, sample count, exclusions, uncertainty and validity; inspection studies carry predictions and their independent confirmation. Keep evaluator success separate from candidate improvement.

For optimization programs, only the declared comparison rule may advance the incumbent, using a revision-checked update bound to the exact completed experiment. Changing candidate, inputs, evaluator or program revision invalidates an old result. Winnow development gains nominate fresh prospective validation and establish no repeatable profit or live-order authority. Reproducibility can mean replaying retained observations or rerunning a pinned stochastic procedure; declare which is supported.

## Environment, remote execution and provisioning adapters

The operator's clarification closes the execution-reach question: arbitrary CLI dispatch, configurable shells, SSH and existing cloud workers are foundations to compose. Embedding additional runtimes, creating environments or provisioning compute are adapter choices. This proposal does not require a new cloud platform before the research loop can work.

| Adapter/recipe | Compose with existing dispatch | Additional experiment binding |
|---|---|---|
| Existing local or SSH/cloud worker | Run the project command on an already available target. | Worker/host incarnation, runtime/architecture capabilities, remote job ID, staged input hashes and collected output/receipt hashes. |
| Docker | Run container CLI commands; use a native API adapter later if useful. | Image/config identity, declared mounts/environment, owned container ID, supported limits and status/cancel/collect behavior. |
| Tart VM | Use the VM CLI and the guest's supported command/SSH path. | Base image/snapshot and guest identity, provisioning/configuration manifest, owned VM/lease, artifact transfer and actual guest-job settlement. |
| Pulumi / Terraform / Ansible | Run an admitted infrastructure/configuration recipe through its CLI. | Versioned recipe and state scope, provider operation/resource IDs, result endpoints/capabilities, cost/expiry and ownership/reconciliation rules. |
| Kubernetes | Use a cluster CLI to submit a workload; an optional native adapter can watch the same lifecycle. | Cluster/workload identity and incarnation, image/config refs, outputs, status/cancel and resource ownership. |
| Embedded runtime | Invoke an approved runtime/session behind the same action interface. | Exact runtime/profile/version and compatible state/checkpoint, limits and receipt semantics. |

These are possible implementations of the contract, not a statement that each integration or account has been checked in this review. CLI recipes can be the initial adapters. Native SDK/plugin implementations become useful when their lifecycle, watch or performance needs warrant them; they do not change the experiment semantics. Existing coding-worker dispatch scripts are implementation references for worktree/log handling, not research policies: do not inherit their code-landing behavior or broad coding brief into an experiment.

Separate environment acquisition from experiment execution. A program can use an existing worker with no provisioning, reuse an owned environment lease across attempts, or create a bounded ephemeral environment. A provisioning recipe returns an environment lease linked to the exact recipe and resolved manifest: provider/resource identity, owner/incarnation, configuration digest, capabilities, budget/expiry and cleanup disposition. It does not become an independent authority or a global queue.

Conceptual adapter operations are acquire/prepare → stage → start → query/cancel → collect → release. Each stage has an original operation ID and durable outcome; a no-op stage is valid for an existing local environment. These names are proposed internal contracts, not installed public tools. The same per-job owner records progress and keeps unrelated programs moving. Provider-specific state locking stays scoped to the relevant environment/state, not the whole research system.

### Required environment recipe and resolved manifest

Every experiment must record how its worker environment can be created again. Selecting a host, copying a current shell or recording a provisioning command's exit is insufficient. Reuse the existing recipe document type for the procedure, and artifact references for its executable definition and resolved environment evidence; no additional KB type is required.

| Record | Meaning | Required evidence |
|---|---|---|
| Environment recipe | The versioned instructions and inputs for constructing/configuring a worker. | Backend/base image or snapshot identity; setup/provisioning/configuration source revision; interpreter/toolchain/provider/dependency locks; selected configuration and environment policy; workspace/input layout; worker harness/profile/tools; resource and confinement requirements; noninteractive prepare/reset/verification entrypoints. |
| Resolved environment manifest | What was actually instantiated and verified before the run. | Recipe identity, exact resolved image/build/package/tool/config/input identities, worker/harness/runtime and relevant OS/CPU/GPU/driver facts, verification observations, declared external dependencies and unavailable/unreproducible elements. |
| Environment lease | Which actual resource/incarnation is owned and how it is released. | Worker/resource ID and generation, owner, manifest reference, budget/expiry, lifecycle and cleanup disposition. A lease is not the build recipe. |

The recipe may be a pinned setup script plus lockfiles, a Dockerfile plus resolved image digest, a Tart image plus bootstrap definition, a versioned IaC/Ansible project, or an admitted reference to a prebuilt environment with a verified construction manifest. One recipe can serve many experiments. Record all source/base/dependency material required for the claimed reconstruction; a floating latest tag or an unrecorded install step leaves a reproducibility gap. Snapshot reuse is valid when the snapshot itself is retained and identified; rebuilding a costly base from scratch for each attempt is unnecessary.

A KB recipe explains the procedure. The run must also retain its exact executable definition/input bytes or retrievable immutable references. Use a supported provider checkpoint or an admitted content-addressed recipe artifact; an etag alone does not retain the recipe after it changes. The existing experiment type registration does not yet implement these executable-field checks.

Compile selected local configuration into the portable run bundle. Reuse existing launch overlays and exact-environment support. Do not copy an entire HOME, current seat capability, session database or credential files. Record non-secret environment/configuration values and credential reference/version or stated dependency as policy permits, keeping credential material private. A newly created worker receives its own admitted identity/scope. Pin the worker's harness build, profile, instruction/tool configuration and any checkpoint/context needed for a resumed investigation, as well as the numerical workload's dependencies. Live model/service versions that cannot be fixed remain explicit external assumptions.

The execution sequence is: resolve recipe → acquire/create worker → apply recipe → verify resolved manifest → bind experiment and evaluator → run. Verify staged input bytes at the target and collected output/receipt bytes at the receiver. An existing worker can be reused after matching the recipe and reset/verification contract; a recorded remote path or a live Python kernel is not a reconstruction recipe. On mismatch, mark that attempt ineligible or reconstruct through its bounded adapter; do not silently change the environment or require operator interaction.

Keep a reproducibility fingerprint of the material configuration separate from incidental lease IDs, hostnames, timestamps and boot nonces. Retain those instance facts for ownership/provenance, but fresh instances need not have identical IDs. Candidate/reference arms must satisfy the compatibility conditions declared by the experiment: compare exact material fingerprints where required, and explicitly scope any permitted hardware/runtime differences. Matching environments support repeating the procedure; stochastic inference, training and external observations still require their declared sampling/uncertainty treatment.

### Environment acceptance witness

For each adapter, an unattended fixture creates a worker from the pinned recipe in a clean owned location, verifies its manifest, runs the tiny workload, recreates a second worker from the same definition and checks the declared material fingerprint and workload expectations. A missing dependency, changed base/configuration, forbidden ambient environment variable or incompatible worker must produce a retained refusal/mismatch rather than an apparent reproduction. Verify reset/reuse separately from fresh creation. Use private scripted/local fixtures first; documentation of a recipe is not evidence that its reconstruction has been tested.

SSH disconnection or local CLI cancellation does not prove that remote work stopped. Retain the remote run/resource identity and query the original attempt before retrying. Cancellation targets that owned workload; release or destroy only resources owned by the lease after actual settlement. Ambiguous remote execution/provisioning remains unknown/quarantined with its budget/expiry and recovery action, rather than creating another VM/job or deleting shared infrastructure. Preserve failed provisioning and teardown evidence as run outcomes.

Isolation is a selected environment capability: workspace-only on a trusted worker, container/VM confinement, or an appropriately restricted remote host. Remote placement alone is not an isolation proof. Reproducibility still comes from the pinned run bundle, evaluator and retained observations. All tests use private scripted/local provider fixtures without accounts, prompts or live provisioning; a real backend adds its own unattended conformance witness when implemented.

## Durable controller and messaging

Persist admitted → prepared → running → evaluating → settled, with cancelling, failed and unknown-effect paths. Register owned children before execution; retain ownership until actual reap/settlement. GUI detach leaves jobs running. On restart, inspect retained receipts and query original operations before retrying; duplicate triggers identify the same logical attempt and never duplicate protected effects. Cancellation and status use a responsive lane independent of progress output.

Use existing messages/events for experiment-completed, evaluation-ready and review-due notifications carrying stable IDs. Delivery is not execution evidence. Scheduling is a durable program capability with explicit finite budgets and renewal policy. It must work independently of ordinary peer-message wake admission: self-authored messages do not wake, and the current circuit latches after eight admissions in sixty seconds until a direct operator message (crates/bbt-mux/src/messaging/ledger/circuit.rs:10-13,50-75,116-121). Keep that protection; do not build continuation from message ping-pong or require operator resets.

## KB, provenance and Dream extensions

Reuse KB storage, signatures, reference handling and history. Add experiment validation/projection, then typed bindings from spec → candidate/base → input inventory → run receipt → metric → verdict → retained lesson. Runner and evaluator attest observations they actually made; agent claims remain distinguishable. A signature authenticates a statement and its bytes, not scientific truth.

Dream consumes settled reviewed evidence, including negative results and applicability/revisit conditions. Its proposals can inform the next candidate; execution still follows the program contract. Reuse existing trial/adoption/Recall proofs and integrate through the active Dream host boundary. Reading, citing, applying and demonstrating benefit remain separate observations. No loop can grant its own executable trust, alter protected instructions or promote its own memory by writing a KB record.

## Interactive investigation and weight-space programming

The operator added [Takehi, Teoh, Zhang and Khattab's Weight-Space Programming post](https://rikiyat.github.io/blog/weight-space-programming/) as another required workflow. It reports weight inspection, direct construction/editing, and investigations with limited model feedback. The Qwen improvement is associated with reducing repetitive unfinished answers; the Gemma follow-up does not establish a comparable gain. These are published claims, not replications performed here.

The released [harness description](https://github.com/RikiyaT/lm_understand-logs/blob/4d09ba6d6a7e891dd8063974dfe81b3d78e3494f/README.md) documents model continuations whose returned code is executed by an external sandbox and whose outputs become subsequent input. It explicitly omits earlier prompt trials and some intermediate artifacts. We can inspect this released workflow; it does not establish the authors' entire private discovery history.

The [math experiment record](https://github.com/RikiyaT/lm_understand-logs/blob/4d09ba6d6a7e891dd8063974dfe81b3d78e3494f/exp8_math/README.md) describes a retained Python workspace, budgeted practice probes, handoffs across conversations, a frozen tensor edit and later hidden evaluation. Its usage records accumulate within a conversation. The [Go record](https://github.com/RikiyaT/lm_understand-logs/blob/4d09ba6d6a7e891dd8063974dfe81b3d78e3494f/exp6_go_from_scratch/README.md) describes both zero-feedback and iterative variants. These observations motivate the following design extensions; they are our proposed requirements.

### Protocols the same controller must support

| Protocol | Researcher can do | Retained result |
|---|---|---|
| Inspection | Read admitted artifacts, derive properties and commit behavioral predictions under the declared tool policy | Claim plus supporting observations and separately obtained confirmation or counterexample |
| One-shot construction/intervention | Produce an artifact before receiving any behavioral feedback | Frozen submission followed by independent scoring |
| Interactive investigation | Inspect failures, make a hypothesis, edit an artifact and spend a bounded probe budget | Ordered actions, observations, candidate branches and rejected ideas |
| Confirmation | Run predeclared controls, ablations, repeated samples and held-out checks on a frozen candidate | Scoped verdict, uncertainty, regressions and remaining explanatory gaps |

Adaptive exploration uses practice/development evidence. Final confirmation consumes a declared held-out partition after the artifact is frozen; exposing its result for later selection changes its use to development. A revised question, tool privilege or evaluator gets a recorded protocol revision and does not retroactively upgrade an earlier attempt.

### Additional adapter and evidence requirements

1. **Stateful analysis workspace.** Support repeated code/probe exchanges within one durable job, with retained files and optional persistent kernel. A folder shared by bounded subprocess steps is sufficient where appropriate. Save external actions, concise research notes, artifact checkpoints and outstanding request IDs for handoff. After process loss, restore from a proved checkpoint or a declared replay; missing interpreter state stays unavailable. Starting a new model conversation does not reset the experiment or its budget.

2. **Artifact candidates.** A candidate may be a tensor edit or model checkpoint rather than a Git commit. Pin architecture/config/tokenizer, base bytes, edit delta, tensor shapes/dtypes and resulting bytes. Check shape, finite values and format constraints before evaluation. Large weights, activations, traces and results stay in bounded private artifact storage; KB records contain manifests and findings. Declare retained/excluded material and rebuild limitations before launch. An unchanged Git revision alone cannot identify mutable numerical state.

3. **Task-specific tools and budgets.** Declare which stages permit artifact inspection, CPU analysis, behavioral probes, attention/activation views or training. Meter Python steps and model probes separately, including batch size, samples, token limits, CPU/GPU usage and total elapsed time. Preserve actual cost and unknown execution on failed/ambiguous requests independently of any domain's credited-probe rule. Each probe request/result binds the job, checkpoint, protocol revision and original operation ID; delayed results cannot attach to a different candidate. Use local CPU or an existing SSH/cloud worker through the common dispatch contract; selected CPU/GPU/runtime capabilities and job receipts must be explicit. This review does not establish a particular provider's GPU support or a shipped native Docker/Tart/IaC/Kubernetes integration.

4. **Honest restriction evidence.** Restrict exposed tools and actual filesystem/network/compute access where the adapter can enforce them. A general Python process with tensor access cannot be presumed incapable of implementing a forward calculation merely because a prompt forbids it. Distinguish enforced restrictions, audited adherence and unverified scientific protocol claims. The inspected [Go sandbox](https://github.com/RikiyaT/lm_understand-logs/blob/4d09ba6d6a7e891dd8063974dfe81b3d78e3494f/harness/sandbox.py) uses Linux bubblewrap; reusing this protocol on macOS requires a tested local isolation backend or an explicit supported execution host.

5. **Mechanism and generalization checks.** A weight-space adapter can declare ablations, matched random interventions, dose comparisons, unchanged-input controls, additional tasks and repeated seeds. Report behavior changes, proposed explanations and independently supported causal claims separately. Improving one measured outcome does not establish the explanation or universal transfer. Do not require a single scalar score for every useful result.

6. **Research memory and continuity.** Retain negative findings and why an intervention was tried or rejected. Link a reviewed lesson to the precise evidence and conditions under which it held. Dream may propose further questions and consolidate such evidence through its existing trust path. Iterative research actions run within the admitted job; ordinary peer-message wakes are not the per-step execution mechanism. Normalize cumulative versus per-turn usage at the adapter boundary to avoid charging conversation totals repeatedly.

### Acceptance extension

Add a tiny, CPU-only numerical-artifact fixture alongside the code-optimization fixture. Demonstrate inspection with a committed prediction, a one-shot path with no behavioral feedback, bounded interactive edits/probes, and frozen held-out confirmation. Verify artifact lineage, invalid tensor refusal, zero/exhausted budgets, duplicate/delayed probe results, handoff accounting and restart from a checkpoint. Include an intentionally persuasive but false explanation and an ineffective edit; both must remain valid retained research outcomes without becoming accepted conclusions. No reproduction of the post's large models, paid inference or heavy training is authorized by this documentation update.

Extend the future lifecycle models and broken controls for checkpoint identity, probe budget, kernel/worker incarnation and validation-data exposure. Keep unsupported backend guarantees and unmodeled behavior explicit. This adds protocol requirements to the existing runner/controller design; it does not introduce another KB type or parallel orchestration system.

### Source pin and review evidence

Read-only review on 2026-11-05. Public main resolved to 4d09ba6d6a7e891dd8063974dfe81b3d78e3494f via git ls-remote (exit 0); bounded fetches of these exact revision bytes all exited 0. No downloaded code was executed.

| File at that revision | Bytes | SHA-256 |
|---|---:|---|
| README.md | 6198 | b1c51eeb9b9e2935ff227fdc7796c2a67bd3a55014cf99291224f9cc7b93d342 |
| harness/sandbox.py | 3282 | f3e81f9ae9edac578a64b4f7d19e742beea2950866da969b41c576512cfbc46d |
| exp6_go_from_scratch/README.md | 8611 | 835e03d29cae4041d41a6b822593434dd455fa331ba4611cee0ec47fa685a5b1 |
| exp8_math/README.md | 13447 | 473e386ade21ff52240103003b6b125e4362d9fa11010916c1b770ddb0edcbbd |

The original URL's tracking query failed in the web reader; its canonical URL and public raw files were accessible. Initial formatting of a tool summary used an unavailable TextEncoder helper; it was corrected without repeating a write. No source files, timers, workers or experiments were changed or launched. The earlier experiment-type landing remains unlanded at its recorded gate failures.

## Verification and smallest delivery

First implement one bounded fixture-backed program: baseline, worse candidate, crash and improvement. Retain all attempts, advance only the valid improvement and stop at the declared budget. Repeat with interruption/restart and duplicate/delayed completion; require no duplicate execution or stale incumbent update. Use owned throwaway identities and headless process controls, without prompts, credentials or desktop interaction.

Before implementing the stateful controller, specify finite state variables, initial state, owners and atomic transitions. Model attempts, program revisions, leases/child ownership, budget, receipts, cancellation, restart and incumbent selection with TLA+/Yarn plus pinned TLC parity. Include A→B→A revisions and broken controls for stale promotion, duplicate launch and premature ownership release; pair each with a real API/process witness. Record bounds, fairness and implementation-only gaps.

Deliver in order: (1) validated experiment envelope and result wrapper over existing shell/CLI dispatch, using existing explicit-environment support and a source-bound receipt; (2) a deterministic command scorer and tiny CPU fixture; (3) durable bounded continuation/cancellation/restart binding using existing operation identities; (4) the existing harness-session path for interactive work and one existing SSH-worker adapter, with staging, remote-job reconciliation and collection fixtures; (5) reviewed-result feedback through Dream's accepted host/evidence interface, then Winnow integration and UI. Docker/Tart, infrastructure-as-code and Kubernetes recipes/native adapters are independent follow-on choices under the same contract; existing workers require no new provisioning. A retained folder across bounded command steps is sufficient until a persistent kernel is needed. Run checks selected by each diff. No new workflow language, replacement task/message/KB system, mandatory container service or research-program document type is required for the first loop.

## Selected approach and remaining implementation contracts

Start with the workspace-only trusted command adapter, optional existing harness sessions, one receipt/result format and a thin durable job owner. Current task/KB records supply the human-readable program/work/review layer. Current process/session owners perform execution. Current events/journals supply correlation and recovery patterns. Dream supplies evidence-grounded learning. Each piece keeps its existing responsibility.

The additions are concrete integration contracts: experiment plan validation and a wrapper over existing dispatch; reuse of existing environment controls plus output-limit binding; common run/metric receipts and evaluator adapters; durable budget/continuation/restart correlation; optional environment/provisioning/backend recipes; numerical artifact ingestion when needed; and a provider-proof bridge for trusted Dream consumption. Stronger isolation is selected per protocol. No replacement dispatcher, universal provisioning engine or mandatory sandbox service is required.

The first domain fixture is a deterministic local CPU command and scorer plus a pinned worker-environment recipe and clean reconstruction witness. Production programs supply their exact command/profile, recipe and resolved environment/dependency/input manifest, validation procedure and numeric ceilings as configuration. The host enforces supported declared limits and refuses unsupported mandatory capabilities for that run. Existing task completion does not autonomously select a scientific winner; existing signed provenance does not hash experiment outputs for the runner.

This source review changed only the existing native KB proposal. No source edits, builds, tests, workers, experiment execution, new schedules or installs were performed. Source readback confirms process environment inheritance, preview-versus-spill limits, session/fork/blob primitives and current Dream trial restrictions; it does not renew their runtime verification. The earlier unlanded type change and gate evidence remain below unchanged.

## Implementation checkpoint — 2026-11-05

The small type registration is prepared in signed commit 0183806ca0ae04297efaae5780c07de298579d42 on branch codex/experiment-type-20261105 in /Users/feral/mydev/bboxy450-term-vesper-experiment-type. It is NOT landed on main. The tracked worktree is clean. Proposal was already a built-in type and required no registration change.

Changes: crates/bbt-mux/src/knowledge/types.rs registers experiment, its experiments/<slug>.md path and bounded-comparison writing guidance. crates/bbt-mux/tests/mcp/knowledge_types.rs covers public type/schema discovery, signed Oracle creation, worker refusal, readback and type-filtered search. Two stale assertions in that existing test module were corrected to current knowledge_orient bootstrap guidance and callable orient error recovery.

Actual checks:

- make init-submodules — exit 0; exact pinned harness initialized.
- rustfmt --edition 2024 --check on both changed Rust files, and git diff --check — exit 0 after formatting the added type list.
- make test-knowledge-types — final exit 0: 3 unit + 5 type integration + 1 bridge tests passed. Initial exit 2 exposed the stale bootstrap assertion, retained in the original log.
- make test-mcp-hints — final exit 0. Initial exit 2 exposed the stale tools/list recovery assertion; corrected to the current callable orient hint.
- make test-licenses — exit 0; source notices/checks and 10 + 9 fixture tests passed.
- git verify-commit HEAD — exit 0, approved Boxy identity/key 9B07E849480C68328CA20684844ECC0587DE7AB0.
- make land — exit 2 (inner landing gate exit 77). Strict mux Clippy passed, but the mux lib gate produced 366 passed, 2 failed and 1 interrupted, with 10 skipped. No push occurred. After both definitive failures, the owned nextest PID 37585 was sent SIGINT to stop workspace_ready_activity_churn_reaches_writable, which was still running after 246.810 seconds. Dependent app check was not reached.
- A targeted serial diagnostic of the two failed cases with NEXTEST_TEST_THREADS=1 — exit 100: both failed again (1.667 s and 13.243 s). This diagnostic used the direct quarantine wrapper and therefore rebuilt under its default debug settings; it is additional diagnostic evidence, not a substitute for the canonical landing result.

Blocking cases: logic::knowledge_responsiveness_tests::workspace_held_scan_keeps_input_chat_and_other_root_live_and_excludes_writes (500 ms control-call deadline), and logic::ownership_tests::projection_ownership_real_io_survives_cancel_restart_and_root_aba (child private API deadline). Their implementation and test files are untouched by this change. No quarantine, skipped-case override or gate bypass was applied. The remaining scope decision is whether to repair these workspace/projection gate failures before retrying the landing.

Private command logs are retained beside the worktree: bboxy450-term-vesper-experiment-type-{test,test-final,hints,hints-final,licenses,land,gate-diagnostic}.log under /Users/feral/mydev/. No full make test, smoke or formal-model suite was run for this static dictionary addition. No worker was recruited for implementation; no research runtime, live experiment, paid inference, credential or installation change was made.

This proposal was written/read back through the authenticated native KB and remains a draft. The knowledge_commit tool is not exposed to this session; no shell write or source-Git addition was used to bypass it.
