---
type: report
title: Hashbrown implemented and formally checked; parent landing blocked
task: '[[tasks/2026-11-04-hashbrown-search]]'
supersedes: '[[reports/2026-11-04-hashbrown-search]]'
commits:
- b3c396d76f563d411a2fe86eba24adccaa0fb2ab
- f8fb79710b15c9bb637e03bb43a40b05a7a53194
- 287a66a07b4a1e4e43b8133233c6ff5fc1df5393
- 5ae09f773eb15e44e8d145964e781ace3ad56f56
- 3ae0c42eab0a3ac49153c6095a4366ea325a798f
- 5a11647087f9e1a8af8acb37011804cba0d2ac67
verify:
  command: make test-hashbrown
  exit_code: 0
author: seat:01M6KN12436S0N6CJDQ0RWBQNG
created: 2026-11-04T22:17:22.786Z
id: 01M6MA6QJCEEW13FSFQQB2018B
updated: 2026-11-04T22:17:22.786Z
number: 93377961363617
created_by: seat:01M6KN12436S0N6CJDQ0RWBQNG
updated_by: seat:01M6KN12436S0N6CJDQ0RWBQNG
authorship:
  signer: seat:01M6KN12436S0N6CJDQ0RWBQNG
  kind: seat
  public_key: 4FB10dq226S4+Z6CUKQrEvBUsQCbt4dAXA3Nd2bPO6o=
  process_generation: '1'
provenance:
  digest: sha256:f735fc04e4e332c77c5a2a7fed7a78154195303745c306b72fbc18bf73ddb531
  frontmatter_digest: sha256:e6b7fe29f8c54c4fac272804b945704f82264b0ca7a2698e9e01c1f13374ef62
  signer: seat:01M6KN12436S0N6CJDQ0RWBQNG
  sig: teix29ROIM0eNtAxmdW27nRd9P5ZGbnuYN5eDtUkp3BdCDJPyr9sMSxFRL5DkWKhcNSh22feUS0i2GWmuTGMAQ==
  log: .bboxy/log/2026-11-04.jsonl#130898
---
Implemented for [[tasks/2026-11-04-hashbrown-search]], with the measured selection in [[hashbrown-search]]. The parent task is incomplete: source is signed and committed locally, but the full landing gate and parent smoke are red. It has not been accepted or pushed to parent main.

Worktree /Users/feral/mydev/bboxy450-term-vesper-hashbrown; branch perf/hashbrown-search-20261104. Final tested source is 5a11647087f9e1a8af8acb37011804cba0d2ac67. Its six signed source commits are listed in frontmatter. This supersedes the earlier focused report; the containing commit adds records only.

The selected implementation is:

- crates/bbt-mux/src/index/search.rs:14 — hashbrown maps with RandomState hold immutable snapshot-local text and graph projections. Retained normalized-text admission is limited to 16 MiB and 4,096 records, including table/path/digest/string capacities and Arc allocations. Ordinary records precede artifacts; misses use the old exact algorithm. Degree-map allocations, transient normalization, allocator metadata, other Index data and multiple retained snapshots are outside this text budget.
- crates/bbt-mux/src/index/search.rs:120 and index.rs:703 — source digests fence reuse/hits; snapshots share immutable entries and rebuilds publish fresh projections. Degree scores preserve original edge order, duplicate IDs, once-per-ID self-links and the existing f64 cap. The real provider in knowledge/bboxy.rs consumes these projections without replacing filtering, ranking, snippets or cursor logic.
- crates/bbt-render/src/cells/terminal/gpu.rs:40 and crates/bboxy450-term/src/scene.rs:349 — borrowed Equivalent keys remove owned-string construction from warm glyph and text-width hits. Sprite, style, span and font identity remain in the keys; misses retain ownership normally. Content-keyed maps retain randomized collision-resistant hashing. Ordered documents, prefix/range reads, stable output and small/noisy graph alias tables retain their existing structures.
- Cargo manifests/lock, ATTRIBUTION.md and canonical package copies — pinned hashbrown 0.17.1 integration and attached API/source credit; only three direct dependency edges were added to the existing registry resolution.
- crates/bbt-mux/src/index/search/tests.rs and src/models/search.rs, generated models, Makefile and scripts/test-hashbrown-control.py — real provider equivalence/lifecycle/budget witnesses, bounded projection/budget/progress good and broken models, and a production digest-fence mutation.
- verification/{corpus,contracts,coverage}.toml — register the new six models and synchronize the parent with its harness pin. Canonical checks also exposed malformed upstream two-name blocks, three incomplete mandatory check declarations and the missing existing TaskPanelRetry pair. Four collided pairs are split; BrokenWatcherSettle retains the liveness expectation specified by watcher_model.rs:120 and its eventual-settlement contract; the RichHost control remains safety. Existing focused commands remain in focused_checks while mandatory checks retain the unchanged validator's exact pair. Thirteen imported harness model names were missing from parent coverage and are now registered. No model or validator was weakened.
- scripts/harness-submodule.py — archive/restore the producer's newly tracked scrolled smoke screenshot alongside its other generated outputs.
- The preceding standalone harness producer repair is documented independently in [[reports/2026-11-04-hashbrown-harness-gate]]. Signed producer afe92dae97b2a5f86209863fd1545d7208171b9b passed its complete private canonical make test and was pushed normally to its repository main. The parent pin was changed through make bump-harness. Standalone success does not claim embedded workspace success.

Actual final verification:

