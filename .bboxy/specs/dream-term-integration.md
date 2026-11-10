---
type: spec
title: Dream integration into bboxy450-term
derived_from:
- '[[research/dream-term-integration]]'
links:
- to: '[[interfaces/dream-term]]'
  rel: interface
id: 01M6NKFP4H8VH3F73Q4D1YA9RB
updated: 2026-11-05T10:28:20.714Z
created: 2026-11-05T10:18:46.692Z
number: 93377961364360
created_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
updated_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
authorship:
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  kind: seat
  public_key: L62ggzXO9nS9icBZrd6nPN71BnmasbyzYyfTJE3eu0k=
  process_generation: '1'
provenance:
  digest: sha256:53976687574d53b9ba82aed054057c6c2493037afdb3d737ac0e1a1e51494cd2
  frontmatter_digest: sha256:ef844564e57dab2a607706e89560b41b61cba3bdb5a7608849b67ec43423f9a9
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  sig: pdQC7G6ErxC0vQYziTCprAjqGKxJKDL4IhAx+1aW+UVouQafgKqe6YUnP1Gy/QygKiKRb8xEBBO9lc42RzssCA==
  log: .bboxy/log/2026-11-05.jsonl#134000
---
# Integrating Dream into bboxy450-term

Status: proposed integration contract; architecture only. Source baseline and uncertainties: [[research/dream-term-integration]]. Detailed boundary: [[interfaces/dream-term]]. Task: [[tasks/2026-11-05-dream-term-integration-architecture]].

## Purpose and decisions

Reuse the harness's Dream implementation behind a bounded background worker. Term supplies authenticated source selection, provider routing, budgets, persistent job control and result presentation. Keep one algorithm implementation in the harness and one authorization boundary in the mux; the existing rich face remains a client.

The current source supports substantial Dream primitives, not a complete exposed personal/swarm workflow. The smallest useful end-to-end delivery is selected closed IR → scoped D0 inspection → durable redacted result → status/provenance lookup inside term, with zero inference. It can ship while trust enrollment, production trial support and provider history are unavailable. The first full learning loop is a separate staged outcome, not a claim made by that inspection slice.

Personal/swarm describe the source selector and output audience. Dream IR Scope stays Repo/System. A swarm is not a new trust class or an automatic cross-repo opt-in. Knowledge records may live in Git; the selected provider determines their custody and versioning. A live accepted task, generic etag or arbitrary commit string is insufficient for the current Git-pinned trust proof.

## Observable behavior and controls

Proposed control surfaces, all through the same admitted job API:

| Actor/surface | Initial behavior | Later behavior |
|---|---|---|
| Operator ctl dream inspect/status/cancel | Select an exact owned seat-session or explicit IR import in a selected workspace; inspect and page its result; cancel by original run ID | Start a budgeted manual learning run with explicit source/audience/provider/profile; manage opt-in schedules and native dissemination |
| Agent MCP dream_inspect/dream_status/dream_cancel | Own authorized catalogue entries and own runs only; immediate admitted/busy/refused result | Existing grants may permit explicit same-swarm inspection; write/commit/run capabilities stay separately authorized. No role-name or remembered text grants them |
| Term GUI | Reuse existing status/document presentation after the command slice; selected seat/swarm actions call the same API | Show phase, exclusions, Pending/Inconclusive/unproven causes, exact evidence and output audience; no synchronous corpus scan |
| Harness worker | New versioned host mode for a manifest-bound job; existing Dream library performs replay/audit/query | Run caller-owned stages and constructive outer receipts; report actual supported adapters/features |
| Automatic path | Existing detached harness close-triggered D0 remains deterministic; no newly invented swarm scheduler | Explicitly enabled per-object event/schedule/quota policy, with retained trigger identity, budget admission and no automatic model wake from status events |

These command/tool names are proposed, not installed. The current dream CLI lacks run/trial/flush/lower dispatch and synthesizes only with --scripted. Current trial::prepare accepts the crate-version scripted engine path, not native/ACP or arbitrary production profiles.

A result distinguishes selected/omitted sources and proof states: inspection-complete; proposal-Pending; Tested/Inconclusive; per-harness Adopted; publication observed; outer run sealed; Recall insertion reported. None is a synonym for model use or instruction authority.

