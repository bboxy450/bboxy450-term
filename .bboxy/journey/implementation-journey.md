---
authorship:
  kind: seat
  process_generation: '1'
  public_key: fewHFqX1/aT03A8Ik46eNy0fXOwtv5Rt1gZmJu1Aids=
  signer: seat:01M649MS05FAD5CY9CS0Q07H76
created: 2026-10-29T18:56:38.749Z
created_by: seat:01M649MS05FAD5CY9CS0Q07H76
id: 01M64GAXAJF20VSN6GN6XB6SB3
links:
- rel: context
  to: '[[vision]]'
number: 431
provenance:
  digest: sha256:fdcbe8effcfe8bc96f78ecaf5b5f1bdc286c0b93672a96e224906d9464a6c9a8
  frontmatter_digest: sha256:7f8c13d5bad000a24ec81357d7d737f02f25085d8853861b4eebde42af240234
  log: .bboxy/log/2026-10-29.jsonl#2779
  sig: is6Y+XI26Cs6bz0VDWz+8dQIKlkREVj8pLjyrSmYQ5Z+iCRgTusXNM0AYuDUBmqie1cQjLysaqrJt5GokN1KDg==
  signer: seat:01M649MS05FAD5CY9CS0Q07H76
title: Implementation journey — from terminal primitives to a usable swarm
type: note
updated: 2026-10-29T22:49:35.048Z
updated_by: seat:01M649MS05FAD5CY9CS0Q07H76
---
<!-- bboxy450-term: https://github.com/bboxy450/bboxy450-term
Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md -->
# The implementation journey

This is the local entry point for resuming bboxy450-term. Read it in order to see why each layer exists, what actually landed, and what the next cut must prove. Stable specs, decisions, task IDs and evidence stay in their original locations.

Snapshot: 2026-10-29, main `014e555ca341cf5f4037360a74bf75a7c96c23a4`, matching GitHub main at inspection. The initial GitHub inventory had **53 open issues**. The review inspected the 296 first-parent entries returned by `git log --since=2026-10-26`, source and retained evidence. Unmerged worktrees are inventory, not shipped changes. The final issue states and commands belong to [the audit summary](../runs/issue-triage-20261029.json).

## 1. Establish the pieces — 2026-10-21

The [vision](../vision.md) made seats, swarms, messaging and a reusable GUI kit the product vocabulary. The [original plan](../plan.md) separated terminal core, supervisor, rendering, UI and identity. The [historical task board](../tasks/README.md) preserves the first assignments and accepted commit pointers; its old RUNNING/QUEUED labels are not today's assignments.

Read [terminal-core](../specs/terminal-core.md), [mux-daemon](../specs/mux-daemon.md) and [GUI components](../specs/gui-components.md) before extending those pieces. Current continuation: [terminal and recovery](terminal-and-recovery.md).

## 2. Make the pieces survive together — 2026-10-22 to 23

The cockpit moved from examples to live seats, separate backend owners, generation-bound identity and local messaging. [C1 live seats](../specs/c1-live-seats.md) and the [architecture description](../specs/architecture.md) explain the process split. Architecture prose is dated; check the current code and [interface matrix](../interfaces/interfaces.md) when deciding a boundary.

The suite is the product and the terminal is its first app: [decision 39](../decisions/d39-suite-is-the-product.md). The visual foundation and web/native parity remain priorities in [vision](../vision.md). Current continuation: [UI and documents](ui-and-documents.md).

## 3. Make work and evidence addressable — 2026-10-23 to 25

[Decision nodes](../decisions/d47-decision-nodes.md) and [global record numbers](../decisions/d48-every-record-has-a-number.md) gave knowledge stable identities. [Work items](../specs/work-items.md) connects briefs, allowed paths, verification and lifecycle; [records-sweep.log](../runs/records-sweep.log) records the migration without claiming all later product acceptance.

The [whole-promise rule](../decisions/d316-pass-means-the-whole-promise.md), [acceptance matrix](../specs/e2e-acceptance.md) and [script map](../specs/e2e-script-map.md) distinguish full coverage from an exit-zero subset. Current continuation: [records and trust](records-and-trust.md).

