---
type: report
title: Dreaming research and bounded MCP design handoff
task: '[[tasks/2026-11-05-dreaming-sota-mcp-design]]'
author: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
created: 2026-11-05T14:35:16.808Z
commits: []
verify:
  command: git diff --check
  exit_code: 0
id: 01M6P25BBB4A6CK2556R1YHNK0
updated: 2026-11-05T14:35:16.808Z
number: 93377961364666
created_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
updated_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
authorship:
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  kind: seat
  public_key: L62ggzXO9nS9icBZrd6nPN71BnmasbyzYyfTJE3eu0k=
  process_generation: '1'
provenance:
  digest: sha256:948bbea488f27917d13d0fe03a48fc3926934d926944acd8505e2a24c72627e1
  frontmatter_digest: sha256:d3eddbf18d7bb4717f5886fed006ac200b8a29dfdb3934658b311a395e9d23be
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  sig: VDwmvlsmKVjw5ro1gc3zQ3cvuUubM2iY/HfXCpWnPOATH1YJIyrjGV/TUxnHXrrN0fmU8MhTgcDIpqbbkdPjAg==
  log: .bboxy/log/2026-11-05.jsonl#136782
---
# Dreaming research and MCP design handoff

Task: [[tasks/2026-11-05-dreaming-sota-mcp-design]] (01M6P0K2WNQAT187215T2P8V3J). Research/design only; ready for Oracle selection, not implementation acceptance.

The useful first cut is **dream_search + dream_read over completed, settled own-seat C1 observations**. It exposes exact origins without inventing a fix, verified lesson, adoption or Recall. Two later drafts prepare already-published verified lesson snapshots and integrate them with reported-only feedback. Heavy preparation remains after authoritative SessionClosed; a live read never starts consolidation. Current C1 authors and their scopes were left unchanged.

## Evidence against acceptance

| Criterion | Delivered evidence and limits |
|---|---|
| Current primary-source research | [[research/dreaming-services-mcp]] compares Anthropic managed-agent Dreams, Codex memories, Letta, Mem0, Zep/Graphiti, Cognee and LangMem using fetched official documentation/repositories. [[research/dreaming-services-mcp/papers]] covers seven original papers on sleep-time compute, consolidation, contrastive experience, memory evaluation and poisoning. Publication/revision dates, retrieval date 2026-11-05, supported mechanisms and availability limits are beside claims. Documentation without a stated publication date is marked accordingly. No benchmark reproduced or vendor service exercised. |
| Grounded minimal contract/workflows | [[specs/dreaming-mcp]] and [[interfaces/dreaming-mcp]] define closed v1 DTOs, role/scope/currentness, bounded immutable pages, ordinary MCP tool-result fallback, source/result custody, error/status semantics and causal verification matrix. [[research/dreaming-services-mcp/harness-capabilities]] records actual source lines, hashes and nested engine/native/ACP flow. Observations, qualifying published lessons and reported feedback are distinct. Missing publication/provider proof remains unavailable, not an implicit trust adapter. |
| Actual actionable draft tasks | Q1 [[tasks/2026-11-05-dream-mcp-observation-reads]] (01M6P1PS9F0AGDDW9HNJR1N99C), Q2 [[tasks/2026-11-05-dream-mcp-verified-snapshot]] (01M6P1PTMD3SDRJ4JQFFV2AXV8), Q3 [[tasks/2026-11-05-dream-mcp-verified-feedback]] (01M6P1PWPKEEYWPZGQW21EYCW9). Native task_status returned open with no assignee for all three. Each has ROLE: EXECUTOR, isolated-worktree placeholder, exact allowed paths, dependencies, acceptance witnesses, proposed Make entry point and existing relevant gates. None was delegated. Oracle must pin exact worktree/baseline before assignment. |
| Artifact/readback/verification discipline | All eight artifacts below were fully fetched through continuations as needed; returned digests match current writes and full bodies match intended content. git diff --check exited 0 with no output. Only allowed knowledge records were written. No production/source/submodule edits, builds, tests, accounts, credentials, service calls, installations, commits, merge or publication. |

## Source-grounded dispositions