## Source, authority and custody requirements

R1. Capture a host-authenticated manifest of exact retained IR bytes, blobs and membership. Personal defaults to the caller's explicitly selected history; cross-generation history requires retained mapping and current permission. A swarm manifest freezes one swarm roster/workspace revision, includes only readable members, and partitions by canonical repository. Missing historical membership is unavailable, never inferred from cwd, a shared repo, a display name or the current roster. Terminal captured screen/history bytes do not become IR.

R2. No live terminal input, messaging, GUI frame, knowledge read or unrelated job waits for Dream. Snapshotting, Git/provider reads, parsing, synthesis and verification run off logic/render owners. Admission uses bounded try-send/self-service claims and returns Busy/Skipped when unavailable. Independent shards have independent owners; no single Dream agent, model seat, global scan, blocking lock or serial machine-wide runner.

R3. Persist original operation ID, request digest, captured context, stage intents and original effect IDs before launching effects. Duplicate same-request delivery returns the same run. Unknown write/commit outcomes are queried by original ID. Same-ID/different-content refuses. No automatic retry of uncertain inference, verify, write or publication after restart. Durable flush recovery requires a tested resumable ticket API; until then a lost ticket is Pending/unavailable, not permission to create a fresh one.

R4. Cancellation stops new admissions and retains actual owners until processes/effects settle or are quarantined. GUI detach does not stop work; explicit Cancel does. An enclosing run can seal only after all children and protected effects are observed/joined, then store the receipt before SessionClosed. Claim release, API return, an exit code and a closed UI are not settlement evidence. Logic/worker restart cannot turn an unproven run into completed adoption.

R5. Use the captured selected provider, root and plugin revision. For the first trusted path, require exact Git object read, independent signed KB provenance, allowed-main ancestry and configured producer approval. Acceptance of a live task remains separate from committed truth. knowledge_commit commits explicit documents and never pushes; main-unavailable stays Pending. External providers without those exact semantics retain inspection/Pending mode. No local fallback, automatic export/mirror, fabricated commit or self-approved trust policy. A non-Git immutable-evidence bridge is a later explicit contract.

R6. Enforce the resource table below. No inferred zero usage, quota bypass, operator prompt or invisible heavy-test allowance. Missing capacity affects only that job/stage. Retained source/effect evidence is not deleted to make a budget pass.

R7. The learning loop uses current strict source/prompt checks, same-entry/same-harness trial criteria, confirmed committed memory, immutable no-clobber publication and complete approved outer receipt. A delayed Pass cannot reopen a terminal revision or transfer to another harness/version. Source sessions exposed to the entry/closure are excluded from evidence/held-out trials, regardless of Recall basis. Real profile support is not obtained by removing the current scripted allowlist.

R8. Output dissemination is separately authorized. Personal evidence does not authorize a repo-wide committed memory or native account sink; a swarm run does not authorize another swarm's Recall. A provider must be able to enforce the requested audience, or the run needs explicit broader dissemination authorization already present in its request. Start with engine context for an admitted target, not global native files. Revalidate account custody epoch, seat generation/grant, swarm/workspace/root/provider incarnation before new effects, exposure and result presentation, including A→B→A. Already admitted effects settle on their original target. Context is quoted user/reference data, never system/developer prompt, policy, grant, executable approval or AGENTS/rules/skills mutation. If preparation/logging is unavailable, the actual conversation proceeds without memory.

R9. Native input is retained through the harness import custody APIs; exact original line/blob pins remain resolvable. Native EOF is labelled snapshot completion, not proof of application exit. No wholesale scan of account transcripts and no import into a different root when selected custody fails. Native/ACP trial/loader capability is checked independently from import or display capability.

R10. Sink publication preserves existing differing bytes, hand edits, symlinks and unknown ownership. Use the existing no-replace law, complete temporary bytes, sync and constructive receipt. Cleanup touches only recorded owned roots/inodes and child generations. Native consolidation can retain stale copies; retirement is not evidence of immediate vendor forgetting.

R11. Store immutable result summaries under the original account/job context. Status reads and presentation revalidate authority and generation; a completion for A1 cannot replace B or a newly selected A3. Bound result pages and graph nodes, show gaps and proof status explicitly, and keep derived Dream graph separate from authoritative task/knowledge graph mutations.

