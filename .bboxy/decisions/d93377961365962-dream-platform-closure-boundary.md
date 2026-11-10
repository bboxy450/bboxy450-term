---
type: decision
title: Dream proof-tool closure ends at a bound platform boundary
status: decided
date: 2026-11-06
task: '[[tasks/2026-11-05-dream-launch-entry-and-signature-boundaries]]'
derived_from:
- '[[reports/2026-11-05-dream-platform-closure-packing-evidence]]'
- '[[reports/2026-11-05-dream-proof-tools-provisioner-source7-blocked]]'
- '[[interfaces/dream-proof-tool-qualification]]'
id: 01M6QCMA6JC6DEGEN4NYCYFZR3
updated: 2026-11-06T03:14:24.170Z
created: 2026-11-06T02:57:27.217Z
number: 93377961365962
created_by: seat:01M6QBVWBSF0CCV4HGVC3V9JQY
updated_by: seat:01M6QBVWBSF0CCV4HGVC3V9JQY
authorship:
  signer: seat:01M6QBVWBSF0CCV4HGVC3V9JQY
  kind: seat
  public_key: X/rzXNejssjUR5rEaGHy1fr/qmfAtbpX+NAM+IbU7Hk=
  process_generation: '1'
provenance:
  digest: sha256:853753696ceb6a2bcbb7afcb9c62b2e7e6fafbbf43e78ad95742b1aea8a92812
  frontmatter_digest: sha256:0226a7f5f89f8d1febb318fbbf52c13d75cbad4b55abc4f15dcffa1211baf8ec
  signer: seat:01M6QBVWBSF0CCV4HGVC3V9JQY
  sig: 9a7C0WfTBrhnzwipDPYKte1vXllTvSuy/s6hQOFBK10QZ1AXSQ1Xn0KO3/fVf/jLcV9ABVj65vjy9pux0oOABA==
  log: .bboxy/log/2026-11-06.jsonl#149235
---
# Dream proof-tool closure ends at a bound platform boundary

Date 2026-11-06. Architect seat 01M6QBVWBSF0CCV4HGVC3V9JQY. The canonical normative text is in [[interfaces/dream-proof-tool-qualification]] under "Closure boundary". This record keeps the evidence, the alternatives and the reasons.

## Evidence consumed (not re-run)
- Packing report [[reports/2026-11-05-dream-platform-closure-packing-evidence]], digest 4285c552…0753, inventory bcc52145…b2ad. Root rehashed it and decoded its 6 transforms.
- The input is an INCOMPLETE lower-bound graph from the earlier pair (9e48c55d), not the final pair at 78391c4c. Its retained platform context: Darwin 27.0.0, arm64, OS 26A434, cache UUID c284a291b68b3d0da1a03f866f87efdc. These are sibling facts, not a current measurement.
- The current typed Closure lower bound is 54,813 B, of which nodes are 37,169 and edges 16,863. That already exceeds Basis 49,152.
- The fact-only interned form is 58,784 B, at depth 12, which also exceeds the limit.
- The graph has 467 edges and 61 identities: 2 roots, 9 static, 49 platform, plus the unknown `/usr/lib/libobjc-env.dylib`.
- The source7 refusal is at edge 129, `libdyld→libunwind`. That is a platform-internal edge.

## Premise correction
The ICD row `platform | review_only` and the "conditional trusted premises" paragraph already put OS-library behaviour outside machine proof. Yet the Closure law required every platform image's UUID, load paths, rpaths, install names and platform→platform edges to be enumerated inside the Basis. That double standard is the defect. Most of the bytes, and the failing edge, are **OS-internal facts that the qualification never claimed to prove**. A fourth narrowing of the enumeration schema would only measure our tolerance for redundant OS metadata. It is not a currentness check.

## Alternatives
| Option | Verdict |
|---|---|
| Raise the edge/array/byte caps | Rejected: budgets are allocated, not discovered, and raising them spends other shares. |
| Intern or compress the full platform graph | Rejected: the measured forms fail at 58,784 B / depth 12, and still no complete graph exists. |
| An external platform catalogue blob | Rejected: it would be an uncharged proof store, and it is not needed for any runtime check. |
| Declare Unsupported now | Rejected as premature: the failure is in a representation the law never needed. |
| **Platform boundary binding** | **Chosen**, as below. |

