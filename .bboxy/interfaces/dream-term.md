---
type: interface
title: Dream and terminal host boundary
derived_from:
- '[[research/dream-term-integration]]'
id: 01M6NK8ZDMW0Y8GTHXQS5B85PD
updated: 2026-11-05T10:20:56.299Z
created: 2026-11-05T10:15:06.750Z
number: 93377961364358
created_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
updated_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
authorship:
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  kind: seat
  public_key: L62ggzXO9nS9icBZrd6nPN71BnmasbyzYyfTJE3eu0k=
  process_generation: '1'
provenance:
  digest: sha256:c8cd9dc2fd16a90af3ee01f26e16114958ae6af5751d971cbd2cdbe3dd7b1250
  frontmatter_digest: sha256:bf1cc1ca8273648a4792baa0bcc7410acbae69d541368ea54d142149a888bccc
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  sig: FKM3wU0SnDXQ40ix4xmZMORdSQKXTsnxZ64eReOjf6tzEXucw+6iIkFPO6hdYFLavdFt6jczAl5BbC5UrTydAw==
  log: .bboxy/log/2026-11-05.jsonl#133969
---
# Dream ↔ terminal ICD, proposed v1

Governs [[specs/dream-term-integration]]; grounded by [[research/dream-term-integration]]. This is a proposed boundary for subsequent implementation briefs. No new endpoint, permission or producer enrollment is installed by this document.

## Participants and ownership

Term owns admission, account/seat/swarm/workspace identity, source selection, selected knowledge-provider routing, durable job status and audience. Harness owns IR interpretation and Dream algorithms, trial confinement, publication effects, receipt/seal validation and context construction. The existing harness-wake-up crate owns bounded local transport. Neither the GUI nor an agent model owns orchestration.

The versioned runner protocol is owned by harness Dream maintainers, proposed at crates/bboxy450-harness/src/dream/host.rs. Term's adapter at crates/bboxy450-harness-wake-up/src/dream.rs consumes that protocol without importing the full Dream engine into the mux. Its wire conformance fixtures must match the harness-emitted schema. User-facing request/status DTOs belong at crates/bbt-proto/src/dream.rs. These are proposed new files, not current APIs or new crates.

Both the term host owner and harness Dream owner must review a boundary revision and record its reason before consumers change. The baseline approval remains outstanding; this architecture task does not assign implementation seats.

## Contract table