## Component/file ownership and ordered cuts

Component owners below identify implementation boundaries, not assignments made by this task.

| Cut | Exact outcome and dependencies | Likely files and owners |
|---|---|---|
| C1: one-seat inspection | One selected closed bboxy harness IR session, captured source binding, private bounded snapshot, explicit-root D0, durable paged summary/status/cancel and missing-evidence diagnostics. Include source-byte preservation and generation/ABA tests. No inference, trial, KB write, native sink or automatic run. Requires only authenticated source discovery and negotiated inspect capability; missing trust is a reported limitation. | Term protocol owner: new crates/bbt-proto/src/dream.rs. Term mux owner: new crates/bbt-mux/src/logic/dream.rs and src/mcp/dream.rs, registration in existing logic/MCP modules; source binding at src/adapters/launch.rs and src/adapters/observe.rs; private registry/store job/catalogue. Transport owner: new crates/bboxy450-harness-wake-up/src/dream.rs, reuse harness_log.rs identity. Harness owner: new src/dream/host.rs, explicit selected-input Resolver helper in src/dream/resolve.rs, dispatch in src/dream/mod.rs and src/frontends/headless.rs. Operator surface: new crates/bboxy450-term/src/ctl/dream.rs. Proposed private tests crates/bbt-mux/tests/dream_term.rs and harness tests/dream_host.rs; Make and model registrations on both sides. |
| C2: manual proposals | Select one repo/cluster; prepare bounded trusted input; run a configured engine Provider with no tools; write lawful Pending/Proposed IR and retained usage; show exact unmet publication/trial gates. Requires provider authorization, spend allocation and independent trusted anchors. Never call a scripted CLI --profile flag a production scheduler. | Harness src/dream/{host,prompt,synth,trust}.rs, provider/profile.rs; term admission/usage adapter. Add explicit Git-backed evidence capability to captured knowledge routing. |
| C3: complete controlled loop | First complete fixture-backed loop: D0 → D1 → confined scripted trial → confirmed flush → approved lower publication → outer run receipt/seal/audit → next engine start with pinned reported Recall. Add observable child/effect accounting, durable original ticket recovery or explicit refusal, and request-shape translation for the actual knowledge_commit schema. Requires C2, exact source/main pins, supported Git provider, approved executable and owned test identity; tests never enroll or borrow the operator's live approval. | Harness src/dream/{host,trial,flush,lower,receipt,seal,recall}.rs and src/gateway/host.rs; term src/knowledge/{routing,effects,commit}.rs and src/mcp/knowledge.rs adapter. Existing src/engine/mod.rs Recall seam; shared private process tests. No new thresholds or trust shortcut. |
| C4: production engine adapter | Enable the first real engine profile only after its policy/capability, no external writes, environment/MCP isolation, held-out fork/verify, comparable usage and cleanup witnesses pass. Current non-scripted source rejection must be replaced by an explicit tested adapter capability. Until then real learning produces proposals/Inconclusive, not auto-adoption. | Harness src/dream/trial.rs, src/engine/{mod,turn_loop}.rs, src/tools.rs, src/process.rs and private adapter fixtures; term budget/evidence host. C3's scripted success does not prove this cut. |
| C5: same-repo swarm | Reuse C1/C2 for a frozen authorized member catalogue and one partition per repo, with separate output audience and no privilege inheritance. Swarm inspection can follow C1 independently; shared adoption requires C3/C4 plus supported adapters. Duplicate evidence unions by full pin, not seat count. Closed/removed/restarted members retain historical attribution without becoming current members. | Term registry/proto, logic/dream.rs, selected knowledge routing and swarm controls; harness receives manifests, not account-roster access. Existing rich_pane.rs and harness gui/face_rows.rs may expose shared result views; changes require existing parity gates. |
| C6: opt-in expansion | Native Claude/Codex exposure + separately proved native/ACP trial/loader support; non-Git immutable evidence adapter; cross-repo/system promotion; account policy; automatic schedule/quota triggers. These are independently gated extensions, not prerequisites for C1. Cross-repo/system still requires every contributing repo's committed opt-in. Grok/Proposal publication is currently unsupported. | Harness import/{mod,edge}.rs, dream/{recall,lower,resolve,trust}.rs, backend/acp_agent.rs; term knowledge/plugin contract and usage_strip.rs; per-object job alarms/policy. Additive IR/evidence revisions need their own current-source formal and compatibility evidence. |

