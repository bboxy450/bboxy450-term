---
type: task
title: Adopt measured hashbrown lookups and bounded KB search projections
verify: make test-hashbrown
allowed_paths:
- Cargo.toml
- Cargo.lock
- ATTRIBUTION.md
- Makefile
- crates/bbt-mux/
- crates/bbt-render/
- crates/bboxy450-term/src/scene.rs
- crates/bboxy450-term/src/scene/
- crates/bboxy450-term/Cargo.toml
- crates/bboxy450-term/tests/formal_contracts/
- verification/
- scripts/
- .bboxy/
- crates/*/ATTRIBUTION.md
- plugins/*/ATTRIBUTION.md
- release-inputs/
- crates/bboxy450-harness
- crates/bboxy450-harness/tests/gui_render.rs
- crates/bboxy450-harness/.bboxy/runs/2026-11-04-render-init.json
criteria:
- Hashbrown is adopted only in unordered hot lookups; ordered/public outputs and HashDoS-sensitive stores preserve their contract.
- KB normalized-text and graph-score projections preserve search matches, ranking and cursor semantics, remain root/snapshot scoped and respect a fixed memory budget with uncached fallback.
- Actual provider API tests cover edits, deletion, rename, clone/reopen, Unicode, graph weights/self-links/duplicate IDs, low-budget fallback and negative controls.
- Bounded lifecycle model and broken controls pass native/Java parity; targeted unit/integration and smoke checks run unattended, signed landing gates pass.
- Record source-bound performance comparison and full committed report; land signed changes through Make.
id: 01M6KT39X6DZNJVEWNP4Z11Y23
status: reported
delegated_by: seat:01M6KN12436S0N6CJDQ0RWBQNG
swarm: 01M6CQARWH5A79TE3H4NNWJ4XG
updated: 2026-11-04T22:17:48.481Z
created: 2026-11-04T17:35:52.040Z
number: 93377961361086
created_by: seat:01M6KN12436S0N6CJDQ0RWBQNG
updated_by: seat:01M6KN12436S0N6CJDQ0RWBQNG
authorship:
  signer: seat:01M6KN12436S0N6CJDQ0RWBQNG
  kind: seat
  public_key: 4FB10dq226S4+Z6CUKQrEvBUsQCbt4dAXA3Nd2bPO6o=
  process_generation: '1'
role: solo
assignee: seat:01M6KN12436S0N6CJDQ0RWBQNG
artifacts:
- '[[01M6KW3FCFEZ2VJ34M2QKENTNN]]'
verify_result:
  command: make test-hashbrown
  exit_code: 0
  evidence:
  - criterion: Hashbrown is adopted only in unordered hot lookups; ordered/public outputs and HashDoS-sensitive stores preserve their contract.
    reference: reports/2026-11-04-hashbrown-search-delivery.md; runs/2026-11-04-hashbrown-delivery.json — source/API/negative-control evidence, final source 5a11647087f9e1a8af8acb37011804cba0d2ac67.
  - criterion: KB normalized-text and graph-score projections preserve search matches, ranking and cursor semantics, remain root/snapshot scoped and respect a fixed memory budget with uncached fallback.
    reference: reports/2026-11-04-hashbrown-search-delivery.md; runs/2026-11-04-hashbrown-delivery.json — source/API/negative-control evidence, final source 5a11647087f9e1a8af8acb37011804cba0d2ac67.
  - criterion: Actual provider API tests cover edits, deletion, rename, clone/reopen, Unicode, graph weights/self-links/duplicate IDs, low-budget fallback and negative controls.
    reference: reports/2026-11-04-hashbrown-search-delivery.md; runs/2026-11-04-hashbrown-delivery.json — source/API/negative-control evidence, final source 5a11647087f9e1a8af8acb37011804cba0d2ac67.
  - criterion: Bounded lifecycle model and broken controls pass native/Java parity; targeted unit/integration and smoke checks run unattended, signed landing gates pass.
    reference: 'reports/2026-11-04-hashbrown-search-delivery.md — PARTIAL/UNMET: parent full gate and smoke red; no parent landing; runs/2026-11-04-hashbrown-delivery.json records actual exits.'
  - criterion: Record source-bound performance comparison and full committed report; land signed changes through Make.
    reference: 'reports/2026-11-04-hashbrown-search-delivery.md — PARTIAL/UNMET: parent full gate and smoke red; no parent landing; runs/2026-11-04-hashbrown-delivery.json records actual exits.'
