---
type: task
title: Implement term own-seat post-session Dream C1 controls and worker custody
worktree: /Users/feral/mydev/bboxy450-term-dream-c1
role: executor
allowed_paths:
- crates/bboxy450-harness-wake-up/src/dream_c1.rs
- crates/bboxy450-harness-wake-up/src/lib.rs
- crates/bboxy450-harness-wake-up/src/launch.rs
- crates/bboxy450-harness-wake-up/tests/dream_c1.rs
- crates/bbt-proto/src/dream.rs
- crates/bbt-proto/src/lib.rs
- crates/bbt-proto/src/gui/dream.rs
- crates/bbt-proto/src/gui/mod.rs
- crates/bbt-proto/tests/dream_c1.rs
- crates/bbt-proto/tests/golden/gui_wire.tsv
- crates/bbt-mux/src/adapters/launch.rs
- crates/bbt-mux/src/adapters/mapping.rs
- crates/bbt-mux/src/adapters/mod.rs
- crates/bbt-mux/src/logic/dream_c1.rs
- crates/bbt-mux/src/logic/harness_status.rs
- crates/bbt-mux/src/logic.rs
- crates/bbt-mux/src/registry.rs
- crates/bbt-mux/src/registry/dream_c1.rs
- crates/bbt-mux/src/mcp/dream_c1.rs
- crates/bbt-mux/src/mcp/mod.rs
- crates/bbt-mux/src/mcp/tools.rs
- crates/bbt-mux/src/mcp/hints.rs
- crates/bbt-mux/tests/dream_c1.rs
- crates/bboxy450-term/src/ctl/dream.rs
- crates/bboxy450-term/src/ctl.rs
- crates/bboxy450-term/src/main.rs
- crates/bboxy450-term/tests/dream_c1.rs
- crates/bboxy450-term/tests/swarm_workdir.rs
- crates/bboxy450-term/tests/formal_contracts/dream_c1.rs
- crates/bboxy450-term/tests/formal_contracts/models.rs
- verification/corpus.toml
- verification/coverage.toml
- verification/contracts.toml
- scripts/test-dream-c1.py
- Makefile
- ATTRIBUTION.md
- crates/bboxy450-harness
- Cargo.lock
- reports/2026-11-05-dream-c1-consumer.md
- crates/bbt-mux/src/seat_socket.rs
- crates/bboxy450-harness-wake-up/src/rich.rs
- crates/bboxy450-usage/ATTRIBUTION.md
- crates/bboxy450-term-desktop-core/ATTRIBUTION.md
- crates/bboxy450-term-desktop-macos/ATTRIBUTION.md
- crates/bboxy450-term-desktop/ATTRIBUTION.md
- crates/bboxy450-keychain-macos/ATTRIBUTION.md
- crates/bboxy450-keychain-windows/ATTRIBUTION.md
- crates/bboxy450-keychain-file/ATTRIBUTION.md
- crates/bboxy450-keychain/ATTRIBUTION.md
- crates/bboxy450-desktop-notifications-macos/ATTRIBUTION.md
- crates/bboxy450-desktop-notifications-windows/ATTRIBUTION.md
- crates/bboxy450-desktop-notifications-linux/ATTRIBUTION.md
- crates/bboxy450-desktop-notifications/ATTRIBUTION.md
- crates/bbt-plugin/ATTRIBUTION.md
- crates/bbt-plugin-sdk/ATTRIBUTION.md
- crates/bbt-plugin-fixture/ATTRIBUTION.md
- plugins/gitkb/ATTRIBUTION.md
- crates/bboxy450-tlc/ATTRIBUTION.md
- crates/bbt-yarn-macros/ATTRIBUTION.md
- crates/bbt-yarn/ATTRIBUTION.md
- crates/bbt-core/ATTRIBUTION.md
- crates/bboxy450-harness-wake-up/ATTRIBUTION.md
- crates/bbt-mermaid/ATTRIBUTION.md
- crates/bbt-supervisor/ATTRIBUTION.md
- crates/bbt-mux/ATTRIBUTION.md
- crates/bbt-proto/ATTRIBUTION.md
- crates/bbt-rt/ATTRIBUTION.md
- crates/bbt-render/ATTRIBUTION.md
- crates/bbt-ui-model/ATTRIBUTION.md
- crates/bbt-ui/ATTRIBUTION.md
- crates/bbt-doc/ATTRIBUTION.md
- crates/bboxybook/ATTRIBUTION.md
- crates/bboxy450-term/ATTRIBUTION.md
- crates/bboxy-firehose/ATTRIBUTION.md
- crates/bbt-mux/src/logic/ring_batch_tests.rs
- crates/bbt-mux/src/identity/status.rs
- crates/bbt-mux/src/identity/chain.rs
- crates/bbt-mux/src/identity/mod.rs
- crates/bbt-mux/src/identity/custodied.rs
- crates/bbt-mux/src/messaging/keys.rs
- crates/bbt-mux/src/registry/routing.rs
- crates/bboxy450-term/src/stall.rs
- crates/bboxy450-term/src/stall/headless.rs
- crates/bboxy450-term/src/stall/load.rs
- crates/bboxy450-term/src/live.rs
- crates/bbt-mux/src/knowledge/roots.rs
- crates/bbt-mux/src/knowledge/workspaces.rs
- Cargo.toml
- scripts/quarantine.py
- scripts/test-landing.py
- crates/bboxy450-harness-wake-up/src/dream_bootstrap.rs
- crates/bboxy450-harness-wake-up/tests/dream_bootstrap.rs
- scripts/formal.py
- scripts/test-formal.py
- crates/bboxy450-term/tests/native_signing.rs
- crates/bboxy450-term/src/ctl/plugin.rs
- crates/bboxy450-term/src/rich_pane.rs
- crates/bboxy450-term/src/rich_faces.rs
- crates/bboxy450-term/tests/formal_contracts/rich_host.rs
- crates/bboxy450-term/tests/rich_client.rs
- crates/bboxy450-term/tests/harness_attachments.rs
- reports/2026-11-06-dream-c1-consumer-landed.md
- reports/2026-11-06-dream-c1-persistence-correction.md
- crates/bboxy450-keychain/src/lib.rs
- crates/bboxy450-keychain-file/src/lib.rs
- crates/bboxy450-keychain-windows/src/lib.rs
- crates/bboxy450-term-desktop/src/lib.rs
- crates/bbt-plugin-fixture/src/lib.rs
- crates/bbt-plugin-fixture/src/main.rs
verify: make test-dream-c1 && make test-formal && make test-formal-contracts
id: 01M6NPVC7QS7PFD7ZBRCAPAT56
status: reported
delegated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-07T01:32:53.829Z
created: 2026-11-05T11:17:35.433Z
number: 93377961364376
created_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
updated_by: seat:01M6RZZY4FR22NC0Q6DAZGHYKC
authorship:
  signer: seat:01M6RZZY4FR22NC0Q6DAZGHYKC
  kind: seat
  public_key: vTy8kxzALPVGVJxPquLZelvl5D6TNzgsirkLE/DA5ek=
  process_generation: '1'
