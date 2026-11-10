---
type: research
title: Actual Dream and nested MCP source boundaries
parent: '[[research/dreaming-services-mcp]]'
id: 01M6P166946GZQV13BYBW9K4WD
updated: 2026-11-05T14:50:53.592Z
created: 2026-11-05T14:18:15.526Z
number: 93377961364652
created_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
updated_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
authorship:
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  kind: seat
  public_key: L62ggzXO9nS9icBZrd6nPN71BnmasbyzYyfTJE3eu0k=
  process_generation: '1'
provenance:
  digest: sha256:fa192fb265700af571d3edb19fea69a3c83bfaa8799a91f9fcca61e1b5ac3399
  frontmatter_digest: sha256:6afa06d9132b067cd496cb26775b879d975f50dcbb8f020b49c189499bc44c33
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  sig: n19hw6iHTuWpxl8DOR2wr86nYvvxjjopBbKibOBWccXqsP4T9nmE/uIgHK3BjNJ5PBgVR5tw+hpA5k5bH4XqDQ==
  log: .bboxy/log/2026-11-05.jsonl#136917
---
# Prepared tools fit the actual harness boundaries

## What is reachable today?

### Takeaway
The shipped outer bboxy server has no Dream tool. The embedded harness has real Dream CLI/library primitives, but prepared verified retrieval and production orchestration are missing. Dirty C1 code adds a useful own-seat closed-IR control surface; it is a dependency, not shipped main behavior.

### Cited findings
Read 2026-11-05 UTC. Root main **342faefd57e697d32713bd8ed6cd0a99def539ad**, embedded harness **a2ca8f393630081b6be583151293ae286dc2419e** (clean at initial check). C1 producer author tree **f5838ec407175d465cd436078580490130f4c99b**, dirty; consumer author tree **ad243195daf74153756308346d01a56390c1261c**, dirty. No build or product was launched.

| Boundary | Actual source:line — mechanism | Implication |
|---|---|---|
| Outer MCP | term crates/bbt-mux/src/mcp/bridge.rs:21-23 supports through 2025-11-25; :280-312 advertises tools and resources, resources map only to skill_list/skill_read; tools.rs:77 adds knowledge catalogue | No Dream resources/prompts/tasks are exposed by main. Do not add a resource URI and assume it works. |
| Native launches | wake-up src/launch.rs:423-429 writes bboxy MCP config; :628-667 injects per-seat Codex command/config and admitted environment; Claude wiring uses the same overlay | Reuse the existing admitted bboxy server; no extra install, endpoint or home-config rewrite. No capability bytes were read. |
| Engine MCP client | embedded src/mcp.rs:86-111 chooses profile→ACP→launcher sources; :181-221 caps 16 servers; :228-295 snapshots ≤128 tool bindings per turn; :320-349 calls tools and accepts CallToolResult only | Engine models discover tools. MCP resource/prompt/task support in an SDK does not mean this adapter imports it. Non-tool alternate results need explicit implementation. |
| ACP nested client | Embedded harness crates/bboxy450-harness/src/backend/acp_agent.rs:332-346,1709 passes relayed mcpServers; crates/bboxy450-harness/src/mcp/relay.rs:63-82 creates uid-private Unix relays, :125-152 bounds outstanding/deadlined requests (paths relative to term root) | Managed ACP provider is a client of the relayed outer bboxy server. It is not a second Dream truth store. |
| CLI/read primitives | src/dream/mod.rs:32-39 dispatches replay/synthesize/audit/graph/why; audit.rs:125-130 works over explicit resolver roots; graph.rs:421-444 computes provenance closure | Real library/CLI code exists. Calling full audit/why synchronously for every MCP read would be a new CPU/IO problem. |
| Unsafe shortcut to avoid | resolve.rs:323-369 enumerates all JSONL in supplied sessions and reads them whole; trust.rs:660-692 resolves pinned records through Git | Add explicit selected bounded inputs/off-owner preparation. Do not put Resolver::open, Git, KB or provider scans on live tool/logic owners. |
| Verified state | projection.rs:450-500 distinguishes active/adopted from terminal/retired/superseded; receipt.rs:840,1131,1542,1658 checks acceptance/outer/publication/adoption; trust.rs:713-758 checks producer approval | A valid IR hash or Markdown status is insufficient for a verified lesson. |
| Recall | recall.rs:28-97 requires successful audit, approved trust, published/idempotent EngineContext/AcpContext receipt, active harness pair and Current record; :163-195 syncs Reported Recall before yielding context. Engine mod.rs:1062-1080 inserts it as user/reference text | Library seams are real; ordinary engine/gateway opens do not construct this prepared Context. Tool fetch is not automatically Reported Recall. |
| Native exposure | recall.rs:202-243 has explicit validated ClaudeMemory/CodexExtension exposure preparation | Native memory-file publication is a different adapter, not equivalent to exposing MCP data. |
| Production gaps | synth.rs:877 requires scripted input; trial.rs:1012 names initial scripted-engine support; join.rs:32-83 requires committed task bytes; protocol/mod.rs:82-88, scope enum has repo/system, no swarm | No production swarm Dream scheduler or provider-independent accepted-task bridge is established. Current native KB drafts cannot be reclassified as trusted committed verdicts. |
| In-flight C1 | C tree mcp/dream_c1.rs:12-20 names dream_c1; registry/dream_c1.rs:565-568 exposes captured Context/PublicFence; explicit inspect/status/cancel return bounded runs/pages | First new read slice can index validated completed C1 episode/cluster rows, labelled observation/not_evaluated. Depend on accepted C1 landing, not its moving worktree. |