report:
  summary: 'Implemented bounded KB search projections and borrowed glyph/text-width hashbrown lookups; signed local branch perf/hashbrown-search-20261104, committed report 6d83c3b3208cc6f6704af36dd7cdd5db2a818bc6. Final make test-hashbrown exits 0 (27 tests and real red/green digest mutation); make test-formal/check-formal-evidence exit 0 (383 models, 159 contracts, 34 crates). Parent landing remains blocked: full gate has 17 failures and private smoke fails idle callbacks. Latest upstream still has a proven missing-task record blocker. Criteria 4/5 landing portions are unmet; do not accept. Standalone producer repair afe92da is separately green and landed; parent changes are unpushed.'
  commit: 6d83c3b3208cc6f6704af36dd7cdd5db2a818bc6
  reported_by: seat:01M6KN12436S0N6CJDQ0RWBQNG
  op_id: 01M6MA7HA1GRXVQ52A7CBM2TWD
  verify_revision: 0
  report: '[[reports/2026-11-04-hashbrown-search-delivery]]'
provenance:
  digest: sha256:8cdf7b81f41701feaeef8f823c41f2d0f9b03af99dd5ce8373d03463439feaaf
  frontmatter_digest: sha256:f7f6e5f5d2ced561f7bcc724d82609438e9ace32d84c662e8f827f7e9db9c4de
  signer: seat:01M6KN12436S0N6CJDQ0RWBQNG
  sig: /IjLIE2NW5B2vUM/XlGMoM2xx2i9E2UNtoGyqhWXMEVKR54YsNz4aSpQYz8+zopPnmn7VcLp+SzeJMWEsuFFDg==
  log: .bboxy/log/2026-11-04.jsonl#130904
---
ROLE: SOLO — Vesper Greywind; direct operator implementation request, 2026-11-04.

Worktree: /Users/feral/mydev/bboxy450-term-vesper-hashbrown; branch perf/hashbrown-search-20261104. Implement the operator's request to adopt hashbrown wherever justified by the preceding research. Own this task alone, without spawning or delegating to other seats. Land verified changes under repository autonomous authorization.

Preserve ordered document storage, prefix/range reads, deterministic paging and exact score accumulation. Add root/snapshot-local bounded normalized search text and precomputed graph degrees, with exact fallback when a cache entry cannot be admitted. Use hashbrown for measured temporary graph lookups and hot renderer/metrics lookup improvements where semantics are unchanged; avoid a mechanical map sweep and keep random collision-resistant hashing at exposed key boundaries. Preserve fresh-source number uniqueness and resolver ambiguity checks.

Required checks: make test-hashbrown (targeted API/unit tests and model controls), current-source formal native/Java gate, license checks, affected smoke, canonical full test for changed Cargo.lock, then make land. Add the minimal missing Make workflows. Performance evidence must call production search/cache paths, assert output equivalence, report cache bytes and avoid claiming extracted-core timings as app latency.

Stop scope: no credential changes, release/publish, live operator app/UI manipulation, harness source changes beyond the gate repair below, unrelated cleanup or worktree removal. Keep peers' work untouched. Write and commit .bboxy/reports/2026-11-04-hashbrown-search.md with source/command/exit evidence and a Report trailer.

Mechanical license-policy copies under crates/*/ATTRIBUTION.md, plugins/*/ATTRIBUTION.md and release-inputs/ are in scope through make sync-licenses/sync-release-inputs; the pinned harness keeps its own independent attribution. Temporary graph alias maps remain ordered because the measured direct map swap was small/noisy; adoption targets new KB projections and borrowed text cache lookups.

Canonical full gate disposition, 2026-11-04: the rebased upstream harness pin a1ad9fbd (and current harness origin/main 3f151e4) contains a strict-Clippy field_reassign_with_default error in tests/gui_render.rs::gui_polish_loader_is_centered_and_pending_bubbles_are_scoped. Permit the solo seat to fix only that initializer in its own nested harness checkout, record JSON evidence, sign and land the producer repair under the harness repository's standing autonomous authorization, then update the parent pin and lock through make bump-harness REV=<verified producer SHA>. No new seats, runtime behavior change or crate release. Read harness instructions/spec, run its canonical unattended verification/formal/smoke plus the parent full gate. Preserve all upstream commits; no force push. This explicitly amends the earlier harness-edit stop line only for this gate repair.

The harness canonical formatter additionally found only onboarding/provider module declaration order in src/lib.rs. Include that formatting-only correction in the same signed producer repair; no runtime or protocol change. The harness canonical make test remains required.