assignee: seat:01M6RZZY4FR22NC0Q6DAZGHYKC
artifacts:
- '[[01M6PHJANDHRZB8AJX3YZK4SBM]]'
- '[[01M6PYZ7H0PBAVPJY7WVAPYMVR]]'
- '[[01M6Q0SWSSNA4G1JANJB5YQQRH]]'
verify_result:
  command: FORMAL_REUSE=1 make test-formal && make check-formal-evidence
  exit_code: 0
  evidence:
  - criterion: Signed source landing
    reference: '06dacc38da2488e845bc25acebbb3855ed2c64b6; /tmp/dream-c1-protocol-wall-pin-make-land.log: LANDED; make land exit0'
  - criterion: Final exact-main formal and evidence gate
    reference: /Users/feral/mydev/bboxy450-term/.git/bboxy-formal-evidence/f71ba0486b94f751fe2ca7e12ef2896fa0ced67f7a49f6d4edd3e8a79a8a2442/evidence.json; /tmp/dream-c1-verified-main-formal.log exit0; /tmp/dream-c1-verified-main-check-evidence.log exit0; final harness f6a247b2be2ef21bb4d38ef805d404876d9e4adc
  - criterion: PERSIST/FULL, custody and unchanged production 1ms/9x law; bounded admission with broken control
    reference: '[[reports/2026-11-06-dream-c1-persist-entry-landed]]; [[reports/2026-11-06-dream-c1-source-landed-final]]; dated focused/native/contracts logs and final679-model receipt; known exceptions explicitly listed'
  - criterion: Exclusive reservation carrying complete parked view
    reference: '/tmp/dream-c1-parked-rich-pane-tests.log: make test-rich-pane exit0 (4+2+4passed); Oracle01M6SQRZX1QB6XF4J1T71GF7AT; final report gives dated source and final-pin disposition'
  - criterion: Ordinary run recorded honestly under source-only override
    reference: '/tmp/dream-c1-help-one-joined-run.log: exit2; gate-3tubkf94/receipt.json; Hello passed250ms, producer export exit100; Oracle01M6S8S6276F1C2HJ7H13YQYMR'
  - criterion: All exclusions and known implementation gaps disclosed
    reference: '[[reports/2026-11-06-dream-c1-source-landed-final]]:10load/12runner/1KNOWN DEFECT; tasks/2026-11-06-dream-c1-two-owner-storage-race; runtime-unapproved, cold-MCP and installed-admission limits'
