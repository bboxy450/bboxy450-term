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
created: 2026-11-04T17:39:39.717Z
number: 93377961361169
intent: Hashbrown replaces lookup maps only where ordering does not matter, and KB search uses one owned graph-score and normalized-text projection per Index whose results and score bits equal the uncached reference.
updated: 2026-11-16T19:44:46.302Z
created_by: seat:01M79HGAW321GYNK8468FYN4K9
updated_by: seat:01M79HGAW321GYNK8468FYN4K9
authorship:
  signer: seat:01M79HGAW321GYNK8468FYN4K9
  kind: seat
  public_key: O/GLv3wEDMQc2X2CJwZN8QH5avRWsmx3x4SdZZC6lKU=
  process_generation: '1'
provenance:
  digest: sha256:0046b80b9db33acec233dc09306b5fb8632b8b56d6674a488df9dc396691acbb
  frontmatter_digest: sha256:0aec833a749363dfea3a23caa26b658a1383e1c0ffb2447ecf17a4d84beb3e83
  signer: seat:01M79HGAW321GYNK8468FYN4K9
  sig: xCKNNgne5jLG2YrZfezxk4/MY1c6ETa+82vvqMRBpjghSUu2bpF686a+c1++XjtLOhEp7/cOkaTcQurD70g1CQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151728
  event:
    id: 01M7JY7ZBAQK38Z37CDEYJWAE6
    sequence: 151728
    hash: b049ff6939b5dd010de9e4a0a0acc02820303b5bcf087e379c47f29fb7ffe464
---
The operator authorized adopting hashbrown wherever it makes sense after the 2026-11-04 source review and CPU probes. Implement in the solo hashbrown-search task.

## Lookup selection

Keep Index.documents and public graph-kind maps ordered: prefix/range reconciliation, stable snapshot hashes, graph insertion order and page cursors depend on them. Keep number allocation/current-file validation and ambiguity rules unchanged. Do not mechanically replace maps. Use hashbrown with std::hash::RandomState for KB/content keys and borrowed renderer/metrics text lookups; faster default hashing is confined to bounded, internally generated numeric/font keys where measured. Preserve exact key hashing/equality for borrowed composite keys, Unicode clusters and style fields. Existing crypto hashes never change.

## KB projection

One Index owns its graph-score projection and an immutable Arc of normalized search text; clones retain their own document/graph snapshot and may share immutable normalized entries. The Index root cannot be retargeted. Rebuild installs projections only in its exclusively owned Index. Normalized text entries identify the complete source digest and path, and every cache hit checks that digest; a miss uses the existing JSON-serialization/Unicode-lowercase algorithm. Unchanged entries may be reused across rebuilds. New text admission is bounded to 16 MiB retained cache allocation (hash table, path/digest/normalized string capacities and value allocation) and 4096 entries per snapshot; oversized/missing entries always fall back. Admission prioritizes non-artifact knowledge records before artifact projections, without changing result iteration. No global cache, lock or IO is introduced in search.

Graph degree totals are accumulated once per rebuild in original edge order with saturating f64::MAX addition. Duplicate node IDs share the same ID-level degree, self-links count once, and incoming/outgoing links preserve existing weights. Search filtering, score constants, tie ordering, snippets, paging, freshness and scope stay identical. The native model explicitly bounds cache identity, clone/rebuild/reset and admission; broken digest/admission controls must fail. It abstracts allocation, Unicode, hashing, float arithmetic and OS scheduling; API tests cover those implementation details, including a retained stale cache and A-B-A contents.

## Evidence

Compare actual provider API results and score bits against an uncached/edge-scan reference on private fixtures, including edits/deletes/rename, clone/reopen, Unicode, self-links and duplicate IDs, filters, pagination and budget fallback. Profile cold/repeated queries and report cache bytes. Renderer borrowed lookup tests prove hash/equality and style distinctions and count hit-path ownership/allocation where possible; existing render unit/profile and unattended smoke establish behavior. No UI geometry/design changes are intended. Run make test-hashbrown, mandatory current-source formal parity, licenses, the canonical full stack for Cargo.lock changes and make land. Research timings are not product latency claims.