## 4. Ship a bounded first release — 2026-10-26

The signed `v0.1.0` source is `d02f6135`; [the retained release handoff](../../resume.md) records publication and its limits. GitHub's already-closed release/repair issues are history, not new work to rebuild: #41, #42, #44, #45, #53–55, #62 and #65–68. Their state was read from GitHub; this triage did not freshly revalidate every historical closure.

The first full night also exposed real scale failures and honest coverage gaps. [Frame partition evidence](../runs/night-2026-10-26-frame-partition.log) preserves those failures. Current continuation: [scale, extensions and release](scale-extensions-release.md). A development build labelled 0.1.1 is not evidence of a published 0.1.1 release.

## 5. Turn the app into the everyday cockpit — 2026-10-27 to 29

Recent main adds native navigation, desktop/session survival, scoped workspace reads, terminal selection/font/emoji, image custody, independent Chat/Tasks sections, role-first identity, retained layout reuse and bounded Codex proxy traffic. Start at [navigation evidence](../runs/native-navigation-20261027.log), [role identity](../runs/role-first-20261029.log), [input evidence](../runs/input-hover-drags-20261029.log), and the implementation commits `78a3ee9f`, `a4658316`, `3bfc5c60`, `7198e4cc`.

Several old acceptance issues remain open because these useful cuts do not prove their whole promise. In particular, real-harness delivery, a normal produced streaming frame, retained event backlog, and the full-count timing verdict each require their own evidence.

## 6. Prove native documents, then finish their interactions — 2026-10-29

GitHub already closed #34/#35 for D1/D2 and #18/#29/#30/#52 for Settings/notification-boundary evidence, plus #51/#60 for signed mentions/typed wake causes. [Native document verification](../runs/night-documents-verification.json) preserves actual paint assertions, negative controls and parity limits. Newer table fit, Find feedback and backdrop dismissal are recorded in [document-dismiss-tables](../runs/document-dismiss-tables-20261029.log).

D3/D4, Cmd-click links, cards and real OS notification activation are separate open promises. Do not recreate closed D1/D2 work or relabel a partial test full. Continue in [UI and documents](ui-and-documents.md).

## 7. Resume from the remaining boundary failures — initial triage