- Main has no Dream MCP tools: term crates/bbt-mux/src/mcp/tools.rs:77 is the existing knowledge catalogue; bridge.rs:21-23 and :280-312 negotiate through 2025-11-25 and expose tools/skill resources. Embedded harness src/mcp.rs:228-295 snapshots tools and :320-349 handles CallToolResult. Official newer MCP Tasks documentation does not establish installed adapter support. Use ordinary tool results and existing durable C1 handles; a future tasks adapter requires explicit negotiation and a fallback.
- Existing launch/relay paths are reusable: wake-up src/launch.rs:423-429,628-667; harness src/frontends/gui/backend/acp_agent.rs:332-346,1709; term relay.rs:63-82,125-152. The design adds no separate server installation, global config or circular MCP self-call.
- Existing Dream verification is useful but cannot be placed on the hot tool owner: harness src/dream/resolve.rs:323-369 scans directories and reads whole files; the proposed producer uses selected bounded inputs in the dedicated owner. src/dream/recall.rs:28-97 checks audited compatible active publication state; :163-195 writes Reported attribution only after actual injection. src/dream/join.rs:32-83 still requires committed task evidence. A native accepted draft is not this bridge.
- src/dream/synth.rs:877 and trial.rs:1012 expose scripted prerequisites, not complete production synthesis/trials. Protocol scope is repo/system (src/protocol/mod.rs:82-88), with no Swarm scope. Neither feature presence nor seat membership proves public reachability or sharing authority.
- Existing C1 control/closure/public-fence and refined memory law are dependencies, not redesigned here. Search/read preparation retains CPU/duty/latency/32+96 MiB/control/foreground-pause/settlement/ABA gates. New cache storage spends the existing parent share. Formal models prove declared abstractions; opaque native allocation and measured latency remain implementation evidence.

Exact paths/hashes and the moving-source qualification are retained in [[research/dreaming-services-mcp/harness-capabilities]]. No finding required an in-flight author scope amendment; no routine author messages were sent.

## Revisions and actual verification

Initial date command returned 2026-11-05 14:08:12 UTC. Final recorded Git/verification snapshot: 2026-11-05 14:31:12 UTC.

| Read/check | Actual result |
|---|---|
| git diff --check, /Users/feral/mydev/bboxy450-term | Exit 0, empty output. This checks tracked diff only, not the untracked KB Markdown. Full tool readbacks verify the new records. |
| git rev-parse HEAD; git status --short, term | Exit 0; 342faefd57e697d32713bd8ed6cd0a99def539ad. Status listed untracked .bboxy knowledge records only; no tracked source changes. Existing records from peers were preserved. |
| git -C crates/bboxy450-harness rev-parse HEAD; git -C crates/bboxy450-harness status --short | Exit 0; a2ca8f393630081b6be583151293ae286dc2419e, empty status. |
| Read-only author-tree source/Git snapshot | Producer f5838ec407175d465cd436078580490130f4c99b; consumer ad243195daf74153756308346d01a56390c1261c. Both had dirty in-flight source; selected exact file SHA256 values are in the source-map child. They are not shipped main behavior. |

No product/formal/performance run occurred. Proposed make test-dream-mcp is explicitly not installed in inspected main. Drafts require it to be added, plus existing owner-specific formal/process gates. Yarn/Rust and pinned Java TLC parity, causal broken controls and real unattended witnesses are specified as future acceptance, not reported green. Benefit scenarios are an evaluation plan, not a claim of improved task success.

## Readback artifact digests

| Locator | Current digest |
|---|---|
| [[research/dreaming-services-mcp]] | b82ecd57121bebe950a4a52abefd527db708156e4cb63de4f35c669a46a6e4d9 |
| [[research/dreaming-services-mcp/papers]] | dece22fbd9bfb28e86ad7fa3705b72dd92d89c32fd9f86370048550718e12b6e |
| [[research/dreaming-services-mcp/harness-capabilities]] | a6afb9cefcb8da7e82ff039e6a1ae408a67380a706fe5fa6fa30d6d1620cf2ed |
| [[specs/dreaming-mcp]] | a705dc7744cf714a00bcd69b23076602cc604ce033eafd694273c342ce24b3cb |
| [[interfaces/dreaming-mcp]] | b4f570aff37a126cdf90525e161e702ab5b472ee5c4798fd8ecc0c77f90d1aab |
| [[tasks/2026-11-05-dream-mcp-observation-reads]] | 16051374d43cbb8206353a64ab7c61d3d144bb6868c8e01a2849076bb0167b1e |
| [[tasks/2026-11-05-dream-mcp-verified-snapshot]] | e60d926e91051b7b93acc18c8f001e6b9263d5395069d3fb0b9218bdd6b3cbcb |
| [[tasks/2026-11-05-dream-mcp-verified-feedback]] | a7132f7df671308142d3369fb83a7638394772a373298e57bc5af23a91842298 |

## Remaining dependencies and deliberate exclusions

Q1 depends on accepted C1 producer/consumer integration. Real Q2/Q3 lesson availability additionally needs authorized qualifying published input and a provider/public-proof bridge; absent that, fixtures can verify the exporter while real reads remain unavailable. Production inference/trials/publication, account/system/swarm sharing, native-memory modification, MCP Tasks/resources/prompts expansion, automatic policy promotion and an actual new IR-writer bridge are excluded.

Research fetched public primary sources only. Preview access, hosted pricing/limits, installed client behavior and maintenance guarantees were not independently exercised; no private account was inspected. No raw account logs or secrets were read. Web-fetch failures/redirects were replaced with supported official pages or left uncertain. No paper score, vendor marketing or search snippet was treated as local verification. Source moves can stale this snapshot; Oracle owns final-source review and delegation.
