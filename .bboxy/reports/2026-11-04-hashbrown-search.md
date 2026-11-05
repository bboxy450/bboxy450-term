---
type: report
title: Hashbrown and bounded KB search — focused verification
task: '[[tasks/2026-11-04-hashbrown-search]]'
commits: []
verify:
  command: make test-hashbrown
  exit_code: 0
author: seat:01M6KN12436S0N6CJDQ0RWBQNG
created: 2026-11-04T18:27:01.908Z
id: 01M6KX0ZAMY8SQKG37HRM2XMH0
updated: 2026-11-04T18:27:01.908Z
number: 93377961362687
created_by: seat:01M6KN12436S0N6CJDQ0RWBQNG
updated_by: seat:01M6KN12436S0N6CJDQ0RWBQNG
authorship:
  signer: seat:01M6KN12436S0N6CJDQ0RWBQNG
  kind: seat
  public_key: 4FB10dq226S4+Z6CUKQrEvBUsQCbt4dAXA3Nd2bPO6o=
  process_generation: '1'
provenance:
  digest: sha256:a49d826c560246a292038d78435545fe4cd12f7c105d5b46705165ad73e3271c
  frontmatter_digest: sha256:724c6c1f5630e0784d9d49a4c44b759e53f92700c9fd8a10fecbd22e1506d6ae
  signer: seat:01M6KN12436S0N6CJDQ0RWBQNG
  sig: qScEsE1PAURpK6fJDLPapoNfl3lh0TBNsX9Y4R0qN5SxqPruYRY2CbDoNfYviqtjCqDvnF1VFXGimvH6xXEnDQ==
  log: .bboxy/log/2026-11-04.jsonl#127814
---
This is the focused verification report for [[tasks/2026-11-04-hashbrown-search]] and [[hashbrown-search]]. It is committed with the implementation; `commits: []` avoids a self-referential SHA. The signed containing commit identifies this cut. Full canonical CI, Java TLC parity, smoke and landing are pending and will be recorded in a superseding final report.

## Implementation and selection

- crates/bbt-mux/src/index/search.rs:14 — 16 MiB / 4096-entry normalized-text admission; counts the table, retained path/digest/text capacities and Arc value/container allocations. No table grows beyond the reserved entry bound. Empty/oversized entries fall back to the original algorithm.
- crates/bbt-mux/src/index/search.rs:88 and :120 — reuse/hits require the source digest; immutable Arc entries may be shared across snapshots. Current rebuilds cannot mutate a retained clone. Ordinary knowledge records are admitted before artifacts.
- crates/bbt-mux/src/index/search.rs:103 — stop admitting at a full budget instead of re-normalizing the uncached tail on every update. The native work-count regression checks reuse plus at most one new candidate for an unchanged full cache.
- crates/bbt-mux/src/index/search.rs:130 — derive Markdown graph degrees in original edge order using hashbrown entry_ref; duplicate IDs share totals, self-links count once and addition retains the existing f64::MAX cap.
- crates/bbt-mux/src/knowledge/bboxy.rs:415 and :944 — actual search/graph scoring reads the projection and uses exact JSON serialization/Unicode-lowercase fallback for uncached text. Filtering, tie order, snippets and cursor generation remain on the existing provider path.
- crates/bbt-render/src/cells/terminal/gpu.rs:40, :200 and :581 — borrowed glyph keys compare/hash exactly like owned keys; plain cell/complete-cluster text stays borrowed. New glyph misses own their key; sequences spanning parser cells allocate only when joined.
- crates/bboxy450-term/src/scene.rs:351 and :381 — borrowed string/font-width keys avoid hit-path string allocation; every font/style field stays in identity.
- Content-keyed hashbrown maps explicitly retain std::hash::RandomState. Existing ordered document/graph-kind maps and temporary graph alias tables remain ordered: prefix reads, stable snapshots/paging and the small/noisy direct graph-table comparison do not justify replacing them. Fresh-file number allocation/uniqueness, resolver ambiguity and authority checks keep their existing path.
- ATTRIBUTION.md and 33 canonical package copies credit the hashbrown API integration and preserve upstream terms. Cargo.lock adds only the three direct package dependency edges; no registry package versions change.