Gate-repair scope clarification: the upstream canonical harness gate carries a small cluster of pre-existing formatter/test-structure lint issues, not just one initializer. Authorize only behavior-preserving formatting, declaration/test-module ordering and test-initializer corrections that its canonical checks name (including src/frontends/gui/zoom.rs::load_theme placement). Do not change runtime behavior, protocols, dependency versions or weaken any lint/check. Keep all producer repairs in the same signed cut and record each diagnostic. This replaces the prior one-file repair restriction while retaining every required verification gate.

The behavior-preserving gate-repair category includes test-code lint idioms named by the canonical all-feature pass (tests/gui_rich.rs constant RGBA chunks_exact -> as_chunks iterator), in addition to formatting/module ordering/initializers. Runtime behavior, test assertions, dependency versions and lint/check strength remain fixed.

The canonical test run reached the remote-profile API fixture and exposed a host-PATH dependency introduced by upstream ACP setup discovery: attach_setup seeded installed operator adapters into the private catalogue, breaking its exact eleven-profile assertion. Permit a test-only private onboarding Controller with an empty discovery PATH, registered at the fixture's unique profiles directory before connection. Preserve every existing assertion and runtime path. This designs out operator-machine dependence; no global environment mutation or runtime change. The strict Clippy commands may use --keep-going solely to gather complete diagnostics; warnings still fail.

Canonical gate repair extension: current upstream protocol.schema.json advertises SetAgentConfig with five newly required fields while verification/protocol-v1.lock.json lacks the variant. This violates log-ir-v1 E1/E2 and breaks three IR proposal tests before formal/smoke. Repair only absence defaults on this new operation: zero epoch/revision, a deterministic nil child sentinel and empty selector/value, all rejected by the existing ACP identity/options guards. Preserve every valid request and all guard logic, prove every missing-field case causes OpRejected before any child wire call using the existing private process fixture, regenerate the schema and refresh the lock only through the existing additive Make workflow (which must itself pass unchanged). Map the parsing boundary to the existing ACP controls contract and implementation witness; no validator weakening, frozen-corpus edit, provider behavior change or dependency change. This narrow specification-conformance repair amends the runtime/protocol stop solely for missing-field decoding; full canonical native/Java/unit/integration/GUI/smoke checks remain mandatory. Keep remaining unrelated failures out of scope and report them instead of indefinitely extending baseline repairs.

The generated current-source schema (unlike its stale checked-in snapshot) exposes the same E2 defect on QueryInputCapabilities.request and InputCapabilities.{request,image_custody_version}. Include only these three fields in the same absence-default repair: nil challenge and zero custody version, omitted when absent/default. Use the existing actor-intake witness to prove missing challenges/versions cannot grant custody, and preserve every current challenge/epoch/durability guard. This is the same finite IR-compatibility family; remaining unrelated failures still stop the repair.

The ACP/test-fixtures variant exposed the same operator-PATH contamination in tests/acp_controls.rs::acp_controls_default_home_path_is_selected_in_private_child: discovery could create the deliberately missing catalogue before the assertion. Supply an empty PATH only to its already-private child process (the test executable is absolute and starts no agent), retaining its present/absent/empty/missing/unreadable assertions. This is the existing private-fixture isolation repair; no global environment or product behavior changes.

Replay diagnostic identified a bounded product regression from the new capability query, rather than a replay-data mismatch: a Unix replay-only/old actor's refusal of QueryInputCapabilities enters the user-dispatch error lane. The transcripts are equal, but the unrelated discovery error prevents healthy replay parity. Permit a narrow response-classification fix in ViewGateway::send: query admission outcomes never enter user-intent outcomes; only the existing current challenge/epoch/live actor event can grant custody. This matches the existing in-process ignored query result and handles old-query completions across reconnect without tracking an unbounded nonce set. Preserve other user/control outcomes. Extend HarnessAttachmentCapability with the discovery-completion stutter and an independent broken discovery-error control; map GUI replay and actor-proof witnesses. No other runtime/gate expansion. Verify unchanged twelve-fixture local/Unix/replay equality, current-source native/Java controls and the complete stack.

The unchanged canonical formal validator additionally exposes four upstream declaration omissions in these affected/new families: ACP controls uses tests instead of implementation_tests, both attachment families are absent from crate coverage, and ACP/attachment/onboarding check lists omit the required canonical pair. Correct only those source/test/check declarations while registering the independent BrokenHarnessAttachmentDiscovery control. Do not relax the validator or omit any model. Replay parity red is private nextest exit100 on producer ef003a6; the response-classification guard makes the unchanged twelve-fixture test exit0. No diagnostic test edits remain.