First implementation brief should be C1 only, covering the new boundary and private test fixtures, with make test-dream-term and make -C crates/bboxy450-harness test-dream-host added as canonical focused gates. No implementation is delegated here. A harness source change lands in its own repository before term deliberately bumps the submodule through make bump-harness in a separately scoped integration cut; a planning task must not move it.

## Resource ceilings and allocation decisions

These are proposed acceptance ceilings, not measurements or an assertion that the existing global memory/inference budget has spare capacity.

| Resource | C1 allocation/ceiling | Later inference/trial rule |
|---|---|---|
| Live UI/logic | No source IO, parsing, provider wait or scan on render/logic owners. Admission/status handling capped at 1 ms CPU between yields. Preserve existing 4 ms p99 total frame allocation; Dream receives no new slice of its margin. | Background work only; request-context optional preflight has a 250 ms deadline and a no-memory fallback, not a delay on conversation send. |
| Input/heap | Up to 32 selected sessions, 64 MiB cumulative retained input including required blobs; reject before reading/allocation beyond cap. Parsed/result working set ceiling 128 MiB per job, monitored and explicitly fail/quarantine on overrun. Bounded selected-input Resolver needed because current Resolver scans its whole directory. | Existing native 16 MiB/10,000-record snapshot and 128 MiB custody ceiling remain; stricter job cap may omit such an import. Memory allocation for concurrent jobs must be measured and approved before automatic operation. |
| Scheduling/CPU | One existing blocking-work permit at a time per job; per-shard try-claim; at most two admitted inspection jobs per root, immediate Busy beyond that. Distinct roots/shards use independent owners and existing host resource admission; no new unbounded pool or global single runner. 30 s inspection wall deadline, then cancel/settle. | Trial arms serial inside their candidate; independent candidates continue subject to self-service resource shares. Host aggregate CPU/RSS share remains an explicit decision below. |
| Disk | At most 128 MiB new inspection staging/output per job; summary page ≤16 KiB, graph page ≤128 nodes. Admit materialization against this job's explicit staging/output budget and the actual applicable owning subsystem's admission check. Retain immutable hashes/status when a result is omitted. | Scratch allocation and retained-evidence quota must be supplied before trial start; no implicit clone/build allowance. Do not delete referenced IR/receipts on disk pressure. Existing user retention/never-delete policy controls reclamation. |
| D1 inference | Zero | One call/cluster; at most 3 proposals; existing 60,000-byte conservative input and 32 KiB output cap; caller ceiling ≤200,000 tokens per run; no tool execution or added retries. Missing Usage is conservatively estimated and labelled, never free. |
| Trials | Zero | Caller may tighten, never exceed current 2,000,000 tokens/entry, 30 min turn and 10 min verify ceilings; current 4 held-out episodes, 2 runs/arm and 2 controls unchanged. Heavy verifiers need an available existing host share; otherwise Inconclusive. |
| Quota | Not required for deterministic work | Automatic inference requires chosen-provider observation ≤600 s old, valid integer percent 0..100, non-stale worst window, reset within 3 h, used/100 <0.80; stop admitting more at ≥0.90. Recheck between jobs; already admitted work obeys its cap. Current global sampled_at is insufficient provider freshness. Add per-provider observed_at or refuse automatic admission. |
| Transport | Control/status frames ≤256 KiB, result pages ≤16 KiB, bounded 32-entry per-job event lane; no raw corpus in frames | Negotiate smaller limits; dropped viewer events recover through retained sequence/status, not a second action. Cap actual hidden/failed usage and retained output too. |

Disk-source discrepancy, review correction: .bboxy/specs/budgets.md:58 lists a 10 GiB free-space floor, while current LAND.md:58-60 gives recovery its own 128 MiB plus 1 MiB/seat reserve and explicitly rejects a blanket 10 GiB free requirement. The older budget row does not establish a Dream product gate. Use the Dream job's explicit materialization budget above together with the actual applicable owning subsystem's admission check; LAND's recovery reserve does not automatically apply to Dream. Defer only the affected materialization when its budget or admission check fails. This correction does not modify either source document.