report:
  summary: Source-only C1 landed as signed06dacc38da2488e845bc25acebbb3855ed2c64b6 through make land(exit0). Per Oracle's final disposition, exact landed main was checked out with matching harnessf6a247b2be2ef21bb4d38ef805d404876d9e4adc; final formal and check-formal-evidence both exited0, receiptf71ba0486b94f751fe2ca7e12ef2896fa0ced67f7a49f6d4edd3e8a79a8a2442 (679models/34crates/254contracts). Committed report includes the full mechanism/evidence history, 10load exclusions,12runner-only tests still selected by the canonical runtime gate, and the separately labelled HIGH two-owner StorageUnavailable defect. The ONE ordinary run remains FAILED after a passed250ms Hello (producer receipt-layout mismatch); runtime remains unapproved/not enabled. No install or bundle change. Final report checkpointc7aa823bda4aef7255715e2eb0ec17384107a5a6; clean detached worktree is ready for Oracle closure.
  commit: 06dacc38da2488e845bc25acebbb3855ed2c64b6
  reported_by: seat:01M6RZZY4FR22NC0Q6DAZGHYKC
  op_id: 01M6ST66EQYKD906MD1M8GEANQ
  verify_revision: 0
  report: '[[reports/2026-11-06-dream-c1-source-landed-final.md]]'
provenance:
  digest: sha256:631fa26bf201cae316cf7c5ea524de7bec93e15453a910af1d274d2f41585af2
  frontmatter_digest: sha256:92d40c33608bebf080a4612635499a5a41450dd71812a92052c0247df73a53d8
  signer: seat:01M6RZZY4FR22NC0Q6DAZGHYKC
  sig: 5YDZnDpim3w0F/432BiNBhS1H8ZlKw88ThZRTNEVYBn7mFacUFDU/fjDDVZqpCqSG60Wdh4kq8X69UOi0CBKCA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150086
  event:
    id: 01M6ST67AT7B7SND8TRDSC8HN3
    sequence: 150086
    hash: d4f285be1029976fce3f364318652c3d4377902ef72a327a4c2f03fd24765337
---
ROLE: EXECUTOR

## Outcome and authority
Finish and land the scoped term C1 integration in /Users/feral/mydev/bboxy450-term-dream-c1, branch feat/dream-term-c1. The operator explicitly requested merging completed work and cleaning up on2026-11-06. This is a landing handoff of the SAME task, not a new feature, a relaxed acceptance gate or permission to alter another component.
The COMPLETE prior brief is preserved verbatim in [[notes/2026-11-06-dream-consumer-scope-snapshot]] (task digestbf366097648c54741a9bc1e24ede0830585231337a40f833cc08fd9d8d7e5db0). All effective scope, controls and acceptance details in it remain binding. This concise handoff supersedes only obsolete baseline/milestone narrative and the earlier no-push clause as stated below. Read the current specs/ICD/correction decisions for exact contracts; do not invent a competing interpretation.

## Preserve the real work and recover ownership
Previous bboxy harness seat01M6NPQVR1STF4MY8C3TV382TJ recorded exit1 after the app restart. Root saved the signed5dffa17aceebeed957f21cb4de9903c332f2a206 head and the ELEVEN uncommitted tracked source changes at /Users/feral/mydev/bboxy450-term/target/dream-merge-handoff-gc2afvl6/bboxy450-term-dream-c1.patch,121930B/SHA6259d95e42741e5e2fde577554e287189081a75f2535305366dfd8ae4dbd983b. That is a backup, not an instruction to replay it over newer files. Begin with actual Git state and retained logs. Keep every existing edit; never stash, reset, broadly stage, erase or replace another seat's work. You own this writing tree after native reassignment.
Fetch current term origin/main (Root observed2847e213, may advance). Preserve the inherited changes in exact signed commits before rebase; resolve conflicts by retaining upstream and the scoped integration. Reports/notes use knowledge tools; native knowledge now has an explicit knowledge_commit checkpoint. Resolve any original unknown MCP operation outcome before repeating that effect. Root will checkpoint reviewed reports if a worker lacks commit authority. No shell KB writes or forced .bboxy additions.

