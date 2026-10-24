---
id: "01M5NY2BD67BNB8993E7Q625HH"
number: 252
type: note
title: "L9-M-c native messaging, tasks and unread: verification handoff"
status: reported
created: "2026-10-24T03:08:01.629861Z"
---
# L9-M-c

Source **69d2fc542eab35bb83f490d75ebb3f6981c49a58**, accepted parent **9c2022a7288286981f0cb3a5b1938691bf8a3bf5**. Branch nyx-l9-m-c, worktree /Users/feral/mydev/bboxy450-term-nyx. All 20 source commits have good machine-key signatures. These records are evidence-only successors; the production source is frozen at that SHA.

The native composer, shared conversation and task review now use the accepted GUI Messaging/Task wrappers and the single durable event lane. Reusable widgets live in bbt-ui (decision #39); the app binds actual destinations and task IDs, bounded dispatch and the published model. Wren's paired lab uses the canonical MessageRow, TaskCard and TaskReport projections (decisions #37/#38).

## Mechanisms

- crates/bbt-ui/src/text_edit.rs:1 and widgets/text_area.rs:6 — shared UTF-16 selection, caret, paragraphs and IME preedit. Preedit neither filters nor sends; candidate positioning uses the real caret rectangle. The launcher uses the same editor/controller core.
- crates/bbt-ui/src/modules/composer.rs:133 and composer/report.rs:198 — recipient/kind/priority controls, attachments and report review. A stored reply clears only the unchanged submitted draft. Reject requires a committed reason; Accept requires reported status and verify_result at exit 0. A commit reference never implies a signature.
- crates/bboxy450-term/src/messaging.rs:440 — actual scoped registry IDs, role fan-out capped at 32, stable operation IDs and explicit retry after unknown outcomes. Correlation is bounded to 64 replies / 1 MiB; task pages to 128 / 256 KiB per scope, documents to 16 / 512 KiB, and the tree to 256 / 256 KiB.
- crates/bboxy450-term/src/messaging/history.rs:69, :215 and :370 — MessageRow projections and receipts share server data; ledger and bus cursors stay separate. Only an explicit view requests a durable read-mark, and only the confirmed server cursor proves unread. Receiving, publishing and transport ACK never mark read. Incomplete counts remain More.
- crates/bboxy450-term/src/model.rs:74 and live.rs:260 — monotonic page completions/refusals, including empty/repeated pages; the requested read page survives a full live window within the shared 512-row / 1 MiB bound. Live data wins if both cannot fit, with a named gap. No bus consumer, ACK policy, GUI version or protocol lane change.
- crates/bbt-proto/src/messaging.rs:453 and crates/bbt-mux/src/mcp/work_items.rs:313 — the approved default-false unassigned inventory flag. Only the account Operator can read open unassigned tasks; seats remain refused and Delegate still scopes by its actual assignee.
- crates/bboxy450-term/src/main.rs:867 and scene.rs — native conversation hosting, focus/input/IME/hits, sidebar task/unread projection, and the --compose verification flag. Kitty, terminal key ownership, Verify, scrollback and spawn placement remain covered by regression tests. Launcher/HUD paint order is retained.
- crates/bbt-ui/src/components.rs and modules/stream_dock.rs:1032 — approved optional repeating Rects paint (None preserves existing output), implementing the designed Creativity rail at 4 px on / 3 px off. Report cards keep the accepted G5 kit rim outside their clipped contents and explicitly compare the wrapper rectangle. No renderer/shader or parity-policy change.

## Results

| Check | Actual result | Evidence |
| --- | --- | --- |
| Strict workspace/all-target Clippy | exit 0 | Full CI and development log |
| Workspace tests/rustdoc | 1029 passed, 0 failed, 21 intentionally ignored | Full CI; invariant commands add 3 passing tests |
| Clean exact-source sh scripts/ci-local.sh --parity | exit 0, 2011.90 s; 17 invariants pass, 5 named slow skips | [CI log](l9-m-c-ci-69d2fc5.log), [metadata](l9-m-c-ci-69d2fc5.json) |
| Current visual matrix | 2024 captured/rendered; 1920 required measured; 0 unproven, 0 mismatches | [report](l9-m-c-parity-report.json), [summary](l9-m-c-parity-summary.json), [requested manifest](l9-m-c-parity-requested.json) |
| Pixel advisories | 172 to 148; 0 new keys; 24 cleared | Summary; G5 retained the G3 key set in g5-review-keys.log |
| Historical diagnostic | 1664 captures/renders; 867 pixel reviews, non-gating under existing policy | CI log; hashes in [file manifest](l9-m-c-parity-files.json) |
| Native app capture | exit 0; idle 10.003 s, 0 frames/callbacks | [capture metadata](l9-m-c-native-capture.json), [native log](l9-m-c-native-compose.log) |
| Fresh default A/B on identical 1768 frozen inputs | both replays exit 0; blanket byte test exits 1: 1728 exact, 40 rail-only deltas | [per-file hashes](l9-m-c-default-bytes.json), [rail proof](l9-m-c-rail-deltas.json), [stage logs](l9-m-c-extra-proof.log) |
| Scoped rail assertion | exit 0; every changed RGBA pixel inside captured Creativity rails, zero outside | [rerunnable check](l9-m-c-verify-rail-deltas.py) |

Full CI SHA-256: ec19c48a743b680dc384fca6c497646262eac6e698bd3b8a6cbcb1146e67710b. Current parity report SHA-256: 94d9bd6d44d626365658fe7a2e72df100ef69b43c571b94c235c2bc8ca15e808.

The native image is target/l9-m-c/native-compose-69d2fc5.png, SHA-256 fef77815bab8cf87726998a30b06837e8431a73d7f9f09c2787a3d84a2468d52. I read it: focused composer/caret at the sidebar foot, kind/priority/send controls over the deterministic 15-pane smoke fixture. Empty history is expected in this fixture; no manual desktop or live-message screenshot claim.

The 40 A/B differences are exactly five message/stream states across all densities/themes, each containing Creativity messages. The source fixes their formerly solid 3 px rail to the existing design. Every other pixel is exact. The raw blanket check's exit 1 is retained; no default identity claim or parity exclusion hides it. Approval: Oracle 93dbb857; repeating primitive commit 1fae4199. The new/current lab matrix passes with no new advisories.

## Acceptance evidence

- tests/messaging_flow.rs:219 composes and sends a decision through the real GUI boundary, reads its retained row, checks worker-progress refusal, delegates an actual task path to an actual seat, observes frontmatter/receipt, and exercises real seat MCP report/verify refusal and exit-0 acceptance.
- live.rs:1860's production IO-owner fixture publishes the MessageRow/task/read-mark projections through the accepted lane. tests/messaging_ui.rs:410 proves receive does not mark read; later cases reject stale/empty page coverage and old receipt projections. Model tests preserve a requested old page beside a full live window and complete empty/refused reads distinctly.
- Composer, UTF-16 editor, TextArea and main routing tests cover selection/preedit, no preedit send, candidate caret positioning, committed reject reasons, scoped recipients, stable retries, send cap and unchanged-draft clearing. Shared task marks use the accepted mark kit.

## Earlier failures and limits

[Development evidence](l9-m-c-development.log), the [earlier clean gate logs](l9-m-c-ci-attempts.log) and [manifest](l9-m-c-evidence.json) retain real exits. Gate 9cfad21 exited 101 on a stale SeatRow count; 92c5091 exited 1 on a baseline license-table parser and 24 legacy constructor inputs; 1c5fcb7 exited 1 on eight timestamp and eight task-status fixture mismatches. Those were corrected at their owning layer, without policy changes. The G5 preflight found 48 unproven report-wrapper paths; explicit wrapper/descendant capture plus native kit rim fixed all 48, with zero pixel reviews.

Optional whole-workspace formatting exits 1 on 20 unchanged baseline files; no changed-file formatter diffs remain. That baseline debt is deliberately untouched. No --slow scale result is claimed. Real OS keyboard/IME interaction was not driven; state/routing tests and native capture are the proof. Wren owns the original lab-page attachment-picker query clipping follow-up.

## Merge and cleanup

Main advanced during the gate through Wren's 49a1f8c notification title/body change, plus plugin records and a Mermaid fixture. No source rebase followed. At merge preserve her notification(), NOTIFY_BODY export, CSS removal and showcase call, together with this branch's canonical reportCard and prioChip/atHM/hhmm/editRuns exports (Wren union review 4c6d790e). The Oracle owns that merge.

The private CI target was removed immediately after the approved sequential capture/A-B proof: 3,292,524,544 allocated bytes. Two superseded owned baselines freed 184,655,872 bytes earlier. Final record validation and remaining owned-cache/worktree cleanup are recorded in [record checks](l9-m-c-record-checks.json); those successors contain no production source edits. In-flight galleries and peer baselines remain available for review. All retained outputs are indexed by hashes; no peer directory or process was touched.