Operator and schedule triggers never bypass an unavailable inference budget. Every inference start requires a current admitted provider allocation; absent required quota/capacity evidence yields a named skip, while deterministic work continues.

Decisions still required before paid or automatic operation: aggregate Dream CPU/RSS/inference share within [[specs/budgets]] (its existing memory split/inference rows are gaps); scratch/retention allocation for real trials; explicit allowed provider/profile/spend/audience policy; independent producer/trust enrollment; per-provider freshness source; native loader contracts; non-Git immutable evidence proof. Missing decisions disable that stage with a named reason, while C1 and conversations remain usable. No agent or memory may choose its own capacity or approval.

## Verification design and matrix

All checks in this section are future requirements. No model, unit, integration, smoke, e2e or product build ran in this planning task. The requirements become executable invariants when their named targets/models exist and pass; git diff --check alone does not prove them.

Use separate small models rather than one intractable whole-product cross product. Common finite domains: two seats/two swarms, two repos/providers, two shards and operations; incarnations A1 → B2 → A3 (epochs never reused); zero/one child/effect; outcomes success/failure/unknown; capacity 0..2. Initially no jobs/effects/children/claims, zero spend and no sealed result, current context A1. Host owns admission/context, harness worker owns phase/effects, process owner owns settlement, provider owns immutable revision responses. Atomic steps are Admit, Claim/Busy, ObservePrefix, BeginEffect, CommitEffect, Cancel, ChangeContext, Complete, Settle, Seal and Present; external IO, fsync, crypto and code execution are not atomic abstractions.

Safety: no effect/visibility outside captured authority, no duplicate effect, no stale completion/adoption, no seal with active/unknown child/effect, no clobber, no spend-cap bypass, no instruction promotion. Progress: every admitted operation eventually produces a result or explicit failure/quarantine, and another enabled shard can progress while one stalls. Assume weak fairness only for internal cancellation/timer/status/settlement steps, not successful provider/network/OS IO. A timeout is a real process witness; finite fairness is not a wall-clock proof. Reachability assertions must exercise the advertised interleavings, including ABA and unknown outcomes.