1. Restore inspectable Knowledge operations and authorized diagnostics: [#74](https://github.com/bboxy450/bboxy450-term/issues/74), [#75](https://github.com/bboxy450/bboxy450-term/issues/75). The failed write's outcome remains unknown; no repeated effect or backing-store workaround is authorized.
2. Fix the concrete retained-chat P1 [#73](https://github.com/bboxy450/bboxy450-term/issues/73), while proving the normal-frame Codex P1 [#72](https://github.com/bboxy450/bboxy450-term/issues/72) separately. These have different causes and acceptance.
3. Apply the audited historical reconciliation in [#43](https://github.com/bboxy450/bboxy450-term/issues/43); do not confuse a proven private import with completed live rollout.
4. Finish the missing full acceptance lanes by domain below. Keep the scale workload, budgets, quarantine expiry and unattended rule intact.
5. Prepare naming/publication work under its later release brief. Implementation authors use the already-authorized321 landing workflow after real gates; release publication/yanks still require explicit release scope.

The first cold normal gate of this documentation cut passed build/strict lint, then failed the recovery fixture's unchanged 10 GiB free-disk guard. The failed log is preserved, and [#77](https://github.com/bboxy450/bboxy450-term/issues/77) owns cold-gate capacity/headroom. Free space recovered after the run ended; a warm validation run does not close that infrastructure task.

| Continue the journey | What remains |
|---|---|
| [Records and trust](records-and-trust.md) | Record numbering/graphs/RFC, identity proofs, historical reconciliation, operation recovery |
| [Terminal and recovery](terminal-and-recovery.md) | History, keyboard, flaky boundaries, hook evidence, normal-frame rendering |
| [Conversation and events](conversation-and-events.md) | Real-harness receipt, priority, event/read-mark recovery, retained chat backlog |
| [UI and documents](ui-and-documents.md) | Cards, OS activation, providers/themes, links and document actions |
| [Scale, extensions and release](scale-extensions-release.md) | Full-count performance, plugin ports, Windows custody, future transports and packaging |
| [Diagnostic triage](diagnostic-triage.md) | Observed error classes, task links, access limits and reproduction contracts |

GitHub issue numbers in these tables link to GitHub. Local `[[#N]]` identities still mean KB record numbers. The domain tables are a dated triage snapshot; GitHub holds current issue state and authenticated task tools hold current assignments. A KB note does not claim runtime task acceptance.



Admission note, 2026-10-29: this dated triage was first prepared in signed private commits de793c11b7dd3bfd17ac453d8c47c98dc91adbae and 0efe0d917de225fa702667dc646b7981ae7edecc. The authenticated host now assigns this local copy a fresh identity and global number; the private draft numbers are not imported as current authority. Follow [[2026-10-29-zero-issues-cohort]] for subsequent assignments and verified dispositions. Unresolved promises remain unresolved.


## 8. Resume the implementation cohort — 2026-10-29 22:40 UTC

Latest independently inspected remote main is signed d70e94d0c299edb97d2bcfbb0161c155d236ea4a. Exact-source full Make0 and canonical author landing0 are verified. GitHub49 is now closed for removing unused live-swarm fallback construction with actual equivalence evidence; broader scale budgets and measured speedup remain unproven. Earlier skills compatibility at6641b5b7 closed63. Terminal color defaults atd93e9675 preserve explicit child overrides. Main also includes4323e3b2 provenance disclosure/controller/lab with full0/parity, without proving live Graph.

GitHub has53 open issues at the latest server read.73 is now verified closed on signed d70; see the explicit acceptance correction in [[2026-10-29-zero-issues-cohort]].64 stays open: verified fixes are required and the operator cannot remember the frozen-dialog app; do not ask again or close it as a migration. Remaining issues stay linked to concrete tasks and evidence.

Lumen's378/386/445/447/449 fixes landed atd70, including canonical Recent read-through, actual None→Event single-row reconciliation and a corrected348000 same-live-owner/frame/input/read proof. Earlier fatal-lock false-warm and cold-start failures remain recorded.All four written73 acceptance lines are satisfied by the verified full-count private live-owner/frame/receipt proof and exact-source full/landing0. The earlier extra Oracle deployment gate is explicitly superseded for73; production activation/live43 import and deployment of74/75 remain separate. New489 [[2026-10-29-issue-72-codex-rendering]] assigns the separate normal-produced-frame Codex streaming/resize defect; idle captures that force redraw cannot satisfy it.

Rook384/423/451/457 has private observer-safe250-record import, capacity guards and retained-owner activation evidence, with earlier full low-capacity refusal preserved.465 [[2026-10-29-original-operation-recovery]] now has actual focused bridge/store/authority/Index-failure evidence; final current full gate and landing remain.487 [[2026-10-29-read-query-contract-adoption]] adopts483's current operation_get own-query fresh-observation rule consistently in discovery/orient/bootstrap/bridge.75's diagnostics reader remains unimplemented; original unknown operations are not replayed. Completed live reconciliation is still required for43.

Architect381/394/455/467/470/475 are accepted design/inventory work, not native product verdicts.467 defines actual Graph/RFC/digest-history interfaces;475 preserves current350's skills addendum;470 classifies package names without publishing or yanking.481 final signed023ce1c3 is accepted for the narrow current operation_get query-ID exception, with no target effect or Q-result cache.491 [[2026-10-29-seat-diagnostics-producer-contract]] now defines75's concrete host-bound producer/retention/allocation seams for subsequent separate implementation. All1–4 native acceptance scripts remain unproven. Adopt law amendments onto then-current main rather than replacing older whole files.

[[2026-10-29-zero-issues-cohort]] preserves exact sources, command/log hashes, failures, assignments and rollout order. Shared checkout contains native KB writes; do not reset it or import colliding private draft identities. Zero verified open issues remains the objective and has not been achieved.