## Implemented boundary and remaining delivery
C1 is own-seat inspection of authoritative closed nonfork IR after SessionClosed, with zero inference/trial/publication/Recall/provider discovery. Use the one TermC1 close owner; suppress automatic legacy D0 AND IR advice. No Dream computation during an active session. Immediate admitted-turn activity pauses work through the bounded current-session lane, independent of the legacy log follower. Closure remains IR-authoritative.
Retain the exact shared producer schema/fixture/hash authority, SourceKey/op/run/birth/incarnation identity, original effects and custody through duplicate/restart/ABA/cancel/expiry/unknown cost. Persist started fences before effects; do not replay predecessor work. Immutable descriptor/page/result bytes and exact hashes/indexes remain mandatory. All controls, source validation, partial-parser disposal, completion/actual exit/settlement and provenance/authority failures remain fail closed and labelled honestly.
Dedicated low-priority work stays off UI, logic owner and shared async/blocking pools. Preserve bounded independent status/cancel/message/input paths, immediate foreground pause, cooperative1ms application/parser/hash/serde/cleanup/control units, paid9x rest and the ONLY named native creation/publication2msCPU/25mswall classes with inclusive cost/effect/debt coverage. No blanket wrapper/native/control exemption. The bootstrap128B permit/revocation lease precedes controlled argv/channel/Builder/runtime/source, and transfers paid work ownership without parent/child overlap. Keep250ms readiness from the original origin,30s job deadline, actual escalation/reap/quarantine and all four-job/frame/input/control/stall/flood witnesses.
Working credits stay parent32MiB/child96MiB, each protected2MiB control, including the corrected helper headroom and observed ceilings. The corrected owned-credit contract is not an arbitrary all-times native allocator theorem. No limit increase or resource_guard removal. Source arrays, JSON, parser, diagnostics, hint mirrors, context/temp buffers and retained state remain bounded and charged.
Use the custody-free authenticated public fence; no signing_hold/device credential probe on Dream work/control paths. Preserve issuer/head/grant/inode/raw-row tamper, revoke/expiry/provider/root/routing/seat ABA negatives. Busy provider/identity work cannot stall independent control or drop pending invalidation. No fabricated authority, fixture fallback or stale cache promotion.
Shared host hints use the LANDED structuredContent.hints plus text mirror, strict final outer schema and post-journal idempotent off-owner projection. Use the actual shared helper; no worker hint-bearing proof DTO, duplicate helper or wire/control version change.

