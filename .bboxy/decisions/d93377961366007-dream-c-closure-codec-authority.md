---
type: decision
title: One producer-owned bounded closure codec completes the shared C contract
status: decided
derived_from:
- '[[decisions/d93377961365962-dream-platform-closure-boundary]]'
- '[[interfaces/dream-proof-tool-qualification]]'
id: 01M6RNJ2QHQF2YS4D6BH1FAQA0
updated: 2026-11-06T14:52:45.603Z
created: 2026-11-06T14:52:45.603Z
number: 93377961366007
created_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
updated_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
authorship:
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  kind: seat
  public_key: UgGFd6fs0Ln9gJXlN2fw8kUmonpBkr4IBn/U5N0uOrY=
  process_generation: '1'
provenance:
  digest: sha256:41d84e3d9d5333c7bdb2be1b31ee637d23ff70079af4945e467abd45b5ad2733
  frontmatter_digest: sha256:6e662f13e2adc8cebb2c7c44437bff4ed1b3c02658ed712361d52f4dcc0a9383
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  sig: 87ORMAmc2N/xflR2oD8uJvqzJGsEjhii16+vRNvRimvbZ9UfnFQJt7FjTePOV2SLkE+mMmKVrPOqF+qRHkEcDw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/149458
  event:
    id: 01M6RNJ38T910TKCZV9RR3XXH6
    sequence: 149458
    hash: 873b9c49be35442cd2796e3ede328614b1f0e6f9b539dd8738076549a80c18c5
---
# One closure codec, preserved runtime limits

The operator requested landing and cleanup on2026-11-06. The Architect's reported A/B work is useful, but the remaining C literal-schema completion did not run: its seat reports a weekly quota limit. Root will not leave both source owners waiting on that seat or make them guess two formats.

Decision65962's candidate-complete/platform-trust boundary remains the design. Complete it through ONE producer-owned interface artifact before implementing clients: the builder owns tests/fixtures/dream-proof-tools/closure-boundary.schema.json and closure-boundary-vectors.json in its existing allowed tree. The exact typed schema, canonical valid/invalid bytes and hashes are the single C codec authority for builder and Q2. It must be written, checked and signed before either emitter or consumer builds against it; the builder may then continue its emitter without waiting for an acknowledgment. One source/hash handoff goes to Q2 and Root. Root independently reviews it and synchronizes the KB interface, but no second schema or controller is created.

This is an explicit assignment of routine wire-shape completion inside the already chosen boundary, not permission to redesign proof, budgets, source scope or done. The following requirements bind that artifact:

- Preserve the EXISTING Platform field names arch,darwin_machine,release,os_build,shared_cache_uuid; add required premise exactly trusted_platform_boundary/1. The shorthand darwin/cache_uuid in the earlier C prose is not a second accepted spelling. Build.platform and Closure.platform must be exactly equal; canonical arch stays aarch64 and only the already specified raw Darwin arm64 alias is admitted.
- Replace the conflicting Platform/Closure/Node/Macho/load-target/static/link-map pseudocode with complete strict definitions: exact required keys/types/discriminants/nullability, no unknown or duplicate fields, canonical integer/UTF8/JSON grammar and bounded arrays/depth. Retain the private format identifier and public worker wire/version. The new canonical bytes/projection hashes invalidate all old pending tuples.
- Candidate images remain complete. Record every raw LC load command in original order WITH multiplicity, mode load/weak/reexport/upward/lazy, exact rpaths/UUID/file/source bindings and its unambiguous candidate-ID OR bound-cache target. Duplicate raw load commands may be real; do not discard them as duplicate graph edges. Every nonplatform image must be within the admitted prefix and current file pins; unresolved/foreign/noncache images and direct root libobjc-env remain Unsupported.
- Static records must identify the actual archive, selected-member count and digest, the owning tool/output and immutable link-map Ref. The schema must make per-tool selection unambiguous if one archive participates in both tools. Define exact member-hash byte framing, order, names/delimiters, empty/repeated-member handling and map/input/output cross-binding. No fabricated static reachability. All selected linked code remains covered by the explicit source review; unused Libksba may remain build-only but its source/auth/generated evidence is still required.
- Platform-internal nodes, per-image OS facts and platform-to-platform edges stay outside prepared metadata as the explicitly trusted OS/cache boundary, not a hidden external authorization catalogue. Referenced raw source/archive/config/member/map evidence remains offline review provenance; define which bounded refs/digests are prepared, reserve all actual copies and do not read an uncharged catalogue at runtime. Before-birth/after-settle OS/build/arch/cache checks are observations, not proof against every unobserved transient change or arbitrary child mapping.
- Preserve64nodes/64array/128edge/depth10/Basis48KiB/aggregate128KiB/descriptor16KiB/lock32KiB/result64KiB/receipt32KiB and all input/CPU/debt/thread/process/heap/control/foreground/settlement gates. No new source, dependency, approval flag, native class, helper, tracer, service or global allocator. Fit of the final actual pair remains UNKNOWN until a real correctly bound emit; exceeding a cap stays Unsupported.
- Goldens and causal tests cover exact canonical roundtrip, both platform spellings, field/discriminant/nullability failures, unknown/duplicate/unreachable candidates, dropped/reordered/repeated loads, changed mode/UUID/source/file/map/member count/hash, foreign/noncache/rpath target, direct libobjc-env, stale platform/ABA, partial publication and every byte/count bound. These are codec/source controls, not a real Git/GPG qualification or invisible-work theorem.

Both live implementation briefs may consume this SAME signed artifact within their existing allowed fixture/read/model paths; this is advance scope authorization for that bounded interface adoption, not runtime candidate approval. Q2 may not silently derive a competing shape from the old prose. Source/checks may progress independently, while the final coherent Q2 source/executable/compiler pin still goes through Root before the real builder component witness. The old5493 stub and moving source are never substituted.

The builder may implement/schema-test and retain a C projection from existing authenticated metadata now, clearly incomplete and unapproved. It may not mint a fabricated full Basis/probe pin, re-run a lucky native qualification or call the old failed source7 gate green. Full real candidate emission/qualification follows the frozen current Q2 input and remains under the existing exact task gates. C1 source landing does not depend on this Q2 delivery.
