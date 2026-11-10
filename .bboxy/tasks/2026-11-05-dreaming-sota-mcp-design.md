---
type: task
title: Research current Dreaming systems and design useful nested-harness MCP access
allowed_paths:
- research/dreaming-services-mcp.md
- research/dreaming-services-mcp/vendor-apis.md
- research/dreaming-services-mcp/papers.md
- research/dreaming-services-mcp/harness-capabilities.md
- specs/dreaming-mcp.md
- interfaces/dreaming-mcp.md
- tasks/2026-11-05-dream-mcp-*.md
- reports/2026-11-05-dreaming-sota-mcp-design.md
verify: git diff --check
id: 01M6P0K2WNQAT187215T2P8V3J
status: accepted
delegated_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-05T14:53:56.828Z
created: 2026-11-05T14:07:49.498Z
number: 93377961364646
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
  - criterion: Correct episode/cluster row provenance and bounded membership/coverage
    reference: interfaces/dreaming-mcp.md — ObservationProof/ResultRowPin/ClusterMembers; Two-session aggregate conformance case
  - criterion: Concrete Q1 schema/model/process controls and honest investigation-only benefit
    reference: specs/dreaming-mcp.md; tasks/2026-11-05-dream-mcp-observation-reads.md
  - criterion: Mem0 dedicated Dream primary sources; precise relay and MCP Tasks boundary
    reference: research/dreaming-services-mcp.md; research/dreaming-services-mcp/harness-capabilities.md; interfaces/dreaming-mcp.md
  - criterion: Append-only superseding report, full readbacks, source hashes and actual check
    reference: reports/2026-11-05-dreaming-sota-mcp-design-review.md
report:
  summary: 'Corrected the rejected observation proof: all rows bind exact C1 manifest/descriptor/page hashes and row index; only episodes retain EventPin origins, while clusters have bounded optional membership and explicit omitted coverage. Updated ICD/spec/Q1 controls and a two-session aggregate fixture prescription, with no C1 wire/source change or source rereplay. Q1 is literal metadata/fingerprint investigation; Q2/Q3 supply reusable lesson guidance. Corrected embedded-harness ACP relay paths and exact current MCP task negotiation. Fetched and added Mem0 dedicated Dream config/update, no-write preview, run/activity and source-lineage APIs with plan/limit uncertainty. All five updated artifacts and superseding report fully read back; git diff --check exited0, main342faefd and embedded harnessa2ca8f39 unchanged. Q1 remains open/unassigned; no tests/builds/service/account/credential/implementation actions.'
  commit: null
  reported_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  op_id: 01M6P35BHQ6VPDSPDR5BF1MT6C
  verify_revision: 0
  report: '[[reports/2026-11-05-dreaming-sota-mcp-design-review.md]]'
rejection:
  reason: 'Research and staged tools-first direction are useful, but fix one implementation-blocking proof mismatch before acceptance. interfaces/dreaming-mcp ObservationProof requires source:EventPin for both episode and cluster. Actual producer src/dream/c1_wire.rs:359-375 gives an origin only to Episode; Cluster aggregates repo/fingerprint/count/times, and d0.rs:950-1001 can merge episodes across selected sessions. One event pin cannot certify an aggregate and no actual cluster event exists. Keep C1 wire unchanged: define exact result-page+row provenance for every observation (manifest/result/page digest and row index or equivalent canonical binding), episode-only origin nullable/typed variant, and bounded optional member links/coverage if offered; do not fabricate origins or imply one representative proves all members. Update Q1 draft/schema controls and concrete two-session aggregate example demonstrating tamper/membership/omitted-coverage behavior, without source rereplay or raw transcript export. Clarify Q1 is metadata/fingerprint investigation and Q2/Q3 supply reusable lesson guidance; no symptom-language matching from hashes. Also correct report''s malformed source-path attribution for ACP relay and precise task-extension negotiation wording against official current spec if needed. Append-only superseding report, full readbacks, same git diff --check. No source/code/new implementation or C1 scope changes.'
  by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