| ID / participants | Shapes crossing the boundary | Owner on each side | Ordering, lifecycle and age | Errors and version rules |
|---|---|---|---|---|
| D1 caller ↔ mux | Inspect/Start, Status, Cancel; original operation ID; actor derived from authenticated connection; selector own-seat or explicit swarm; expected workspace revision; mode; budget; dissemination target | MCP/ctl host ↔ per-job mux owner | Admit once, persist request digest/context before launching. Status/Cancel never create a second run. Own-seat worker access is read-only in cut 1; swarm reads/writes require current grants. | Same op/different digest refused. Named unauthorized, stale_context, busy, unsupported_mode. No actor/class supplied in JSON can grant authority. |
| D2 mux ↔ source catalogue | Frozen SourceManifest: schema, run_id, context, exact source session/epoch/closed prefix/digest, blob inventory, completion grade, canonical repo and seat-generation membership provenance | Registry/adapters + private catalogue ↔ snapshot worker | Capture roster and membership at admission; validate open files before/after snapshot. No “latest” selection or account-wide glob. Every dependent source must also be authorized. | Missing/partial/changed/oversized source yields explicit omitted reason or incomplete result. Never fabricate a pin or substitute terminal ring bytes. |
| D3 mux ↔ harness worker | Hello/capabilities; Inspect/Run/Cancel/Query; manifest digest; bounded status events; result/receipt locators | wake-up transport + mux job owner ↔ harness host worker | Fixed trusted executable/argv, private job root, off live input/render paths. Independent jobs; nonblocking shard claim. Ack means admission only. Worker boot/run nonce and event sequence survive reconnect in the job record. | bboxy.dream.host/1 is proposed. Reject unknown major/required capabilities. Advertise actual binary hash, build, features, IR reader/writer versions, thresholds and supported stages/adapters. A version string alone is insufficient. |
| D4 harness ↔ selected KB route | Pending draft; exact root-bound write; original write/commit/status IDs; returned record/lossy/refs; committed RecordPin after independent validation | harness flush::Route adapter ↔ captured term knowledge host/provider | No discovery of credentials or shell writer. Persist ticket intent before effect; query original operation after timeout. Revalidate current routing before new admission; preserve returned bytes/provenance. | Current flush sends knowledge_commit{record}; current host requires {locators,message}. Adapter explicitly translates, validates op identity, and resolves exact committed bytes. No lossy record or arbitrary commit string becomes proof. |
| D5 Dream ↔ evidence/trust reader | Event pin; Git Record pin; approved producer/policy capability set; source closure | harness Resolver/Trust ↔ provider evidence adapter/operator-owned trust configuration | Git-backed v1 resolves exact objects from independently allowed main, verifies bytes, signature and ancestry. Capture main once per view; changes require a new view. | Generic fetch/etag/commit does not promise immutable history. Non-Git or incomplete provider capabilities return evidence_unavailable/unsupported_evidence_backend. No bundled .bboxy fallback. A non-Git pin requires a separately versioned contract. |
| D6 trial ↔ child runtime | Validated Plan, caller-tightened Options, isolated scratch/home/profile, retained arm IR, original verify evidence, result and settlement | harness trial owner ↔ existing confined engine/process/tool owners | Serial arms inside an entry claim; unrelated shards keep running. Register children before spawn, retain claims/roots through actual reap. Drop cancels locally; it does not prove settlement. | Present source accepts scripted engine only. Real engine/native/ACP support is a new capability with witness requirements. Unsupported, missing usage, unavailable heavy-test share or unproved cleanup means Inconclusive/quarantine. |
| D7 lower ↔ sink/outer run | Current pinned eligible set, sink domain/audience, immutable generated bytes, PublicationJob, publication receipt + enclosing run receipt/seal | harness lower owner ↔ filesystem/outer run owner | Prepare → observe admitted effects → settle all children → publication receipt → outer R4 run seal → close. No-replace publish after complete temp/sync; existing differing bytes preserved. | producer_unapproved, skipped_conflict, unsupported_loader, unproven_settlement. Stage success alone never enables Recall. Native loaders are capability-specific, with no immediate-forgetting promise. |
| D8 loader ↔ engine/ACP | Current audited Context, audience authorization, exact block digest/RecordPins, reported Recall before dispatch | preflight loader + term admission ↔ harness engine/ACP request seam | Prepare off the live request path. Consume only for the exact target generation/session/harness/repo and current authorized audience. 250 ms maximum optional preflight; absent ready context starts the conversation without memory. | Failed/stale/audit-unproven context is omitted with a bounded diagnostic. No delayed insertion into a different turn. Trial suppresses automatic context; text is a separate quoted user/reference message. |
| D9 job owner ↔ viewers | Immutable redacted summaries, per-run sequence, current phase, proof status, omissions, resource counters and paged provenance links | mux per-job store ↔ CLI/MCP/GUI projection | Result stays attached to original account/root/manifest. Revalidate authorization at read and presentation, including selected-root ABA. Event updates do not automatically wake a model. | Unknown outcome distinct from failure, skip and completed inspection. Bound pages, reject stale cursor; never append Dream nodes directly to the authoritative task/knowledge graph. |

## Context and identities

Context is a host-minted tuple, not an untrusted selector:

    account key ID + custody/revocation epoch
    caller seat ID + process generation + grant epoch (or actual operator admission)
    swarm ID + roster/workdir revision
    workspace ID/revision + knowledge root ID/revision
    provider ID + activated plugin digest/provider epoch
    canonical repo key + selected audience + run/worker incarnation

The manifest binds those values and exact IR prefixes/blobs by digest. Term retains membership sidecars without rewriting source envelopes. A harness SessionMeta has no swarm field. Missing historical membership is unavailable, never inferred from cwd/name/current roster. Explicit operator import can label a historical source as operator-selected; it cannot invent past swarm membership.