Compatibility edge refinement before producer landing: an ACP provider can advertise the empty string, so SetAgentConfig.value must preserve absence as Option<String>, rather than default absence to an executable empty choice. Adapt the existing GUI/fixture constructors and the private configure admission to require Some(value) under unchanged identity/advertisement fences; valid wire strings, including Some(empty), remain valid. Extend the same real child regression to deny omitted/null values even when empty is advertised and prove explicit empty succeeds. Rebuild the still-unpublished IR lock from its unchanged authoritative pre-repair 3f151e4 baseline through make lock-ir; no released baseline or checker is weakened. The interrupted canonical run restored every mutation source before changes and is not final verification. Native canonical source mappings also include the three previously omitted attachment implementation files.

Final canonical record-check repair: the historical unsigned decisions/d5974-log-ir-v1-only-add.md and signed d5974-memories-are-kb-records.md both declare 5974, a previously recorded failure also present before this task. All 132 native/Java model graphs/verdicts passed before this static record gate. The available MCP targets only the parent root and offers no renumber operation; Index::assign_number explicitly refuses writes to an existing conflicting number and requires record repair. Authorize a one-line metadata repair of only the unsigned legacy IR decision in the exclusive nested checkout: assign max(existing physical Markdown numbers)+1, retain its ID, path, exact body and all link targets; leave the signed memory record byte-for-byte unchanged. Record old/new number, body/record hashes and command evidence in the producer report. This is an explicit legacy-file repair, not a claim of a native child-root write. No checker exception, document deletion, signature rewrite or decision-content change. Re-run the unchanged canonical gate with the source byte caches warm.

The complete physical-record audit found four additional collisions in that same imported catalogue (5972, 5975, 5978, 5981), with no duplicate IDs. Each pair has one historical unsigned record and one signed record. Extend only the legacy catalogue-number repair to the four unsigned records; two are unmanaged historical task documents, without native assignee/delegator/authorship. Their accepted/open status, original brief, verify, criteria, dates, ID, path and exact body remain frozen; no native task lifecycle/amend/acceptance action is taken. Assign unique lookup numbers and retain the signed counterparts byte-for-byte. This is a repository source-catalogue migration, not a re-brief or change to any historical verification claim. Commit an old/new number and SHA mapping in the producer evidence. The unchanged whole-corpus validator and broken control must both pass.

The GUI gate reaches six menu-motion assertions whose shared fixture explicitly leaves a reply unprepared while claiming no non-menu motion. Upstream now correctly animates pending row loading, so its clock remains active. Correct only tests/gui_menu_motion.rs::state to prepare the already-completed reply through the production Preparer before the clock witness; retain all seven assertions, timings, menu geometry and production loading behavior. This is an existing fixture-state correction, not a product animation change. Verify the full GUI gate and private packaging/smoke before final canonical producer rerun.

Motion-fixture consistency: gui_menu_motion uses a prepared static Notice with no transcript source-hover zones, so existing empty-hover/menu-only assertions remain exact; preparation runs on the production worker before the hand-stepped clock. The next GUI target shows two gui_motion empty-session witnesses omit SessionMeta, which upstream correctly treats as loading. Supply a matching synthetic SessionMeta in tests/gui_motion.rs::transcript to represent the settled session they assert. Preserve every motion/geometry/hit-map assertion and duration; no product loading or animation changes.

Renderer gap repair (tests remain unchanged): two existing gui_render witnesses require current-birth queued prompts to remain visible immediately and preserve history pixels while Markdown preparation is delayed. The new Markdown-only slot is an empty Spacer until worker data arrives, so the failure is real. Restore a stateless plain-text gray preview only for missing prepared pending rows, bounded to 160 Unicode scalars plus an ellipsis, with the full original text retained for worker formatting. Reuse existing bubble chrome, current session/epoch filter and fixed sibling heights; never parse Markdown or perform IO in the frame. Prepared rows keep their current path. Extend the native renderer witness for bounded Unicode preview/full-source retention and map this pure presentation gap in the existing GUI pending contract; no new delivery, ownership, authority or lifecycle state. Do not replace the deliberate preparation-gap tests with eager fixture preparation.

Packaging fixture correction: tests/app_packaging.py probes BUNDLE through inherited MAKEFLAGS, then asserts the default relative target even when canonical TARGET_DIR is overridden. Make's override works correctly. Probe TARGET_DIR=target explicitly for the existing exact default assertion and add an exact custom-path-with-spaces assertion. Keep every signing/install/launcher refusal and preservation check intact; product packaging unchanged.

Parent gate output custody: the newly pinned harness tracks gui-smoke-scrolled.png as well as gui-smoke.png. Include it in scripts/harness-submodule.py's existing known-output archive/restore list so a successful standalone test leaves the pinned source clean. Preserve both actual images in gate artifacts; no source reset or gate suppression.