provenance:
  digest: sha256:7f41898026ed035fe27b440db2b002223c03cd8b4e5cae88befd7fe8a334cf3b
  frontmatter_digest: sha256:1234fd6faef09a7196429124a2866b02e47d886a2de1d8a501b2153b929633f1
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  sig: lFg8QgjqIYCYYZd3ufH+1DURzq5LtfpEcFA0Hliydt6TBbNVl6m7WgUSjpsFTugBtNSljhrQ7OPqJNnXvyB2Bw==
  log: .bboxy/log/2026-11-05.jsonl#136991
---
ROLE: ARCHITECT

## Operator request
“research SOTA for dreaming and dreaming apis services and clis, then start writing up a task about exposing parts of our dreaming system via MCP inside of harnesses inside of bboxy MCP, implement best ways harnesses can benefit from our dreaming system!”
Operator explicitly delegates research and MCP design to this Architect harness. Produce current primary-source research, a grounded minimal endpoint design, and actual unassigned implementation-ready task records. Root will review/select/delegate implementation; do not change the running C1 authors' scope.

## Scope and workflow
Selected knowledge root remains the term repository; use orient/knowledge_orient and knowledge tools for all records. Working dir /Users/feral/mydev/bboxy450-term. Read-only source access to current main, embedded harness, and both authorized C1 author worktrees (/Users/feral/mydev/bboxy450-harness-dream-c1 and /Users/feral/mydev/bboxy450-term-dream-c1). Capture fresh Git/date: root main has moved to 342faefd (provenance/control work), producer was f5838ec4; do not trust earlier baselines or confuse dirty in-flight code with shipped behavior. Existing C1 work continues independently.
No source edits, builds, live provider/service calls, signup, spend, credential/account access, install, plugin change, merge/publish, task delegation or recruitment. Use this single Architect harness to research and synthesize, respecting scarce shared CPU. Create only allowed KB artifacts/tasks through knowledge_write; task records are open/unassigned drafts with exact paths/checks, not peer assignments. No ordinary chat/cloud Artifact tool.

Research workflow: /Users/feral/.agents/skills/synced/08a1cf3b-109b-4808-b76e-23a64a2cd2f3_d6008890-49fa-4f6a-b119-fd7e4e4c99f9/deep-research/SKILL.md is used by the root coordinator. Read its references/researcher.md before research and references/report-writer.md before final synthesis as applicable; follow the user's established KB/task format instead of new ad-hoc output directories or extra seats. For OpenAI comparisons read /Users/feral/.codex/skills/.system/openai-docs/SKILL.md; use current official documentation/source order and actually fetch supporting pages, with uncertainty explicit. Do not load optional persona/stance skills.

## Research
Explicit web browsing is REQUIRED. Search/fetch actual primary sources as of the current date; use official vendor docs/SDK/CLI/repos and original papers. Do not turn old local research, search snippets or vendor marketing into verified features/benchmarks. URLs/publication/retrieval dates and precise supported claims belong beside findings; follow quote limits. Research notes may form the allowed parent/children tree if useful.
Cover:
1. Current agent dreaming/offline consolidation and memory services/APIs/CLIs: verify candidate Anthropic managed-agent Dreams, Codex memories, Letta sleep-time/memory, Mem0, Zep/Graphiti, Cognee/LangMem or comparable maintained systems. These are leads to verify, not claims they exist/support a named feature. Distinguish hosted vs local, active vs preview/planned, automatic vs explicit, public API/CLI vs hidden product behavior, provider/harness-specific limitations, provenance/temporal invalidation, asynchronous jobs, retrieval, feedback and actual cost controls. Do not sign up/use private endpoints.
2. Original research on sleep-time compute, experience consolidation, contrastive/verified lessons, memory evaluation and poisoning/feedback loops. Select relevant current work rather than a huge survey; separate controlled evidence from claims. Give useful mechanisms/negative lessons for our IR-backed implementation.
3. Official current MCP tools/resources/prompts/async-task patterns and current coding-harness support (bboxy engine/ACP, Claude Code, Codex, other configured profiles only when relevant). Identify actual nested-client/server flow and protocol/feature compatibility from our source. No assumption that all clients support MCP tasks or resources equally.

