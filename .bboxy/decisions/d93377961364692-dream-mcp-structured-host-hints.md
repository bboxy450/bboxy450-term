---
type: decision
title: Keep Dream recovery hints in structured MCP host data with a compatible text mirror
id: 01M6P6D7YA7803CZCNKW9QW46F
updated: 2026-11-05T15:49:33.263Z
created: 2026-11-05T15:49:33.263Z
number: 93377961364692
created_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
updated_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
authorship:
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  kind: seat
  public_key: UgGFd6fs0Ln9gJXlN2fw8kUmonpBkr4IBn/U5N0uOrY=
  process_generation: '1'
provenance:
  digest: sha256:80f86f4c291194baf14eab0a4b60a0514d85f803fb71f9ec774a0d547a47b91e
  frontmatter_digest: sha256:81da4ca8042fc41cab7cf2bc6b8239a4d8dfb4c60c2c259ef159a2c02b35fce8
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  sig: n4IxH0+KUYq8GOLS7foZol1Bvs2hCKNgMwyZD1LKdv98wkKpU2l6D0N2Xo71b2cShmJe9wJUJhxcikOjSZhsAQ==
  log: .bboxy/log/2026-11-05.jsonl#137337
---
## Choice and grounding
Operator reports from a local Codex source checkout that structuredContent becomes the model tool result and ordinary text blocks are omitted on that path. Treat a structured-only consumer as a required compatibility case. Independently fetched official OpenAI guidance says structuredContent is model-usable data and _meta is hidden from the model: https://developers.openai.com/plugins/build/mcp-server#return-useful-results-without-ui (retrieved2026-11-05). This review did not independently inspect that Codex checkout.

Root source inspection: C1 mcp/dream_c1.rs::success delegates to the normal mcp::success wrapper; mcp/bridge.rs:414 applies shared hints::attach. Current local hints.rs:205-226 places hints only in a second text block and _meta, so the operator's shared fix is needed here too. C1 catalogue's current outer object permits host metadata. The supplied parallel branch patch adds structured hints/shared schema and post-journal seat-socket projection plus control-version20; its named worktree is unavailable on this machine, and local main still342faefd. No landed shared-fix or installed-model claim is made.

Use ONE shared host hint generator/projection for Dream and all other tools. Essential next-step/recovery guidance is present in structuredContent.hints, with a matching serialized text copy for clients using content. _meta may duplicate guidance for UI use but cannot be its sole model-facing source. Do not add a Dream-specific second renderer or privately reproduce the parallel shared implementation. Preserve shared projection idempotence if both the seat socket and bridge apply it; project after durable journaling/off logic owner, without rewriting signed receipts, original operation identity, manifest/result/page bytes or their hashes. Advisory hints do not grant authority, run jobs, revive stale context or become lesson knowledge.

## Schema and budgets
Current flat built-in MCP structured shape remains domain fields plus host op_id/hints. For future dream_search/read/feedback, closed domain DTOs remain closed; advertise a distinct closed outer MCP schema extending each domain variant with the known op_id and the shared hints schema. Do not combine a closed unextended domain schema with host fields via an incompatible allOf, loosen unknown domain fields globally, stuff hints inside plugin nested result, or add host hints to inspect-c1/read-prepare worker wire/artifact schemas. Typed domain parsing explicitly validates/separates the known host fields rather than rejecting them or ignoring arbitrary unknowns.

All serialized structured/text/UI copies of host hints count toward existing Dream output/frame/context budgets. Reserve bounded host guidance space before choosing whole entries/pages; keep exact pins, original op_id and compact required recovery guidance. Do not let the shared generic256KiB frame check bypass a future Dream16KiB cap or its token budget. A schema or budget failure is a real failure, never a reason to hide guidance only in _meta.

## Implementation and proof
The parallel harness owns the generic hints.rs/tools.rs/seat_socket.rs/control-version fix. Current C1 consumer adds focused integration/schema assertions within its owned C1 test/component paths and consumes the actual shared fix during final source reconciliation; do not independently overwrite the shared helper or bump that version. Continue independent C1 work while that external patch is not locally available. Final acceptance requires actual joined/rebased evidence, not the pasted patch. Future Q1/Q3 briefs carry the same obligation.

Witness actual admitted Dream MCP responses with a client adapter that discards all ordinary content and _meta whenever structuredContent exists: success/page, empty/unavailable/pending/error, original-op recovery and stale/foreign refusals as applicable to the slice. Validate the FINAL projected document against the advertised outputSchema; compare the text mirror's hints, nested payload bytes/identity and no duplicate projection. Include near-limit results and malicious memory/provider text to show it cannot author host instructions; keep all existing source/authority/closure/resource gates. An ordinary UI screenshot or second text block alone cannot pass. No producer wire/protocol/version change, synthesis work, inference or runtime service use is introduced.