Correction snapshot2026-11-05 14:45:54 UTC: producer src/dream/c1_wire.rs:319-348,359-375,413-416 binds descriptor→page→row, gives Origin only to Episode, and leaves Cluster as repo/fingerprint/count/times. src/dream/d0.rs:940-968,986-999 can aggregate episodes across selected sessions. Row provenance must therefore retain exact manifest/descriptor/page hashes plus row index, with no synthetic cluster event. Producer c1_wire.rs SHA256 b35b24968296f2f581b9bbc20d229ca3014bf32996a1b5b69dda09e8afc3b5a6; d0.rs b92bf40bfeac94944376178db48d7cb1ef2b6fd52adbbe6bcd05eb846518ba25. Embedded ACP backend SHA256 c7e376afe1606fe02accb3eec52aa01c3c3626e0ec12bef1f61c821219d659cd; relay is embedded harness code, not term mux code.

Fresh source SHA256 anchors: outer bridge b1ca0b657408af7ba803a9f3fb0f268674f3481015c04cdd2e7ee640546aae1e; launch 758111bb2b9fb58ec30ab7a4722e457755671318fc63397c5ff934e07f94e0cc; embedded mcp 92fbfc8c4733d019b537d0dcf51c0cbec0bbb88221458f5991345e72589680a3; relay 7ac5159066045c0ade1ec1235e7fea9822c5fb784413ae66ecc698b83f38ab8f; recall b91910994fd748b8131ca0c2ab79789b40ca8751320f073c20c9111d5e6eb4c5; resolve c02970d89b68ccc9ec218f722bdd81478fb67181de96615582f5e73487babeab; audit ecb094a64cecc08a177b43b695c4111c2696a86ec1d0364a70ba0ea8b42ed544; graph 2fa2b4a5a06a096b8a09744ce38ee0c004bc5faf60b59196b64bd29d067aa9d6. Moving C1 producer c1.rs 52e63dacce3fa9d53d041d33704e1e31dad37a67ca48a34048ca020a93c29fcb; consumer registry b8686cfee6ecdd2750974d0fa850a9684d8dd3e9648fc8f36e4d2e9f7b680106. These are static evidence, not final-source acceptance.

## Which MCP pattern survives the compatibility matrix?

### Takeaway
Use small model-callable tools with ordinary results and explicit job handles. Resource aliases and protocol-level async tasks can be later negotiated enhancements.

### Cited findings
Official docs, retrieved 2026-11-05 (publication date not stated): [2025 tools](https://modelcontextprotocol.io/specification/2025-11-25/server/tools) defines structuredContent/output schemas and text fallback; [resources](https://modelcontextprotocol.io/specification/2025-11-25/server/resources) are application-driven; [prompts](https://modelcontextprotocol.io/specification/2025-11-25/server/prompts) are selected by users. None mandates model visibility of every feature.

The [2026-07-28 Tasks extension](https://tasks.extensions.modelcontextprotocol.io/specification/2026-07-28/tasks) uses server/discover advertisement plus request-local client capability, not a client request for task mode; the server decides the response kind. Exact keys and fallback are in [[interfaces/dreaming-mcp]]. This differs from [experimental 2025 tasks](https://modelcontextprotocol.io/specification/2025-11-25/basic/utilities/tasks). We fetched both versions; do not mix their lifecycle APIs. [Python SDK migration](https://py.sdk.modelcontextprotocol.io/migration/) documents removal of its old experimental runtime and no implementation of the redesigned extension yet.

[Claude Code MCP documentation](https://code.claude.com/docs/en/mcp), current fetched page, documents tools/resources/prompts and conditional background tasks on 2026-07-28 connections with feature-flag/provider restrictions; noninteractive elicitation can cancel a task. [Official OpenAI MCP documentation](https://learn.chatgpt.com/docs/extend/mcp) documents local/remote configuration, and [App Server](https://learn.chatgpt.com/docs/app-server) documents resource-read/tool-call and advertised server capabilities. Codex task-extension behavior was not established here. These are documented client capabilities, not tests of installed versions.

### Inferences
Nested flow: model → harness MCP tool binding or ACP/native MCP client → bboxy stdio bridge → admitted mux control handler → own prepared read service / existing C1 owner. Never route from mux back through its own MCP client. Data and evidence originate in retained IR/KB receipts; the prepared index is a bounded disposable projection, not independent authority.

### Gaps
Installed client binary versions/feature flags, remote deployments and negotiated task-extension behavior were not probed. Standalone harnesses have no term seat authority: bounded local published-context reading requires explicit local configuration; otherwise report unavailable and use existing CLI inspection manually. No provider service or private account was accessed.