## Design and deliverables
Write research/dreaming-services-mcp.md with a concise comparison table and source-backed synthesis; optional child notes for evidence. Write specs/dreaming-mcp.md and interfaces/dreaming-mcp.md for the smallest high-value tool surface:
- How an agent benefits from current verified memories/lessons/workflows while solving a task; discovery/search/retrieval, why/evidence/lineage, currentness/invalidation, feedback/citation/Recall attribution, and inspection/status/cancel/run requests only where justified.
- Separate fast reads of prepared verified context during a live session from HEAVY Dream consolidation, which the operator requires ONLY AFTER authoritative session closure. No idle/turn-end/native EOF trigger, no live inference/consolidation, no auto-promoting tool/web evidence to policy.
- Ground each proposed endpoint in currently reachable library/runtime APIs and existing C1 control names; clearly label missing prerequisites (production synthesis/trials, publication/outer seal/trust, provider evidence bridge, etc). A usable subset must not wait for every future stage. Avoid a large speculative endpoint catalogue.
- Exact names/schema/examples, tool vs resource choices, role/capability/own-seat/swarm/repo/account scope, dependency/user-consent source where real, output audience, immutable source/receipt/revival/currentness pins, bounded paging/ranking/token/context budgets and honest unavailable/Pending/Inconclusive/error semantics.
- Nonblocking cached/prepared reads and admitted background jobs; keep CPU/duty/heap/control/pause/closure/settlement/ABA requirements from current C1 law. Optional MCP async tasks only with negotiated support and working fallback. No synchronous Git/KB/provenance/custody scan on tool/UI/logic owners. No prompt spam or model wakes for progress.
- Actual outer bboxy MCP -> managed harness MCP client/provider/native/ACP paths, avoiding circular self-calls or duplicate truth stores; standalone behavior and provider-pluggable KB compatibility. Show concrete agent workflow examples and expected measurable benefit, including failed retrieval/stale memory/poisoned feedback.
- Preserve authentic current public authority without credential custody probes (current decision d93377961364581), existing licensing/origin and privacy/cross-project fences.

Create a SMALL number of implementation-ready task records via knowledge_write under tasks/2026-11-05-dream-mcp-*.md, with type task, native open status, links to research/spec, ROLE: EXECUTOR header, own worktree placeholder/clear component split, exact allowed_paths, concrete acceptance checks, canonical Make commands, formal model/broken-control/real unattended witnesses and explicit dependencies. Do not delegate them or pretend a proposed target exists. The first task should implement a useful bounded subset against real capabilities, with no fake adoption/Recall/trust claim. Root will amend exact worktree/owner when assigning.

## Acceptance and handoff
- [ ] Current multi-source vendor/API/CLI and paper/MCP/harness evidence fetched and cited; availability and claims bounded honestly.
- [ ] Minimal endpoint contract and concrete harness workflows grounded in actual source with typed schemas/scope/currentness/latency/failure/authority.
- [ ] Actual native draft task records are actionable, scoped, staged and verifiable, preserving the ongoing C1 work.
- [ ] Knowledge artifacts/readback locators, exact citations/source hashes, actual git diff --check exit and research limits reported; no code/service/spend changes.
Run git diff --check only; research is read-only. Create append-only report with task, commits: [], actual verify command/exit, locators and evidence per criterion. task_report once. Important source/interface findings may go directly to the current authors within their briefs, but no scope/role overrides or routine status polling. Keep the final synthesis readable and decision-useful, not a raw scrape or a catalogue of every memory product.

## Report from seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR

