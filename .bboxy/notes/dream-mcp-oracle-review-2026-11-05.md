---
type: note
title: Oracle review of Dream MCP research and ongoing C1 boundary corrections
derived_from:
- '[[tasks/2026-11-05-dreaming-sota-mcp-design]]'
id: 01M6P32PMKK5075GPZ612PCP2N
updated: 2026-11-05T14:51:18.487Z
created: 2026-11-05T14:51:18.487Z
number: 93377961364671
created_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
updated_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
authorship:
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  kind: seat
  public_key: UgGFd6fs0Ln9gJXlN2fw8kUmonpBkr4IBn/U5N0uOrY=
  process_generation: '1'
provenance:
  digest: sha256:a6e9a60f9b6da256c60231af6d52cc81749b8c8c75cfa53da90a5c34dc7f42d9
  frontmatter_digest: sha256:d35d6e9e73f71acb7ce14b023066e61e5325ef2e4e520c87510d505644918c80
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  sig: YUXMv7p6fM4Mu5sELjrBNeQzgxtsSFGIGEyBTbxJZI9WV5JOMDEZIpB/0QEwW52Y3jxbku9DMLVd19/AzVtWBA==
  log: .bboxy/log/2026-11-05.jsonl#136929
---
## Review scope and actual checks
Reviewed [[reports/2026-11-05-dreaming-sota-mcp-design]], its research tree, [[specs/dreaming-mcp]], full [[interfaces/dreaming-mcp]] and Q1/Q2/Q3 native draft briefs. Main remains 342faefd57e697d32713bd8ed6cd0a99def539ad; moving source author trees remain implementation evidence only. Root git diff --check in term exited0. Read-only source inspection; no product test/build run by this review.

Primary-source readbacks independently support the offline-preparation/live-retrieval split: Anthropic Dreams is an asynchronous research-preview job with separate unchanged input and output stores (https://platform.claude.com/docs/en/managed-agents/dreams); Codex memories skip active/short sessions and wait for idle, a different trigger from bboxy's required durable close (https://learn.chatgpt.com/docs/customization/memories); Letta documents step/compaction background dreaming, likewise not our trigger (https://docs.letta.com/configuration/memory). Original Sleep-time Compute, LightMem, MINJA and Memory Poisoning abstracts were fetched and support the bounded mechanisms/limitations described in the research; no results were reproduced. Current MCP Tasks extension documentation explicitly requires extension capability in each request, with server choosing task result versus ordinary result; current bboxy ordinary-result fallback remains appropriate (https://modelcontextprotocol.io/extensions/tasks/overview).

## Required design correction, not accepted yet
Task rejected for a concrete observation-proof mismatch. Actual harness c1_wire.rs:359-375 has Episode.origin but Cluster only repo/fingerprint/count/times; d0.rs:950-1001 aggregates across selected sessions. Requiring one source:EventPin for every observation invents a cluster source event. Architect must bind exact immutable result/page/row provenance, keep episode-only origin separate and state bounded membership coverage without C1 wire expansion/source replay. Q1 is investigation through metadata/fingerprints, not symptom-language semantic search or reusable verified fix. Q2/Q3 supply verified reusable guidance. Signed rejection message 3B9YX7MRFHG6Y83C1FFGW9GZJK records correction, examples/controls and append-only resubmission.

Independent browsing also found dedicated Mem0 Dream APIs beyond generic memory add: https://docs.mem0.ai/api-reference/dream/get-dream-config , https://docs.mem0.ai/api-reference/dream/dream-preview , https://docs.mem0.ai/api-reference/dream/get-dream-memory-sources . Sent exact official leads to Architect for the same correction; no API/account use. Delivery storage is not model receipt. No new source implementation delegated; draft prerequisites remain accepted C1 integration and, for real lessons, qualified published/provider proof.

## Concurrent C1 dispositions
Producer task amended for exact managed-public-environment redaction bug, [[decisions/d93377961364668-dream-c1-managed-public-env-redaction]]. src/mcp.rs:720-735 registers every resolved value; redact.rs scrubs serialized typed IR. Exact same-name Launcher public seat/generation/role references may bypass registration after type validation; no broad secret exemption or capability change. Actual generation7/op ending07 managed UserTurn and existing credential-redaction witnesses required.

Consumer task amended for exact knowledge/roots.rs accessor: source:383-395 uses blocking selected.lock() to return revision/provider separately. One bounded try_lock metadata snapshot must keep held provider transitions off C1 per-unit/status/cancel paths, with routed held-lock/latency and zero-custody proof; no provider behavior refactor. Also notified consumer that scripts/test-dream-c1.py is ignored by /scripts/ and must be deliberately tracked in its exact signed implementation commit. Author work remains in progress; no passing final-source/landing claim.