| Command | Actual result |
|---|---|
| make test-hashbrown | Exit 0 on 5a11647087f9e1a8af8acb37011804cba0d2ac67: 5 mux/cache/model tests, 17 provider integrations, 4 renderer tests and 1 native width/style test; 27 positive tests. |
| Production digest mutation, included above | Removing the actual guard exits 101 at SearchCacheDigestFence; restored source exits 0. Restoration SHA256 d7477716392b63d36f955a49262b1c302d5ee0862e54d09a1b151acfbbedca56. No operator input. |
| make test-formal | Final exit 0 on the same clean signed source: 383 models, 159 contracts, 34 crates; full native/Java reachable state and transition sets and safety/deadlock/liveness verdicts agree. All deliberately broken controls retain their expected failure categories. |
| make check-formal-evidence | Exit 0 after the production mutation was restored; receipt is current and clean for the tested source. |
| Signature checker over 2f4d2f135c4443128cb9f0998e7c4f781a8d5e60..HEAD | Exit 0; six approved signed source commits. git diff --check also exits 0. |
| Earlier make check-hashbrown / make test-licenses / make profile-hashbrown | Exit 0; focused report records exact commands/source hashes. The final workspace strict all-target Clippy also passed. |
| BBOXY_GATE_SLOTS=3 make land-full | Exit 2, internal landing refusal 77 on f8fb79710b15c9bb637e03bb43a40b05a7a53194. Build, strict Clippy and doctests pass; nextest: 2,894 passed, 17 failed, 228 skipped, 2,911 executed. No parent push. |
| make smoke | Exit 2, underlying 1: private unsigned app starts, certifies its throwaway identity and renders a headless frame. Idle probe fails at 0 frames / 1 callback over 10.005 s; orderly shutdown reports children reaped. No operator/Keychain/UI input. |

The six new model comparisons are independently recorded: projection good 432 states/1,938 edges; broken projection safety 800/3,556; budget good 4/11; broken budget safety 6/18; progress good 4/10; broken progress liveness 4/9. Models abstract hashing, Unicode/JSON, floating point, allocation, filesystem scheduling and arbitrary direct public-field mutation; provider/native tests cover the concrete listed cases. This is finite model parity, not a proof of arbitrary Rust or complete TLC semantics.

The cache API witnesses compare exact serialized pages and score bits to the original provider fallbacks for Unicode, filters, cursor paging, weighted/self/duplicate-ID links, edit/delete/rename, retained stale text, clones, reopen, distinct roots, A/B/A, low/zero budgets, oversize fallback, priority and bounded normalization work.

Performance remains the source-bound probe in .bboxy/runs/2026-11-04-hashbrown.json: frozen 23,126-record corpus, 22,597 artifacts, SHA256 f6e08d9fa9b65ecb368dabc55cfb25370954df162e5474c5a27c39bde7bf9cb2; Apple M4; opt-level 1/debug 0 test profile on a shared machine. Nine rotating paired samples assert equal actual provider output. Median CPU: index 138.65→111.70 ms, test 604.86→123.27 ms (about 4.9x), missing query 152.28→115.79 ms. Retained text was 15,154,191 bytes for 4,096 records; cold projection rebuild 115.56 ms. Actual BboxySource search includes hit construction, ranking and first-page cursor/size handling; request transport/IO, Index admission/cloning and UI are excluded. Glyph-key construction/lookup alone measured 5.898→2.598 ms per 100,000 hits (about 2.3x), eleven samples; no frame-rate or app-latency claim. Cache/glyph source files match their profiled hashes. The provider file gained an upstream write-path guard; its search method bytes remain identical (SHA256 d1c22d6f14bae4da594336b8193914c7a41b62f364c91d6c338f15c6ae1366de).

Landing blockers are preserved. The 17 failures include corpus links/report-task resolution, startup/table/pane layout, embedded harness chip/zoom/agent-caption assertions, keyboard ownership, an export-only test invoked without its required environment, theme/actor/text witnesses, and MCP write-prefix/migration fixtures. The JSON lists every failed test. Several failed-test/source files match the tested upstream base byte-for-byte. One blocker is proven upstream independently: .bboxy/reports/2026-11-04-ui-status-delivery.md references .bboxy/tasks/2026-11-04-ui-status-and-handoff-diagnostics.md, which is absent both from the tested base and newly fetched upstream 6d32bafd68eff7bb4417de4065771d1b7965d8e9 (git cat-file exits 128). Not every other failure's cause has been independently reproduced on clean upstream; none was declared a flake, quarantined or bypassed. Parent smoke's idle callback failure also remains unisolated.

All changes after the failed full workspace gate are task/formal inventory metadata; runtime source is unchanged. Latest upstream has since moved its harness pin and UI/license work, so a future landing requires a rebase and the appropriate fresh gates. This report does not authorize replacing that newer upstream work.

Acceptance disposition: criteria 1–3 have source/API evidence; criterion 4 has green focused/formal checks but unmet full-suite, smoke and signed parent landing gates; criterion 5 has measured/source-bound performance and committed reporting but unmet landing. The task remains unaccepted. Broader UI/keyboard/MCP and historical record-link repair is outside this scoped adoption; stop and report those failures as the brief requires.

[Aggregate commands, source/hash audit and all failures](../runs/2026-11-04-hashbrown-delivery.json) records real exits, private logs, source equivalence and upstream audit. Formal receipt SHA256 0f236bfd4bd8f918be31b5f85824595309b54cc6e69d2b66964b689589e83ba9 is retained with full evidence under /Users/feral/mydev/bboxy450-term/.git/bboxy-formal-evidence/0f236bfd4bd8f918be31b5f85824595309b54cc6e69d2b66964b689589e83ba9/. Final formal/focused/check/smoke logs and receipt are also retained at /Users/feral/.local/state/bboxy450-term/landing/hashbrown/5a11647087f9e1a8af8acb37011804cba0d2ac67/. Full failed-gate log remains in the landing presubmit store; raw logs/private corpus are outside Git. Worktree was clean after verification, before adding this report/summary; the report and task-status commits contain records only.