The chosen binding enumerates the candidate side completely. Everything the candidate loads from the OS is bound by one immutable platform identity tuple. Platform-internal structure is an explicit trusted premise.

## Choice (summary; the ICD "Closure boundary" literal schema is canonical; revised 2026-11-06 after Root review)
Root accepted the trust-boundary choice but found the wire shape inexact: prose-only fields, a new Platform naming next to the old one, and an obsolete Node/edge shape. The ICD now defines one literal schema. In summary:
1. **No new Platform.** The existing `Platform` shape is reused unchanged (`arch`, `darwin_machine`, `release`, `os_build`, `shared_cache_uuid`), and `Closure.platform` must equal `Build.platform`. The trust premise is fixed law, not a data field.
2. **Root shape.** `Closure={roots,statics,link_maps,platform}` replaces Node/edges:
   - roots are exactly `[git,gpg]`;
   - each root has a Macho with rpaths, and loads carrying `{mode,name}` in load-command order with multiplicity;
   - modes are load, weak, reexport, upward, lazy;
   - loads are ≤64 per root and ≤128 summed, which keeps the edge cap.
3. **Cache-only targets.** The only load target is the bound dyld shared cache. There is deliberately **no private-dylib target**. A prefix name, an `@` path, a non-cache or unresolved name, or a root load of libobjc-env makes the host Unsupported.
4. **Static links.** Static links live on the root: `{id, member_count≥1, members_sha256}`.
   - The member hash is SHA256 over `u32be(len)‖raw name bytes` in link-map order, so there is no delimiter ambiguity.
   - Static archives are `{id,source,file,member_index}`, sorted, unique and reachable.
   - link_maps are exactly `[git,gpg]`, with output_sha256 equal to the root's file sha256.
5. **Counted vs offline.** Basis bytes are counted prepared metadata. Link maps, archives, member indexes and platform evidence are offline bounded-hash provenance: Q2 never reads them. The builder and Root recompute the hashes and the load resolution offline.
6. **Currentness.** Q2 observes the platform at two checkpoints, before birth and after settlement. This does not prove the absence of a transient change between them, or what the child mapped. Kernel/dyld behaviour remains conditional trust.
7. **Example.** A typed fictional example is included (2,603 canonical bytes) only to disambiguate the codec.
8. **Fit and format.** Fit for the final 78391 pair stays UNKNOWN, and the original verify still fails with exit 2. Caps, wire and version are unchanged. The new canonical hash invalidates old tuples.

## What this does not prove
It does not prove OS library behaviour, dyld resolution semantics, cache immutability per UUID, or that the platform-internal closure is free of threads/helpers. These are explicit Root-reviewed premises bound to one cache UUID, exactly as `platform | review_only` already stated. Runtime hidden-thread/process mutants still invalidate the capability.

## Required controls (both canonical owners)
- **Codec (Q2 decoder, builder emitter):**
  - round-trip of the exact canonical bytes;
  - order and multiplicity preserved;
  - the decoder refuses each of: a dropped root load; a changed mode; a reordered load; an unknown or duplicate node; an unreachable static node; a forged root UUID; a non-cache or `@rpath` target; an unexpanded `libobjc-env` referenced from a root; a wrong member hash or count; a changed link-map Ref; a reserved/unknown field; oversize or excess depth; a truncated or partial publication.
- **Currentness:** cache UUID, OS build or arch changed (A→B→A included) → refused before birth, or the result invalidated after.
- **Model:** vars `platform ∈ {P1,P2}`, `basisPlatform`, `rootLoads ⊆ {cache, candidate, rpath, unknown}`, `published ∈ {none, partial, full}`, `approved`. Invariant: approved ⇒ full ∧ every load ∈ {cache, candidate} ∧ basisPlatform = platform at birth and at settle. Broken controls that must fail: accept rpath/unknown; skip the post-settle platform check; accept partial publication.
- **Unapproved runtime:** the candidate stays unapproved under component mode. No green from codec success alone.

## Handoffs
- **Builder (Oracle amends):** emitter for the boundary form at source 78391c4c successor; real-pair emit once, under the existing build brief. Report the actual bytes or the exact Unsupported cause.
- **Q2:** strict decoder plus the platform currentness checks.
- **Root:** freezes the changed Q2 source only after both are coherent.