## Entry record and exact future worker pin
The old12ba/d06/e003fd term runner remains STOPPED. Its one diagnostic was consumed and failed; no retry, warm-up, timer reset, compiler/binary fallback or source-less readiness success. Neither1b577 nor414 approvals transfer.
Fresh producer [[tasks/2026-11-05-dream-c1-bootstrap-entry]], owner Dream Entry01M6QDKWCA62KCAKG3P2Z7ZVCD, is landing the corrected ordinary entry record on current harness main through its own canonical gate. Parent/parser work may proceed independently under [[interfaces/dream-term-c1#bootstrap-entry-record]]78219b91 and corrected65960-6cc010ae. Exactly one<=256B fixed-order decimal entry line precedes unchanged waiting. Missing/duplicate/misordered/oversize/malformed/reversed chain is unknown, never readiness success. Primary timestamps use explicit same-host clock_gettime CLOCK_MONOTONIC only. Child e may precede parent spawn return: preserve checked signed offsets and uncertainty, not a false reversal. The pre-write m bounds emission with observation; it is not emission time. Kernel counters remain raw/unverified, with no conversion/subtraction/CPU/runnable/loader/fork cause inference.
After Root reviews the actual landed producer, use make bump-harness REV=<approvedSHA> and canonical Make worker-only preparation in THIS tree. Give Root the exact source/compiler/features/build-closure/worker-hash receipt BEFORE any new term worker birth. Root supplies the explicit pre-birth tuple ruling; do not self-approve an arbitrary candidate. A materially new approved tuple may receive ONE ordinary joined250ms gate, no diagnostic wrapper. Original failed evidence stays failed. This is an asynchronous component handoff, not a reason to stop independent parser/source/formal/authority work or ask the operator.

## Narrow inherited compatibility work already authorized
Keep exclusive synchronous UI draft-slot reservation, full queued edit restoration and panel/session identity under65940; no UI mutex, lossy try_lock or worker reordering. In rich_client.rs recognize only the pinned bounded nonmutating capability queries while preserving exactly four requested mutations, every original payload/ID/noClose/cursor/durable/livesession assertion and global ID validity/uniqueness. In harness_attachments.rs initialize only optional SessionMeta dream/vcs None, preserving all attachment assertions. Other scoped lint findings are fixed normally, not waived. The formal family validator retains legacy-pair behaviour and every explicit control/verdict/member, strict malformed/missing/duplicate/wrong-verdict rejection and causal infrastructure tests.
For65951, only the three designated broken bootstrap safety-frontier models use the explicit abstraction; compare EVERY reachable safe state's labelled successors to the repaired unfrontiered reference, including first-violation edges, with an independent missing/changed-edge control. Retain original custody/cost/rest/exit/control/fair dispositions and fault latch. Full native/Java state+edge+named-verdict parity under the unchanged cap remains mandatory; no TypeOK/timeout/incomplete/backend-disagreement green. Do not claim complete post-first-fault compound-lifecycle refinement. The original runtime/process controls remain required.

## Verification, landing and closeout
The existing verify command remains make test-dream-c1 && make test-formal && make test-formal-contracts. Run the necessary affected canonical rich-pane/host lint/boundary/licenses/private smoke checks too. Reuse exact unchanged source-bound evidence only where the actual workflow supports it; run missing/stale required gates once on final source. Do not run an unrelated full suite. No operator Keychain/desktop/login/click/physical step: private homes, throwaway identities and scripted same-app native fixtures only. Kill only exact PIDs owned by the fixture.
The current-source actual ordinary worker/source readiness, joint parent+child credits/heap/RSS, four qualifying jobs, independent authority/control, foreground pause and same-app frame/input/message latency MUST pass. Unit/API/model subresults alone are not acceptance. Preserve actual commands/exits, compiler/source/model/tool/binary hashes, per-class/cutoff/debt/unknown-cost evidence and all failed runs. A real failed gate is fixed within scope or reported with the exact remaining decision; no threshold/liveness/control removal or lucky repeat.
After those criteria pass, you MAY land this signed source through make land on current main, without a further operator/Root ACK. Root is reading the actual diff/logs independently. Use the canonical lean gate and source-bound receipt reuse; no bypass CI, unsigned commit, direct forced push, extra install/deploy or release. New report path reports/2026-11-06-dream-c1-consumer-landed.md is authorized; the original report path remains for unchanged workflow. Write the full append-only report with task, signed commits, verify command/actual exit, exact landing head and per-criterion evidence/gaps; checkpoint it when granted, then task_report summary+locator. Do not claim the original report existed or was delivered if an operation remains unknown. Root accepts/marks done, closes the seat and removes the worktree only after all commits are on main, only generated outputs are dirty, and lsof shows no holder. No recruitment or unrelated feature work.


## Recorded bounded persistence/authority correction and owner-safe headroom
Root read /tmp/dream-c1-finish-evidence-index.json and actual mux log/final formal failure. The optimized86/113 +27failed run remains red;642 backend comparisons/check-formal-evidence0 do not replace failed canonical archival. [[decisions/d93377961366026-dream-c1-bounded-persistence-preparation]] is now the exact implementation disposition: use supported DELETE rollback journaling for the PRIVATE C1 catalogue with FULL durability/atomic transactions and safe busy0 existing-WAL transition; keep account/provenance WAL unchanged. Stage guard_with_session through existing prepare_session_cooperative/public_connection_cooperative checkpoints instead of its no-op checkpoint wrapper. Follow the decision's exact phase/cumulative-vs-max/custody/migration/crash/model/control rules, preserving every1ms/9x/final public-proof bound. Source scope is your already allowed registry/dream_c1.rs, identity/status.rs, logic/dream_c1.rs, related existing tests/models/maps; no generic identity rewrite/custom datastore/native SQLite class. First materially changed focused catalogue/authority checks; overrun/unknown remains a numeric failure, never a lucky same-source retry. Add the scoped new report if a detailed correction handoff is needed.
Root losslessly archived ONLY closed accepted producer/formal copies: archive /Users/feral/mydev/bboxy450-term/target/dream-completed-formal-archive-u8u6q_po/completed-formal.tar.zst446407760B/SHAf6bf57db8c642ac1567637b772136d8394cec017b7c54809f1791516a263d844; manifestSHA0b41985f4b5657c4b5e1036cc57a93e2f4838f2a190d06d62147fb03a254b912. All8688 files/10882 entries verified before removal, source/worker/gate logs retained. Shared df is STILL about4GiB, so no physical-space recovery is claimed from logical5.13GB compression alone.
You own the remaining safe cleanup in YOUR worktree: source is clean26e5, own target/debug is14.3GiB (mostly deps), fixture target3.7GiB, no incremental data. You MAY remove closed disposable completed-test/compiler outputs from your own target/debug/deps/build and fixture caches after recording exact paths/hashes/active-build status and lsof proving no process holds them. Keep actual ordinary/fixture pre-birth workers and profile receipts, immutable evidence/raw model graphs/Java tools/logs, source/backups and anything still needed by current proof. Do not touch other worktrees/global shared archives. Prefer cold completed test binaries/obsolete compiler outputs, not useful current libraries blindly; preserve hash/log provenance and retained formal tool binaries. Rebuild any removed artifact before relying on it again.
The ENOSPC destination is /Users/feral/mydev/bboxy450-term/.git/bboxy-formal-evidence/staging-u8jow54c/evidence/models/BrokenC1BootstrapParentOverlap.edges. That staging directory is YOUR failed unpublished archival attempt: after checking original target/formal data/receipts remain complete, exact-owner/source/mtime match, no published reference/active process and no unique data, you may remove ONLY that failed partial staging-u8jow54c directory, retaining its failure log/hash/index. Never prune another staging/cache/accepted receipt. Arrange measured free space for actual remaining archive bytes before the final source-bound archival step; reuse the canonical source/model/tool-current evidence mechanism, no manually minted formal-green receipt or redundant whole-suite rerun. Source changes still require current evidence and all final gates.
Entry2ce4359's fixed writer is independently reviewed; its canonical landing gate is now authorized but NOT yet landed. Wait for Root's actual landed SHA before canonical term bump/preparation and the explicit new source/compiler/hash tuple. Old12ba/16a remain stopped. Persistence and headroom corrections proceed independently now; no operator/Root ACK wait for this defined work.


## Entry producer is now reviewed, landed and accepted:2ce4359
Root verified remote harness main exact2ce4359df034405043d419fe550b0f6cafe7887e, G/Boxy signature, real canonical landing receipt/push0, full raw gate log and f7fc307ce5fcd01de6a4422f83fd3769a86250b1e573ff1da07fe37a1085ffc3 source-bound receipt. Required make check-embedded-build && make land0:116 library pass/1skip,37 C1 integration pass and275-model/76-contract full state/labeled-edge/verdict parity. Root reran ONLY current-source check-formal-evidence0 and independently decoded71 finite/admitted/settled births +71 bounded numeric bookkeeping receipts. Task accepted and seat_close applied. This is SOURCE admission; term1.99.1 runtime tuple remains unapproved.
You may now fetch/rebase current main as needed, then use make bump-harness REV=2ce4359df034405043d419fe550b0f6cafe7887e in your own tree, review/preserve any real Cargo.lock change, and SIGN the exact gitlink/lock checkpoint BEFORE preparation (the script checks HEAD's committed gitlink). Ordinary producer source bindings: bootstrap f6183cfa1f3eea408a853e171f44d456e5e8bf9c177fc088cc184634642aab64; main933e39cb206c63d5a3002d41bdaddb4603cee46edf47ede032ff974f71ba0ca9; c1d044a795700ab346569a2ab4e82b2345c87e805a50d4fc2e7f3d7b28bb6f202a; permit vectorsde919016ac6e5a464de60c86b2bf3bf6b54edc08e29c05f89688a5c80da37307. Report [[reports/2026-11-06-dream-c1-bootstrap-entry-fixed-writer-landed]] is committed/accepted; historical native/timing/diagnostic failures remain unaltered.
Run canonical make prepare-dream-c1-worker for a NEW exact source/compiler/features/build-closure/worker hash handoff. Root read this target and --worker-only path: while APPROVED_NATIVE_BUILD remains the OLD tuple, the new source mismatch builds/captures receipt with native_profile_supported=false/native_provision=null and performs NO Hello/birth. Do not edit that approval literal or use a direct-script/environment-altered substitute before this handoff. Retain before/after closure receipts, actual compiler-vV hash/flags/features, exact immutable binary bytes/hash and exits, then send Root the concrete tuple. Root will inspect and record its pre-birth ruling; ordinary3511aa48 from compiler1.99.0 is historical producer evidence, NOT a term fallback. The future ONE joined250ms ordinary run still starts at its original origin with strict entry evidence, no wrapper/warmup/retry. Old12ba/16a and the consumed diagnosis stay stopped. Independent bounded persistence/headroom corrections continue without waiting for build tuple approval.

## Oracle amend: land gate for empty test targets
`make land` failed on signed 33c97986 only because `bboxy450-keychain --lib` has zero tests and nextest exits nonzero (Oracle confirmed: 0 `#[test]` in crates/bboxy450-keychain/src). Granted `scripts/landing.py` and `scripts/test-landing.py`. In the lean plan, an UNFILTERED lib or bin target that is structurally empty gets a compile check (`cargo check -p <crate> --lib/--bins`) instead of a test run. A FILTER that selects no tests still FAILS. Add landing tests for both cases. Keep the diff minimal: the QoS executor also edits landing.py, so whoever lands second rebases.

**SUPERSEDED (operator: "add a dummy test then!"):** do NOT change landing.py or test-landing.py. Instead add ONE small real unit test to `crates/bboxy450-keychain/src/lib.rs` (a cheap pure assertion on an existing public type or constant, no Keychain access or OS prompt) so `--lib` has a test, then `make land`. The general empty-target gate fix stays on the improvement backlog.

**Oracle amend (empty targets, all at once):** the Oracle scanned every crate. Besides keychain, these have zero unit tests: bboxy450-keychain-file, bboxy450-keychain-windows, bboxy450-term-desktop, bbt-plugin-fixture (lib plus bin). Each is granted its src/lib.rs (and the fixture's main.rs). Add ONE small pure test to each: an assertion on an existing public type or constant, no Keychain, OS, Windows API or prompt. A Windows crate's test must compile and pass on macOS: no cfg(windows) gate on the test itself, and assert only platform-neutral items. STANDING RULE: if the land plan exposes any further structurally empty lib or bin target, add the same kind of pure test in that crate's src/lib.rs or src/main.rs without asking, and list it in the report. landing.py stays untouched.

## OPERATOR ORDER (latest, overrides the quarantine procedure): "speed up C1 land, cut corners, just get it done!"
1. Drop the quarantine and task-copy steps.
2. Add `#[ignore = "load-sensitive 250ms wall window (CPU 0.55ms, wall 207ms under load 11-15); re-enable via tasks/2026-11-06-qos-builds-never-starve-mcp (93377961366081)"]` to `actual_private_process_rejects_blank_invalid_utf8_unknown_duplicate_and_oversized_frames` ONLY. Assertions unchanged.
3. Commit signed and run `make land` NOW, reusing the running formal refresh. No extra suites or experiments.
4. After LANDED: task_report (existing report plus an addendum: SHA, the ignored test, the run marked FAILED with runtime unapproved, known limits), then DM the SHA. If make land fails on anything else, DM the one line; do not investigate.

## Oracle amend: formal verdict nondeterminism (latest)
TLC's periodic, wall-clock-triggered liveness check produced `liveness` instead of `safety` for BrokenHarnessDreamC1QuantumDebtAbaEffect under load (log: TLC hit the temporal check with 2056 states still queued). Fix in `scripts/formal.py` `tlc()`: add `-lncheck final`, so safety, found during the search, deterministically wins. Commit it with the #[ignore] change, rerun formal once, then make land.


## Report from seat:01M6RZZY4FR22NC0Q6DAZGHYKC

Source-only C1 landed as signed06dacc38da2488e845bc25acebbb3855ed2c64b6 through make land(exit0). Per Oracle's final disposition, exact landed main was checked out with matching harnessf6a247b2be2ef21bb4d38ef805d404876d9e4adc; final formal and check-formal-evidence both exited0, receiptf71ba0486b94f751fe2ca7e12ef2896fa0ced67f7a49f6d4edd3e8a79a8a2442 (679models/34crates/254contracts). Committed report includes the full mechanism/evidence history, 10load exclusions,12runner-only tests still selected by the canonical runtime gate, and the separately labelled HIGH two-owner StorageUnavailable defect. The ONE ordinary run remains FAILED after a passed250ms Hello (producer receipt-layout mismatch); runtime remains unapproved/not enabled. No install or bundle change. Final report checkpointc7aa823bda4aef7255715e2eb0ec17384107a5a6; clean detached worktree is ready for Oracle closure.

```json
{
  "op_id": "01M6ST66EQYKD906MD1M8GEANQ",
  "commit": "06dacc38da2488e845bc25acebbb3855ed2c64b6",
  "report": "[[reports/2026-11-06-dream-c1-source-landed-final.md]]",
  "verify_result": {
    "command": "FORMAL_REUSE=1 make test-formal && make check-formal-evidence",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Signed source landing",
        "reference": "06dacc38da2488e845bc25acebbb3855ed2c64b6; /tmp/dream-c1-protocol-wall-pin-make-land.log: LANDED; make land exit0"
      },
      {
        "criterion": "Final exact-main formal and evidence gate",
        "reference": "/Users/feral/mydev/bboxy450-term/.git/bboxy-formal-evidence/f71ba0486b94f751fe2ca7e12ef2896fa0ced67f7a49f6d4edd3e8a79a8a2442/evidence.json; /tmp/dream-c1-verified-main-formal.log exit0; /tmp/dream-c1-verified-main-check-evidence.log exit0; final harness f6a247b2be2ef21bb4d38ef805d404876d9e4adc"
      },
      {
        "criterion": "PERSIST/FULL, custody and unchanged production 1ms/9x law; bounded admission with broken control",
        "reference": "[[reports/2026-11-06-dream-c1-persist-entry-landed]]; [[reports/2026-11-06-dream-c1-source-landed-final]]; dated focused/native/contracts logs and final679-model receipt; known exceptions explicitly listed"
      },
      {
        "criterion": "Exclusive reservation carrying complete parked view",
        "reference": "/tmp/dream-c1-parked-rich-pane-tests.log: make test-rich-pane exit0 (4+2+4passed); Oracle01M6SQRZX1QB6XF4J1T71GF7AT; final report gives dated source and final-pin disposition"
      },
      {
        "criterion": "Ordinary run recorded honestly under source-only override",
        "reference": "/tmp/dream-c1-help-one-joined-run.log: exit2; gate-3tubkf94/receipt.json; Hello passed250ms, producer export exit100; Oracle01M6S8S6276F1C2HJ7H13YQYMR"
      },
      {
        "criterion": "All exclusions and known implementation gaps disclosed",
        "reference": "[[reports/2026-11-06-dream-c1-source-landed-final]]:10load/12runner/1KNOWN DEFECT; tasks/2026-11-06-dream-c1-two-owner-storage-race; runtime-unapproved, cold-MCP and installed-admission limits"
      }
    ]
  }
}
```

## Oracle closure note (2026-11-07 ~01:40): LANDED under operator override; acceptance pending BY DESIGN
- **Landed:** signed 06dacc38 on origin/main via `make land` (exit 0), harness pin f6a247b2 (contains 2ce4359). The Oracle verified the worktree HEAD is on main, the harness checkout matches, nothing tracked is dirty, and the final receipt f71ba048… is present. Formal 679/34/254 passed on that exact source (`FORMAL_REUSE=1 make test-formal && make check-formal-evidence`, exit 0).
- **Not accepted:** `task_accept` refused because verify_result (formal) ≠ task verify (`make test-dream-c1 && …`). That refusal is CORRECT. `make test-dream-c1` is red by design: the ONE ordinary run passed Hello (4.3–6.9 ms) and then FAILED at the producer receipt layout, and runtime stays unapproved. Under the operator's "get C1 landed, cut corners" order, the source landed without that gate.
- **Acceptance path:** [[tasks/2026-11-06-dream-c1-receipt-layout-bridge]] makes `make test-dream-c1` green; then re-run this task's full verify and accept. Other follow-ups: [[tasks/2026-11-06-dream-c1-two-owner-storage-race]] (HIGH), [[tasks/2026-11-06-dream-c1-honest-capture-errors]], [[tasks/2026-11-06-dream-c1-cold-mcp-latency]], and re-enabling the 10 load-ignored tests under [[tasks/2026-11-06-qos-builds-never-starve-mcp]].
- Seat 01M6RZZY… closed. The worktree is removed once its processes exit.
