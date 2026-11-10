---
type: research
title: Current dreaming systems and a useful bboxy MCP cut
children:
- '[[research/dreaming-services-mcp/papers]]'
- '[[research/dreaming-services-mcp/harness-capabilities]]'
id: 01M6P186SWX485JK2NHXTAGVAZ
updated: 2026-11-05T14:50:56.413Z
created: 2026-11-05T14:19:21.646Z
number: 93377961364654
created_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
updated_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
authorship:
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  kind: seat
  public_key: L62ggzXO9nS9icBZrd6nPN71BnmasbyzYyfTJE3eu0k=
  process_generation: '1'
provenance:
  digest: sha256:c76274f8c04b282ed5307560dd2d896ce271c9917fd7077a8532a1f33eaa7350
  frontmatter_digest: sha256:d78172d83b1f3b958da6c62b02bde6230c098bd06ac515c541616e66587ab2fa
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  sig: 0tmNWRa2dex8Vy3m54xmf7Ob/MHmN3fOod/C8zaFDtY/WZ4InAczDKGNP/p9/oHzypw1gMIqAdAvavEuilFDCA==
  log: .bboxy/log/2026-11-05.jsonl#136922
---
# Expose prepared evidence before adding consolidation

Build **bounded read tools over prepared local evidence first**. The strongest shared pattern is offline preparation plus small scoped retrieval, not an always-running memory writer. Our first useful slice can expose completed C1 observations honestly; verified lesson retrieval follows existing receipt/trial/publication checks. Current vendor APIs provide useful interface patterns, but none proves bboxy's authority, source closure or responsiveness. Existing C1 remains an independent implementation dependency. This research retrieved primary sources on **2026-11-05 UTC**; documentation publication dates are not stated unless given below. No service, account or model was invoked.

## Services offer different products under “memory”