## Verification actually completed

- `make update-hashbrown-models` — exit 0. Three native model families and their broken controls are exported. Safety and liveness failure categories are checked separately.
- `make test-hashbrown` — exit 0; 26 selected positive tests: five mux tests (including the three model pairs), sixteen existing provider integration tests, four renderer tests (borrowed identity, parser flags/emoji and actual GPU prepared-frame witness), and one native width/style test.
- The same target removes the real production text-digest guard: Cargo exits 101 specifically at `SearchCacheDigestFence` in the actual BboxySource API comparison; restored Cargo exits 0. Source restoration SHA256: d7477716392b63d36f955a49262b1c302d5ee0862e54d09a1b151acfbbedca56. Raw red/green logs and command/exit record are in target/hashbrown-control/.
- Native API witnesses cover exact serialized pages and score bits, filters, Unicode, duplicate IDs, self-links/saturated weights, A/B/A, stale-cache fallback, updates/deletion/rename, old clones, reopen, separate roots, byte/entry budgets, oversized fallback, record priority and bounded rebuild normalization work.
- `make check-hashbrown` — final exit 0, strict all-target Clippy for mux/render/app. The first run exited 2 (Cargo 101) on an existing test-only evidence write at scene/navigation.rs:234. A scoped allow on that headless geometry test fixes the gate; runtime UI IO lint enforcement remains active. The native CoreText width test is macOS-only.
- `make test-licenses` — exit 0: package policy/notices and preserved upstream bytes held for 34 workspace crates, source notices held, six license fixtures and nine source-notice fixtures passed.
- `git diff --check` — exit 0.

## Performance evidence

`BBOXY_KB_PROFILE_CORPUS=<private frozen JSON> make profile-hashbrown` — exit 0. Actual provider search CPU, not the earlier extracted search core: hit construction, sorting, first-page cursor and response-size checks are included; transport/IO, admission/Index cloning and UI are excluded. The test asserts identical serialized pages against the provider's original uncached/edge-scan fallbacks.

Frozen corpus: 23,126 records including 22,597 artifact records; SHA256 f6e08d9fa9b65ecb368dabc55cfb25370954df162e5474c5a27c39bde7bf9cb2. Apple M4, shared-machine load 11.60/16.64/16.30 before the probe; canonical optimized test profile (opt-level 1, debug 0). Rotating paired sample order, nine query samples, no timing pass threshold.

| Query | Uncached median | Bounded projection median |
|---|---:|---:|
| index | 138.65 ms | 111.70 ms |
| test | 604.86 ms | 123.27 ms |
| missing-private-query | 152.28 ms | 115.79 ms |

4,096 records cached; 15,154,191 retained cache bytes (14.45 MiB), below 16 MiB. Cold graph/projection rebuild was 115.56 ms in this probe. Degree-map/Index allocations, allocator metadata, transient normalization and multiple snapshot lifetimes are outside the normalized-text budget.

The isolated glyph-key benchmark uses the production key types/table/hasher: 100,000 owned-key lookups median 5.898 ms versus borrowed-key 2.598 ms, eleven samples. It measures key construction/lookup CPU, not GPU, frame rate or application latency. The native width test proves hash/style identity and bit-identical repeated measurements; no separate width performance claim.

Exact commands, log hashes, runtime/profile source hashes, machine/corpus bounds and aggregate results: .bboxy/runs/2026-11-04-hashbrown.json. Raw logs and corpus remain outside Git.

## Remaining gates and limits

Run the canonical full stack and current-source native/Java formal gate for the Cargo.lock change, then signed landing through Make. Native-only model success is not yet Java parity. Full CI/smoke and landing are not claimed by this focused report. The finite models abstract hashing, Unicode/JSON, floating point, actual allocation, arbitrary direct public-field mutation, filesystem/OS scheduling and arbitrary Rust refinement; implementation witnesses cover the listed concrete cases. No global cache or query lock is added.
