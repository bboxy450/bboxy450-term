---
type: spec
title: Hashbrown lookup selection and bounded KB search projections
status: decided
derived_from:
- '[[tasks/2026-11-04-hashbrown-search]]'
links:
- to: '[[knowledge-base]]'
  rel: constrains
- to: '[[storage]]'
  rel: constrains
id: 01M6KTA7H9DFX1H51W6X38C00S
updated: 2026-11-04T17:39:39.717Z
created: 2026-11-04T17:39:39.717Z
number: 93377961361169
created_by: seat:01M6KN12436S0N6CJDQ0RWBQNG
updated_by: seat:01M6KN12436S0N6CJDQ0RWBQNG
authorship:
  signer: seat:01M6KN12436S0N6CJDQ0RWBQNG
  kind: seat
  public_key: 4FB10dq226S4+Z6CUKQrEvBUsQCbt4dAXA3Nd2bPO6o=
  process_generation: '1'
provenance:
  digest: sha256:0046b80b9db33acec233dc09306b5fb8632b8b56d6674a488df9dc396691acbb
  frontmatter_digest: sha256:93fecb950a36e195052b7824ebe5149b57a06b6817364fba8d6e581e37462ce0
  signer: seat:01M6KN12436S0N6CJDQ0RWBQNG
  sig: Wy3A8P+cAK/uk+Sq66cT5+y98npj5niXio1uT75MiydIb/8UD2aSSBxq4zV8Uiz1Uojue61A7tOU+L9P7Bw7Cg==
  log: .bboxy/log/2026-11-04.jsonl#122318
---
The operator authorized adopting hashbrown wherever it makes sense after the 2026-11-04 source review and CPU probes. Implement in the solo hashbrown-search task.

## Lookup selection

Keep Index.documents and public graph-kind maps ordered: prefix/range reconciliation, stable snapshot hashes, graph insertion order and page cursors depend on them. Keep number allocation/current-file validation and ambiguity rules unchanged. Do not mechanically replace maps. Use hashbrown with std::hash::RandomState for KB/content keys and borrowed renderer/metrics text lookups; faster default hashing is confined to bounded, internally generated numeric/font keys where measured. Preserve exact key hashing/equality for borrowed composite keys, Unicode clusters and style fields. Existing crypto hashes never change.

## KB projection

One Index owns its graph-score projection and an immutable Arc of normalized search text; clones retain their own document/graph snapshot and may share immutable normalized entries. The Index root cannot be retargeted. Rebuild installs projections only in its exclusively owned Index. Normalized text entries identify the complete source digest and path, and every cache hit checks that digest; a miss uses the existing JSON-serialization/Unicode-lowercase algorithm. Unchanged entries may be reused across rebuilds. New text admission is bounded to 16 MiB retained cache allocation (hash table, path/digest/normalized string capacities and value allocation) and 4096 entries per snapshot; oversized/missing entries always fall back. Admission prioritizes non-artifact knowledge records before artifact projections, without changing result iteration. No global cache, lock or IO is introduced in search.

Graph degree totals are accumulated once per rebuild in original edge order with saturating f64::MAX addition. Duplicate node IDs share the same ID-level degree, self-links count once, and incoming/outgoing links preserve existing weights. Search filtering, score constants, tie ordering, snippets, paging, freshness and scope stay identical. The native model explicitly bounds cache identity, clone/rebuild/reset and admission; broken digest/admission controls must fail. It abstracts allocation, Unicode, hashing, float arithmetic and OS scheduling; API tests cover those implementation details, including a retained stale cache and A-B-A contents.

## Evidence

Compare actual provider API results and score bits against an uncached/edge-scan reference on private fixtures, including edits/deletes/rename, clone/reopen, Unicode, self-links and duplicate IDs, filters, pagination and budget fallback. Profile cold/repeated queries and report cache bytes. Renderer borrowed lookup tests prove hash/equality and style distinctions and count hit-path ownership/allocation where possible; existing render unit/profile and unattended smoke establish behavior. No UI geometry/design changes are intended. Run make test-hashbrown, mandatory current-source formal parity, licenses, the canonical full stack for Cargo.lock changes and make land. Research timings are not product latency claims.