| Product / availability verified in public docs | API / CLI / lifecycle actually documented | Scope, currentness and cost control | Decision-useful boundary |
|---|---|---|---|
| Anthropic Managed Agents **Dreams: research preview, access gated**, not a general Claude Code feature | Async dream takes an existing store plus 1–100 sessions, returns a separate output store; SDK/cURL creation, status/cancel/archive documented; two beta headers required | Inputs unchanged, incomplete output may survive failed/cancelled jobs; usage billed at chosen model's token rates. No hard per-dream dollar ceiling established here | Copy-on-write output and explicit adoption are useful patterns. Do not confuse completed curation with our verified publication. Exact CLI command not validated. ([Dreams](https://platform.claude.com/docs/en/managed-agents/dreams), n.d., retrieved 2026-11-05.) |
| Codex local memories: **documented, opt-in local feature**; separate from ChatGPT web/Work memory | /memories and configuration control reading/generation. Background extraction skips active/short sessions and has idle/quota controls; generated local files are state, not an editing API | MCP/web context can be excluded from generation; model/rate-remaining settings control spend. No generic hosted Dream REST endpoint was established | Keep required policy in AGENTS.md; do not write Codex's private memory directories as our MCP integration. Its idle trigger is not our SessionClosed law. ([Official OpenAI memories](https://learn.chatgpt.com/docs/customization/memories), n.d., retrieved 2026-11-05.) |
| Letta memory/dreaming: **current CLI/app documentation** and separately documented API parameter | MemFS is Git-backed; /remember, /memory, /sleeptime configure use; CLI dreaming can trigger after steps/compaction; API enable_sleeptime moves memory management to a background agent thread | Optional agent review costs another model conversation. Git history is reviewability, not our signed trial/publication proof | Useful explicit memory controls; do not equate the SDK parameter with every MemFS CLI behavior. Its live step/compaction triggers are excluded here. ([Memory](https://docs.letta.com/configuration/memory), [API update](https://docs.letta.com/api/python/resources/agents/methods/update), n.d., retrieved 2026-11-05.) |
| Mem0: **hosted Platform with dedicated Dream REST; separate documented OSS stack** | Project Dream config/update, no-write scope preview, synthesis runs/activity and synthesized-memory source APIs are public docs, alongside generic V3 add/search/CLI | Org/project-scoped API key; enabled/mode plus plan/interval entitlements. Actual plan access/pricing were not exercised; documented page/scan caps are not hard spend caps | Use explicit scope, lifecycle/lineage and coverage. Dedicated Dream availability must not be reduced to generic add; OSS feature parity is unestablished. See the verified API detail below. |
| Zep hosted context / Graphiti OSS: **distinct documented surfaces** | Graphiti incrementally builds temporal graphs; MCP add/search/manage/status. Zep has context MCP and explicit batch/backfill APIs; zep-ingest is a documented batch wrapper | Temporal validity/invalidation and source provenance; MCP project is selected by authenticated connection, not request argument. LLM/database/backend costs still exist; no comparable hard cost budget verified | Separate ingestion from retrieval, and preserve valid/invalid time. Do not mistake extracted graph facts for adjudicated lessons. ([Graphiti](https://github.com/getzep/graphiti), [MCP](https://help.getzep.com/graphiti/getting-started/mcp-server), [Zep MCP](https://help.getzep.com/v3/context-mcp-server), [batches](https://help.getzep.com/v4/adding-batch-data), n.d., retrieved 2026-11-05.) |
| Cognee: **current remember/recall/improve; older memify deprecated** | improve enriches an existing authorized dataset, bridges sessions/feedback, reports skipped stages; background mode and cognee-cli improve --background documented. Default memify now does nothing without custom tasks | Dataset write authorization, conflicts/currentness and optional feedback ranking. Some stages skip without an LLM; dataset background work can queue behind other pipelines | Honest skipped/Pending results are useful. Do not copy the per-dataset serialization as a machine-global queue, or recommend deprecated memify. ([Improve](https://docs.cognee.ai/core-concepts/main-operations/improve), [legacy](https://docs.cognee.ai/core-concepts/main-operations/legacy-operations/memify), [CLI](https://docs.cognee.ai/cognee-cli/overview), n.d., retrieved 2026-11-05.) |
| LangMem: **public library/SDK documentation**, not verified here as a hosted service or current release guarantee | create_memory_store_manager and ReflectionExecutor implement background extraction; manage/search tools use a configured store/namespace. Delayed processing cancels/reschedules per thread | App chooses model/store, persistence and debounce; delay reduces repeated work but is not a hard spend cap | Use namespace separation and explicit scheduling. Its inactivity debounce cannot substitute for authoritative closure. ([Background](https://langchain-ai.github.io/langmem/background_quickstart/), [tools](https://langchain-ai.github.io/langmem/guides/memory_tools/), [delay](https://langchain-ai.github.io/langmem/guides/delayed_processing/), n.d., retrieved 2026-11-05.) |

These are documentation-backed capabilities, not live-access validation, security certifications or comparable benchmark results. No provider integration is selected; none is required for the proposed local subset. License/origin checks precede implementation reuse; citations are research/inspiration, not copied implementations.

## Mem0 has a dedicated project Dream API

Correction after initial review: the generic V3 add API is not the full dreaming surface. These official public pages were fetched2026-11-05; publication dates are not stated. Paths below share /api/v1/orgs/organizations/{org_id}/projects/{project_id}/dream/ and documented API-key authorization. No request to api.mem0.ai was made.

| Documented endpoint / source | Verified scope, effect and limits |
|---|---|
| GET config/ — [Get Dream Configuration](https://docs.mem0.ai/api-reference/dream/get-dream-config) | Reads project configuration and plan entitlements: reflection_enabled, reflection_enabled_at, reflection_mode; plan, reflection_interval_days and features. Example values are schema illustrations, not verified quotas or proof that every plan has access. |
| PATCH config/ — [Update Dream Configuration](https://docs.mem0.ai/api-reference/dream/update-dream-config) | Changes project background-synthesis enablement and mode (batch or direct). This is an effectful control; GET config is not a trigger. The fetched schema does not define the detailed execution semantics of those mode names. |
| POST preview/ — [Preview Dream Scope](https://docs.mem0.ai/api-reference/dream/dream-preview) | Explicitly a no-write preview, not a synthesis run or admission guarantee. Returns active_total, scanned, cap, eligible_users and note; scanned is capped recent coverage. A missing numeric universal cap is not an unbounded guarantee. |
| GET runs/ — [Synthesis Runs](https://docs.mem0.ai/api-reference/dream/get-dream-runs) | Newest-first keyset pages, limit default20/max100; run_id/at/count, synthesized memories and their sources, has_more/mem_cursor, next_cursor. A list of prior runs is not a documented create/cancel-job contract. |
| GET runs/{run_id}/memories/ — [Run Memories](https://docs.mem0.ai/api-reference/dream/get-dream-run-memories) | Synthesized-memory pages within one run; limit default50/max200, opaque cursor from mem_cursor/next_cursor. Results include memory ID/text and source IDs/text. |
| GET activity/ — [Dream Activity](https://docs.mem0.ai/api-reference/dream/get-dream-activity) | Project supersede/merge feed, newest-first keyset pages, default50/max200. Entries include transition/at, replaced_by and sources; useful lifecycle evidence rather than an immutable signed audit proof. |
| GET memory/{memory_id}/sources/ — [Synthesized Memory Sources](https://docs.mem0.ai/api-reference/dream/get-dream-memory-sources) | Returns memory_id, synthesized and source-memory ID/text/lifecycle_state. This is explicit synthesis lineage; it does not attest original IR bytes, trial outcome, outer publication seal or every source's continued truth. |

Availability is **documented hosted APIs with plan-dependent entitlements**, not a live-access test, an asserted GA/preview classification or OSS parity. The fetched pages do not establish a Dream-specific CLI command, caller-defined per-job hard spend cap, universal cadence/price, cancellation/settlement/idempotence semantics or authoritative session-closure enforcement. Existing [generic memory add](https://docs.mem0.ai/api-reference/memory/add-memories), [mem0 search CLI](https://docs.mem0.ai/platform/quickstart) and [OSS documentation](https://docs.mem0.ai/open-source/overview) remain distinct surfaces. No inference is made from an example value123. This adds a relevant comparison pattern—configuration separate from preview, bounded activity/run pages and synthesized-source lineage—without selecting a service or expanding the proposed bboxy endpoints.

## Original research favors small, testable memory units

[[research/dreaming-services-mcp/papers]] records original sources, version dates, controlled-evidence limits and inferences. Sleep-time Compute and LightMem support moving reusable work offline; ExpeL and ACE motivate contrasting experience and incremental lessons; LongMemEval makes temporal updates and abstention test dimensions. MINJA and the 2026 poisoning study show that repeated interaction/feedback can corrupt memory and that defenses must not pass by rejecting everything. These findings motivate tests, not claims that our system achieves their reported scores. Costs must include offline preparation and failed/aborted jobs.

The engineering choice is to preserve an immutable evidence unit and its lifecycle instead of a global editable advice blob. A useful result answers “why this applies, what verified it, what invalidates it, and which snapshot was read.” Current code already contains many of these checks, but their presence as library functions is not an exposed production workflow.

## The local source requires a staged tools-first cut

[[research/dreaming-services-mcp/harness-capabilities]] gives exact current revisions, hashes, file:line evidence and the official MCP/client matrix. Main exposes no Dream endpoint; C1's own-seat dream_c1 is in a moving author branch. Engine imports model tools, outer resources currently expose skills, and ACP relays the same admitted server. Ordinary gateway starts do not construct verified Recall Context; production synthesis/trials and provider-independent accepted-task evidence remain gaps.

[[specs/dreaming-mcp]] therefore selects two read tools, dream_search and dream_read, plus a later reported-feedback tool. Existing dream_c1 remains the inspection/status/cancel route. First deliverable: investigate literal metadata, exact fingerprints and recurrence in own-seat closed-source observations. Bind every hit to an immutable result page/row; only Episode has an event origin, while cross-session Cluster membership may be omitted with explicit coverage. No symptom-language matching from hashes or reusable fix is claimed; Q2/Q3 supply lesson-text guidance. Second: prepare eligible already-published lessons through bounded selected inputs, existing audit/receipt/trial checks and explicit currentness. Third: connect prepared verified reads and reported/cited feedback without promoting that feedback or claiming model consumption from successful transport.

The MCP baseline stays ordinary tool results with bounded structured JSON/text fallback. Resources/prompts and redesigned protocol tasks are optional enhancements after actual negotiation; existing C1 run IDs already provide a nonblocking fallback. No new Dream run may be admitted from idle/TurnEnded/native EOF; heavy preparation/consolidation stays after authoritative closure. Fast reads of existing context during a live task remain allowed.

## Measure useful work, including the failed cases

Proposed validation uses eight private coding scenarios spanning repeat failure, changed dependency, stale lesson, no answer, poisoned feedback, incompatible harness, foreign repo and worker saturation. Record with/without prepared context: successful unattended task outcomes, repeated failed commands, useful citations, false-advice/stale/foreign exposure, model context cost and foreground latency. Require zero foreign/stale promotion and a useful positive in addition to poison rejection. Initial scripted scenarios establish mechanisms; any model-quality benefit needs a separately authorized private provider evaluation and a full cost ledger. There are no measured benefit claims in this report.

No live signup, service/provider call, spend, installation, credential access, source edit or test/build occurred. The actual unassigned drafts linked by the spec are preparation for Root selection, not delegation or changes to current C1 briefs.