| Requirement / interface | Model + deliberately broken control | Real API/process witness and canonical gate |
|---|---|---|
| R1 / D1-D2 scope and exact sources | Proposed TermDreamScope; remove membership or incarnation check | Two private swarms with same cwd/slug, inaccessible peer, closed/restarted seat, wrong blob/digest/torn source, explicit import; source bytes unchanged. Proposed make test-dream-term + test-dream-host. |
| R2 / D1-D3 nonblocking independence | Proposed TermDreamAdmission; global-lock/wait-on-busy controls | Hold one private worker/source/KB read and fill its lane; another shard and existing message/input path still complete within explicit test deadlines. Per-root cap returns Busy. Focused target plus make test-boundaries and make test-workspace-boundary. |
| R3 / D3-D4 duplicates/restart | Proposed TermDreamRunIdentity + extend HarnessDreamFlush; replay-unknown/new-ID and lost-journal controls | Drop reply after effect, restart logic/worker, same-op/different-body, root/provider A→B→A; assert one actual write/commit and original-op queries only. Private subprocess + provider fixture in focused target. |
| R4 / D6-D7 cancellation/settlement | Proposed TermDreamSettlement plus HarnessDreamTrialSettlement, HarnessDreamPublication, HarnessDreamSeal; early-release/early-seal/lost-child controls | Delay owned spawn/reap/fsync, cancel/drop/kill around publication, retain quarantine on unproved cleanup, no late write after seal. Extend tests/dream_{pipeline,seal,sinks}.rs; make test-dream-host and both formal parity gates. |
| R5 / D4-D5 provider/trust | Proposed TermDreamEvidence; etag-as-commit, unsigned-anchor and stale-provider controls; retain existing Dream trust/lifecycle models | Disposable signed Git/KB identities with allowed-main/not-main/revoked/wrong-root/digest/lossy controls; external fake provider lacking immutable history never falls back locally. Extend dream_trust.rs/dream_sinks.rs and term provider tests. make test-kb-reports and focused target. |
| R6 / D3,D6 budgets/quota | Proposed TermDreamBudget; fraction-unit, global-timestamp freshness, over-cap/new-job controls | Fake clock/provider samples at 79/80/90/100%, stale/null/future/invalid; enforce input/heap/output/disk/turn caps; retain failed/cancelled usage; unavailable heavy resource returns Inconclusive. Focused target, no live account. |
| R7 / D5-D7 learning/provenance | Existing HarnessDreamLifecycle/Merge/TrialOutcome/TrialUsage/TrialFrontier/Seal, extended for host sequencing; wrong-pair/stale-revival/contaminated-evidence controls | Full private C3 loop with exact Event/Record/Trial/Publication/Run pins; P1-P9 negatives; C4 real-profile isolation witness before enabling that adapter. Existing dream_pipeline.rs, dream_audit_broken.rs plus new host e2e. make test-dream-host and make test-ir if schema changes. |
| R8 / D8 audience, Recall, authority | Proposed TermDreamAudience + existing HarnessDreamRecall; foreign-audience, stale-generation, missing-append and trial-leak controls | Capture actual engine Provider and private ACP agent outgoing messages: quoted user/reference role, exact digest/pins, durable Recall before send, append failure omits context, malicious wrappers neutralized, startup proceeds when loader fails. Existing dream_recall.rs extended; make test-rich-launch-context for term path. |
| R9 / D2,D8 native custody/exposure | Existing HarnessDreamNativeCustody + scope model; premature-pin/wrong-root/timeline controls | Claude/Codex fixtures import twice with same retained pins, crash before complete publication, source grows mid-snapshot, no timeline/no invented Recall; native EOF explicitly labelled. Existing dream_replay.rs/dream_recall.rs, focused host target. |
| R10 / D7 no clobber | Existing HarnessDreamSink/Publication; check-then-overwrite and wrong-temp/ABA cleanup controls | Actual concurrent private hand writer after Check, duplicate publishers, differing existing target/index, symlink/short-name collision, disk/sync failures; byte-preserve foreign files. Existing dream_sinks.rs, test-dream-host. |
| R11 / D9 status/presentation | Proposed TermDreamViewFence; stale-completion and inspection-as-adoption controls | Retained job replay with reordered/duplicate events, root A→B→A, revoked result read, bounded page/cursor and unavailable provider. Native UI capture only when a view is added; existing make test-rich-pane/parity-swarm or affected parity gate then applies. |

Register proposed models in term crates/bbt-mux/src/models.rs (new focused submodule), verification/{corpus,coverage,contracts}.toml, and in harness tests/formal/models.rs / verification mappings where harness state changes. Use constrained Yarn/TLA+ plus native Rust checker and pinned Java TLC to compare full reachable states, labelled transitions, safety/deadlock/liveness verdicts; unsupported/incomplete/timeout/disagreement fails. Give every broken control a causal failure assertion and real implementation counterpart. Hashes, arbitrary JSON/Unicode, cryptography, syscall races, actual process reaping, usage arithmetic and resource timings remain implementation witnesses.

Future gates: focused Make targets above first; term make test-formal and make test-formal-contracts; harness make test-formal-parity (its make test-formal alone is native), make test-ir and check-embedded-build when affected; term make test-boundaries/test-swarm for process/roster changes, make test-harness-native/test-rich-launch-context for actual launch/Recall, and make smoke using private unsigned homes/identities/scripted providers. make land is the authorized landing interface after implementation; full make test/land-full is reserved for the repository's release/toolchain/lockfile or explicit-wide conditions. Add missing focused smoke/e2e workflows to Make; never require operator clicks, Keychain, credentials or a live inference account for verdicts.

## Acceptance and exclusions

C1 is accepted only with a current-source scope/budget/lifecycle model and broken controls, a real private subprocess-to-mux-to-status witness, exact source/hash preservation, independent-shard/input progress, restart/ABA/cancel tests and explicit unavailable-evidence outcomes. C3/C4 adoption is accepted only with complete independently approved source-to-Recall proof, not a sequence of stage success reports.

Out of scope for the first cut: autonomous schedules, model selection/spend policy, global corpus crawling, peer recruitment, production native sink writes, instruction/skill/policy generation, cross-account/remote synchronization, automatic trust enrollment, general non-Git trust equivalence, automatic merges/releases and UI redesign. None is implied by the presence of Dream library APIs.