Completed current primary-source vendor/API/CLI, original-paper and MCP/harness research, with source hashes and shipped-versus-in-flight boundaries. Proposed a bounded dream_search/dream_read observation cut, then already-published verified snapshots and reported-only feedback; heavy work remains post-SessionClosed under current C1 law. Created/read back the research tree, spec, ICD and three native open/unassigned Executor task drafts with exact scope/dependencies and future formal/broken-control/process gates. git diff --check exited 0; main 342faefd57e697d32713bd8ed6cd0a99def539ad and embedded harness a2ca8f393630081b6be583151293ae286dc2419e unchanged. No source changes, builds/tests, service/account use or delegation. Remaining dependencies are accepted C1 integration and qualifying publication/provider proof for real lesson availability; full evidence/digests are in the append-only report.

```json
{
  "op_id": "01M6P261Y09WQATHYNAK0QD0MF",
  "commit": null,
  "report": "[[reports/2026-11-05-dreaming-sota-mcp-design.md]]",
  "verify_result": {
    "command": "git diff --check",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Current primary-source research and bounded availability claims",
        "reference": "research/dreaming-services-mcp.md; research/dreaming-services-mcp/papers.md; research/dreaming-services-mcp/harness-capabilities.md"
      },
      {
        "criterion": "Typed minimal endpoint design and grounded harness workflows",
        "reference": "specs/dreaming-mcp.md; interfaces/dreaming-mcp.md"
      },
      {
        "criterion": "Native staged open/unassigned implementation drafts",
        "reference": "tasks/2026-11-05-dream-mcp-observation-reads.md; tasks/2026-11-05-dream-mcp-verified-snapshot.md; tasks/2026-11-05-dream-mcp-verified-feedback.md"
      },
      {
        "criterion": "Actual verification, revisions, full readbacks and exclusions",
        "reference": "reports/2026-11-05-dreaming-sota-mcp-design.md"
      }
    ]
  }
}
```


## Report from seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR

Corrected the rejected observation proof: all rows bind exact C1 manifest/descriptor/page hashes and row index; only episodes retain EventPin origins, while clusters have bounded optional membership and explicit omitted coverage. Updated ICD/spec/Q1 controls and a two-session aggregate fixture prescription, with no C1 wire/source change or source rereplay. Q1 is literal metadata/fingerprint investigation; Q2/Q3 supply reusable lesson guidance. Corrected embedded-harness ACP relay paths and exact current MCP task negotiation. Fetched and added Mem0 dedicated Dream config/update, no-write preview, run/activity and source-lineage APIs with plan/limit uncertainty. All five updated artifacts and superseding report fully read back; git diff --check exited0, main342faefd and embedded harnessa2ca8f39 unchanged. Q1 remains open/unassigned; no tests/builds/service/account/credential/implementation actions.

```json
{
  "op_id": "01M6P35BHQ6VPDSPDR5BF1MT6C",
  "commit": null,
  "report": "[[reports/2026-11-05-dreaming-sota-mcp-design-review.md]]",
  "verify_result": {
    "command": "git diff --check",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Correct episode/cluster row provenance and bounded membership/coverage",
        "reference": "interfaces/dreaming-mcp.md — ObservationProof/ResultRowPin/ClusterMembers; Two-session aggregate conformance case"
      },
      {
        "criterion": "Concrete Q1 schema/model/process controls and honest investigation-only benefit",
        "reference": "specs/dreaming-mcp.md; tasks/2026-11-05-dream-mcp-observation-reads.md"
      },
      {
        "criterion": "Mem0 dedicated Dream primary sources; precise relay and MCP Tasks boundary",
        "reference": "research/dreaming-services-mcp.md; research/dreaming-services-mcp/harness-capabilities.md; interfaces/dreaming-mcp.md"
      },
      {
        "criterion": "Append-only superseding report, full readbacks, source hashes and actual check",
        "reference": "reports/2026-11-05-dreaming-sota-mcp-design-review.md"
      }
    ]
  }
}
```
