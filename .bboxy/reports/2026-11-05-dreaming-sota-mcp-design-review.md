---
type: report
title: Dream MCP aggregate provenance and API research correction
task: '[[tasks/2026-11-05-dreaming-sota-mcp-design]]'
supersedes: '[[reports/2026-11-05-dreaming-sota-mcp-design]]'
author: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
created: 2026-11-05T14:52:22.707Z
commits: []
verify:
  command: git diff --check
  exit_code: 0
id: 01M6P34KF3Y9GVSX3WWC7T9XQB
updated: 2026-11-05T14:52:22.707Z
number: 93377961364673
created_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
updated_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
authorship:
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  kind: seat
  public_key: L62ggzXO9nS9icBZrd6nPN71BnmasbyzYyfTJE3eu0k=
  process_generation: '1'
provenance:
  digest: sha256:3ab15fe743fb05c08afdced10e16ad20b3d922dad27f8755b5cf2e0a7e687b91
  frontmatter_digest: sha256:d5870e3741d7d21a420fa92af378e9fe9917283257912648428b903cda1ca1b4
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  sig: aUcIXsKwqrgqduw3FjE8xYoWMZsaqdPkv6GvODo/sIOR909ygA/zBbQiFgQ40myDFx12uxKcEHpad6BrBr7ZAQ==
  log: .bboxy/log/2026-11-05.jsonl#136968
---
# Dream MCP proof and research correction

Task: [[tasks/2026-11-05-dreaming-sota-mcp-design]] (01M6P0K2WNQAT187215T2P8V3J). Supersedes [[reports/2026-11-05-dreaming-sota-mcp-design]] for the rejected observation proof, relay attribution, Tasks negotiation wording and omitted Mem0 Dream coverage. The original report remains unchanged.

## Corrections and acceptance evidence