Root formal inventory completion: register the six new KnowledgeSearch projection/budget/progress good/broken rows in verification/corpus.toml. Canonical formal self-tests expose four pre-existing records containing two name keys: WatcherSettle/RichHost, BrokenWatcherSettle/BrokenRichHost, WatcherChanges/GuiBackendPreservation, and BrokenWatcherAdmission/BrokenGuiBackendPreservation. Split each into its two declared model records while retaining their existing verdict/frontend; no model/validator weakening. This is the required source/model catalogue repair within verification scope. Preserve and report the 17 separate workspace failures; do not broaden into UI/keyboard/MCP or historical record-link repairs.

The whole root contract-declaration audit finds exactly three mandatory-check omissions: WatcherAdmission has none; TieredSeatRecovery and TaskPanelRetry list focused checks without make test. Complete the unchanged validator's exact canonical pair on those declarations, retaining the two existing lists separately as focused_checks. No model, witness, verdict or validator changes; no broad code repair. Run the inventory self-tests before retrying formal.

Final inventory disambiguation: the two-name BrokenWatcherSettle/BrokenRichHost block attached a single safety expectation to both names. Existing watcher_model.rs::cases (line 120) and WatcherSettle's eventual-settlement contract specify liveness for BrokenWatcherSettle; retain safety for BrokenRichHost and restore liveness for the watcher row. The native export now matches all 383 required names and expectations. This restores the pre-existing watcher specification, without changing any model or validator. TaskPanelRetry's already implemented good/safety pair is also registered to complete the audited corpus.


## Report from seat:01M6KN12436S0N6CJDQ0RWBQNG

Implemented bounded KB search projections and borrowed glyph/text-width hashbrown lookups; signed local branch perf/hashbrown-search-20261104, committed report 6d83c3b3208cc6f6704af36dd7cdd5db2a818bc6. Final make test-hashbrown exits 0 (27 tests and real red/green digest mutation); make test-formal/check-formal-evidence exit 0 (383 models, 159 contracts, 34 crates). Parent landing remains blocked: full gate has 17 failures and private smoke fails idle callbacks. Latest upstream still has a proven missing-task record blocker. Criteria 4/5 landing portions are unmet; do not accept. Standalone producer repair afe92da is separately green and landed; parent changes are unpushed.

```json
{
  "op_id": "01M6MA7HA1GRXVQ52A7CBM2TWD",
  "commit": "6d83c3b3208cc6f6704af36dd7cdd5db2a818bc6",
  "report": "[[reports/2026-11-04-hashbrown-search-delivery]]",
  "verify_result": {
    "command": "make test-hashbrown",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Hashbrown is adopted only in unordered hot lookups; ordered/public outputs and HashDoS-sensitive stores preserve their contract.",
        "reference": "reports/2026-11-04-hashbrown-search-delivery.md; runs/2026-11-04-hashbrown-delivery.json — source/API/negative-control evidence, final source 5a11647087f9e1a8af8acb37011804cba0d2ac67."
      },
      {
        "criterion": "KB normalized-text and graph-score projections preserve search matches, ranking and cursor semantics, remain root/snapshot scoped and respect a fixed memory budget with uncached fallback.",
        "reference": "reports/2026-11-04-hashbrown-search-delivery.md; runs/2026-11-04-hashbrown-delivery.json — source/API/negative-control evidence, final source 5a11647087f9e1a8af8acb37011804cba0d2ac67."
      },
      {
        "criterion": "Actual provider API tests cover edits, deletion, rename, clone/reopen, Unicode, graph weights/self-links/duplicate IDs, low-budget fallback and negative controls.",
        "reference": "reports/2026-11-04-hashbrown-search-delivery.md; runs/2026-11-04-hashbrown-delivery.json — source/API/negative-control evidence, final source 5a11647087f9e1a8af8acb37011804cba0d2ac67."
      },
      {
        "criterion": "Bounded lifecycle model and broken controls pass native/Java parity; targeted unit/integration and smoke checks run unattended, signed landing gates pass.",
        "reference": "reports/2026-11-04-hashbrown-search-delivery.md — PARTIAL/UNMET: parent full gate and smoke red; no parent landing; runs/2026-11-04-hashbrown-delivery.json records actual exits."
      },
      {
        "criterion": "Record source-bound performance comparison and full committed report; land signed changes through Make.",
        "reference": "reports/2026-11-04-hashbrown-search-delivery.md — PARTIAL/UNMET: parent full gate and smoke red; no parent landing; runs/2026-11-04-hashbrown-delivery.json records actual exits."
      }
    ]
  }
}
```