A→B→A uses distinct monotonic epochs/generations, even when account name, repo path, selected plugin or session label returns to A. Every new protected-effect admission, context delivery and result presentation checks the captured incarnation. An effect already admitted under the old context must settle against its original target; revocation cancels new admissions and future exposure, not a fictional rollback of bytes already written. An old completion cannot retarget itself to the new A.

Term harness_kind, model/provider name, engine SessionMeta.harness (currently the crate version, src/frontends/headless.rs:377), native harness identity and executable hash are separate fields. Do not pass “bboxy450-harness” as a trial pair just because it is the term seat kind; compare the actual IR/adapter identity. A pass for one harness/version/revival never transfers automatically.

## Custody and provider capabilities

Runtime IR, source snapshots, retained trial arms, process records, flush intent and complete receipts stay in private application/job state, outside source Git. User-requested redacted research/proposals/memory records use the selected provider and its versioning workflow; Git-backed knowledge records may be committed. Secrets and raw transcripts never enter source commits.

The capability adapter must demonstrate: lossless record round-trip; stable root/provider identity; immutable revision lookup; content digest; independent signature/ancestry or explicitly specified equivalent proof; original operation outcome query; explicit commit availability; permitted audience. Existing data_source write/commit support establishes only part of this list.

Git path: task/decision anchors become eligible only when their actual signed bytes are on allowed main. A local knowledge_commit is not push or main reachability. Memory Created requires confirmed exact committed revision; Recall additionally requires current projection, qualifying same-pair trial, trusted anchor and complete approved run/publication evidence. No agent “accepted” message, host etag or newly produced memory can bootstrap trust policy or approve its own executable.

External path: use exact Git semantics if the provider offers and proves them. Otherwise remain in bounded inspection/Pending mode. A later immutable-evidence API must bind provider identity, root, record ID, immutable revision, digest, signer, revocation and origin/source mapping; it must extend consumers/audit/identity together instead of populating a false Git commit field.

## Duplicate runs, restart and failures

A job is addressed by original op ID plus request digest; logical shard claims add account/root/audience/repo/cluster/entry/harness domains. Transport duplicates return the same run. A new op on a busy identical shard returns Busy/Skipped; no waiting or bypass around an uncertain effect. Different shards progress independently.

Cancel uses a separate per-job control flag/lane; a full progress/result lane cannot prevent cancellation or a bounded status query. Viewer backpressure never holds the effect owner or the live conversation.

Runtime phases: admitted → snapshotting → inspecting → completed/skipped/failed for cut 1. Later runs add synthesis → trial → flush-pending → publication → settling → sealed. Cancel enters cancelling/settling; unknown write/commit stays pending on its original ID. Cleanup failure enters quarantine, never sealed. GUI detach preserves the job; explicit Cancel stops new work. Logic restart reattaches an authenticated surviving worker or reconciles retained original operations; it does not automatically repeat inference, verifier, write or publish. A lost affine flush ticket cannot currently be reconstituted: until a tested resumable-ticket API exists, record unavailable and refuse redispatch.

Sources and outputs are append-only by run. Worker restart receives a new nonce. Before/after publication crashes are resolved through exact receipts, original operation status and source/target identity; changed targets are preserved. No symlink traversal or broad deletion. Cleanup addresses only recorded owned roots/inodes and child PID/group generations.

Failures report phase, stable reason code, original op/run, relevant non-secret pins and proof state. “Inspection complete” means the selected corpus was examined, not that memories were adopted or a model used them. A malformed peer, unavailable identity/KB, slow disk or exhausted budget never blocks terminal input, messaging, another swarm or an unrelated Dream shard.

## Versioning and checks

Control DTOs/schema and the runner capability catalogue are separate from append-only IR 1.3. Preserve unknown IR fields on read; reject unsupported semantics for authored effects and trust. First cut adds host control without changing Scope or redefining RecordPin. A future evidence variant requires additive IR/default/golden checks and a trust-policy revision. Feature availability is negotiated from the launched artifact, including acp-agent independently of embedded rendering.

The spec's R1–R11 verification matrix is normative for future implementation. Both owners must pass the shared schema corpus, stale-version/unknown-field controls, real subprocess framing/reconnect tests and the affected source/model gates. Existing Dream models do not alone prove this host boundary.