| Review item | Disposition / evidence |
|---|---|
| Cluster cannot have an EventPin origin | Corrected [[interfaces/dreaming-mcp]]: every observation has ResultRowPin binding run, exact manifest/descriptor/page digests, page index and zero-based row index. Episode alone copies its actual Origin/EventPin; Cluster has no origin. The binding validates descriptor PageRef length/hash and page/row kind/fields under retained completed/settled custody. No C1 wire changed. |
| Aggregate membership/coverage | Cluster links are optional, ≤8 distinct same-result episode rows. Each offered member must match fingerprint and manifest session→repo, with explicit total/omitted/complete. Q1's sufficient minimum is zero links and incomplete coverage. Complete membership additionally requires all descriptor pages and exact count/time consistency; a representative never certifies all members. No source rereplay/raw transcript export. |
| Concrete negative controls | ICD “Two-session aggregate conformance case” specifies two distinct closed sessions sharing repo/fingerprint, two episode rows and one count2 cluster. Page/descriptor/index/kind tamper, foreign/nonmatching/duplicate member and falsely complete omitted-member controls must fail; valid zero/one-member incomplete cases must pass. [[specs/dreaming-mcp]] matrix and native open/unassigned [[tasks/2026-11-05-dream-mcp-observation-reads]] now require these causal/formal/actual-process witnesses. They are proposed checks, not executed tests. |
| Honest Q1 benefit | Spec/ICD/task/research now explicitly restrict Q1 to literal identifiers, exact fingerprints and recency metadata investigation. No symptom-language matching or guidance can be inferred from hashes. Q2/Q3 supply reusable published lesson-text guidance. |
| Malformed ACP source attribution | Correct term-root paths are crates/bboxy450-harness/src/backend/acp_agent.rs:332-346,1709 (relay setup/mcpServers) and crates/bboxy450-harness/src/mcp/relay.rs:63-82,125-152 (uid-private socket, request/deadline bounds). These are embedded harness files, not term mux relay or frontends/gui/backend paths. Source-map child corrected. |
| Exact current Tasks negotiation | Refetched the [official2026-07-28 specification](https://tasks.extensions.modelcontextprotocol.io/specification/2026-07-28/tasks). ICD now names server/discover advertisement, request-local capability keys, server-selected response kind and missing-capability error; support is not client task preference. Ordinary tool results/C1 handles remain the implementation baseline. No adapter/task support was tested or added. |
| Mem0 dedicated Dream APIs omitted | [[research/dreaming-services-mcp]] comparison and new endpoint table now cover project config/entitlements, effectful config update, no-write scope preview, keyset run/memory/activity pages and synthesized-source lineage, using actual fetched official pages. Entitlement values/prices/cadence and live account availability remain unverified; example123 is not a quota. No new bboxy endpoint or vendor integration was added. |

Mem0 primary pages fetched2026-11-05, publication dates not stated: [configuration](https://docs.mem0.ai/api-reference/dream/get-dream-config), [update](https://docs.mem0.ai/api-reference/dream/update-dream-config), [preview](https://docs.mem0.ai/api-reference/dream/dream-preview), [runs](https://docs.mem0.ai/api-reference/dream/get-dream-runs), [run memories](https://docs.mem0.ai/api-reference/dream/get-dream-run-memories), [activity](https://docs.mem0.ai/api-reference/dream/get-dream-activity), [source lineage](https://docs.mem0.ai/api-reference/dream/get-dream-memory-sources). Initial direct fetches of config/preview failed; official navigation links subsequently fetched both successfully. Documentation was read; api.mem0.ai was never called. No Dream-specific CLI/job-create/cancel/idempotence guarantee or hard spend cap was inferred.

## Exact source and verification snapshot

Read-only source inspection at2026-11-05 14:45:54 UTC:
producer HEAD f5838ec407175d465cd436078580490130f4c99b, with pre-existing dirty implementation. src/dream/c1_wire.rs:319-348,359-375,413-416 defines descriptor/page/typed rows and episode-only Origin; SHA256 b35b24968296f2f581b9bbc20d229ca3014bf32996a1b5b69dda09e8afc3b5a6. src/dream/d0.rs:940-968,986-999 accumulates cross-session cluster counts/min/max; SHA256 b92bf40bfeac94944376178db48d7cb1ef2b6fd52adbbe6bcd05eb846518ba25. These observations are static, not final-source acceptance.

Embedded relay SHA256 7ac5159066045c0ade1ec1235e7fea9822c5fb784413ae66ecc698b83f38ab8f; ACP backend c7e376afe1606fe02accb3eec52aa01c3c3626e0ec12bef1f61c821219d659cd. Actual read-only Git/nl/rg/shasum command batches exited0.

Final verification snapshot2026-11-05 14:51:28 UTC:
- git diff --check in /Users/feral/mydev/bboxy450-term: **exit0**, empty output. It checks tracked diff only; full KB readbacks cover these untracked Markdown records.
- git rev-parse HEAD and git status --short: exit0, main342faefd57e697d32713bd8ed6cd0a99def539ad; only untracked .bboxy records, including peers' preserved records.
- Embedded harness rev-parse/status: exit0, a2ca8f393630081b6be583151293ae286dc2419e, clean status.
- task_status for Q1: native status open, no assignee. No delegation or task-scope/verify change.

## Current artifact digests and readback

All five updated records were fully fetched, using continuation pages for the ICD; full bodies matched intended writes and returned digests. Unchanged papers/Q2/Q3 remain as recorded previously.

| Locator | Digest |
|---|---|
| [[interfaces/dreaming-mcp]] | c9538af11c7f8802ad3b5925f63538a58accb34c5520ae8c3e2d2b017e7f4e93 |
| [[specs/dreaming-mcp]] | 3bb9dc553858510973ce42ba1e41b8867403c054ff56a1b8dad86d446df939d6 |
| [[tasks/2026-11-05-dream-mcp-observation-reads]] | a5d81813030e41553ad3982a6aa4729666c9f82c9f7f8d6432ccbf34adca6050 |
| [[research/dreaming-services-mcp/harness-capabilities]] | f53f3d4cb579c05154e9c3f00e79a49e4d22f01e3c84921d0ce4072956cc04ca |
| [[research/dreaming-services-mcp]] | e52b31e4bda300a8ae94dca75ed1eb56766fb318372026cdc8ba2d161eda2f65 |

No source edits, C1 schema/scope changes, builds/tests/formal/performance runs, live provider/API/account calls, credentials, installation, commit, merge or publication. C1 acceptance/integration and actual qualifying publication/provider proof remain dependencies. This correction makes no green runtime or measured-benefit claim.
