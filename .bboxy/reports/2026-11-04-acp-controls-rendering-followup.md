---
type: report
title: ACP controls rendering amendment delivery
task: '[[01M6KF628K0H7DHN4R8BCN9WVZ]]'
supersedes: '[[01M6KGWYYSF1W37CJ3T7ETB898]]'
commits:
- a832bd6ff4b31a36064508632c47840a58cfe56c
- 9501b79845cd7486738e6bf948a09813e70d87cf
verify:
  command: make verify-acp-controls
  exit_code: 0
author: seat:01M6KF1JD34QA47CQP34FRJN5Z
created: 2026-11-04T15:25:09.667Z
id: 01M6KJKX4CVY6TPFHV939A12T7
updated: 2026-11-04T15:25:09.667Z
number: 93377961360863
created_by: seat:01M6KF1JD34QA47CQP34FRJN5Z
updated_by: seat:01M6KF1JD34QA47CQP34FRJN5Z
authorship:
  signer: seat:01M6KF1JD34QA47CQP34FRJN5Z
  kind: seat
  public_key: El6VqSdMLzX3+r6uiKxbH22bBB8VwVCtourrWmlf8Yk=
  process_generation: '1'
provenance:
  digest: sha256:b812329d76d0d08940044480f1cc302c657df41b4cb2debc3b688a9102edbd1b
  frontmatter_digest: sha256:1a4352d1eb42a305498dd776c7c675e1cc2bb9e5bf0ff032d5720b9fe9e956bf
  signer: seat:01M6KF1JD34QA47CQP34FRJN5Z
  sig: FKy3Cehzdb1LHsnBjYrVb/PjZtmyd/YywACIWI48+FgmZk2dsatgIv8cHiYI/VG9BmOvSsJH81SSXrImn2j/Ag==
  log: .bboxy/log/2026-11-04.jsonl#119901
---
Completed the signed rendering amendment to task [[01M6KF628K0H7DHN4R8BCN9WVZ]] in the assigned worktree and branch. This append-only follow-up supersedes the handoff in [[01M6KGWYYSF1W37CJ3T7ETB898]] after the Oracle expanded the brief. The original report and signed source history are retained.

Implementation commit: `a832bd6ff4b31a36064508632c47840a58cfe56c`, parent `c26d785951108835f40dc62ae1b0936f32cafea7`; signature verification returned G for Boxy (bboxy450). No pull, fetch, merge, rebase, cherry-pick, pin change, installation, live-session signal or publication was performed for this amendment. The native term worker's status-title and readonly handoff work was left untouched; `window.rs` changed only pointer-wheel routing.

Mechanisms and acceptance evidence:

- `src/frontends/gui/face.rs` clamps tool-header targets to their first line with the existing ellipsis behavior and clips card children to the card. `src/frontends/gui/face_rows.rs:1320` builds literal command, caption and output panes from the existing ScrollView, frame and text primitives. The complete text is retained; it is no longer reduced to a 14-line terminal preview. Multiline/heredoc commands have compact 76 px and expanded 320 px viewports, execute output is bounded at 320 px, and generic literal output at 400 px. Existing header/body folding and the named terminal output/padding contract remain in place.
- `src/frontends/gui/view.rs:395` handles two-axis view-local scrolling; `:445` clamps each pane's offset to its own content. Separate pane keys prevent command, caption, output and Markdown offsets from colliding. Existing session/epoch scope reset clears these offsets. Horizontal arrow actions work through the same HarnessView in embedded hosts; older vertical-only host wheel calls can also move a single long line horizontally. The standalone window forwards horizontal deltas and Shift-wheel through the shared method. These operations do not send gateway ops or prompts.
- `tests/gui_rich.rs:1821` uses native font/layout geometry for long unbroken commands and 90-line heredocs at 280 px and 1120 px. It checks bounded card/header/pane geometry, compact versus expanded height, command and output tails, horizontal arrow actions and scrolling without moving the transcript. `:1939` verifies that literal text zoom at 90%, 100% and 160% preserves the full horizontal extent while the viewport stays fixed.
- `src/frontends/gui/face.rs:2337` shares the existing welcome-screen breathing halo with the loader. `:2346` keeps the existing portrait asset and places “Loading…” beneath it. `tests/gui_render.rs:1981` checks the exact portrait asset, label, horizontal/vertical centering and preserved pending-history behavior; `:2316` steps the native animation clock and proves motion changes while reduced motion stays static. No second clock or new bitmap was introduced.
- Agent-advertised model/effort controls, ACP requests, refusal/unknown handling and ownership fences are unchanged from the initial delivery. Their actual private process/Unix/native tests were rerun in the amended gate. The queued-history raster, MCP output, live terminal, header fold/padding and Markdown-scroll regressions also pass.

Verification: `make verify-acp-controls` exited 0. All 19 recorded commands exited 0. The gate includes default/embedded/ACP+standalone compilation, embedded/all-feature lint, 11 ACP-control process/native tests, 20 existing ACP tests, seven launch-context tests, ten bridge tests, three pending-core tests, the retained history raster, two loader checks plus the MCP card check, two new tool-card/zoom checks, two terminal checks, the tight-header check, the Markdown-scroll check, the standalone path check, five schema tests and the two selected formal export tests.

The six selected native/Java TLC graph/verdict comparisons still match in full: ACP controls 153 states/402 transitions; its stale control 189/510 and replay control 252/645; pending history 995/2532; pending-ack control 1245/3143; pending-stale control 1283/3812. Good verdicts return TLC 0 and the four deliberate safety controls return TLC 12. No async/wire contract changed in this rendering amendment; geometry, scrolling and animation are covered by the focused native checks. This is bounded model evidence, not arbitrary Rust refinement.

Current source, command/exit/log hashes, tool pins and capture hashes are committed in `.bboxy/runs/codex-acp-controls.json`, SHA256 `25a795b1a2e5d88e9008a4c935ea23039c8911f46f92fc3ecf7f5c13ec39ab5c`. Its 88 source-input hashes matched the working files at verification, and are checked against committed bytes before handoff. The earlier receipt remains preserved in implementation commit 9501b79. `git diff --check`, `git diff --cached --check` and `python3 -I scripts/checks.py headers` exited 0.

A focused terminal regression initially found a lost named output wrapper; it was restored, and the original padding/live-output checks pass unchanged. The failed amended-gate log is retained at `target/acp-controls/rendering-attempt-1-gate.log` and its terminal log beside it. Test-fixture duplicate IDs and an initial private-field assertion were corrected before the final gate. Earlier failures are not counted as passing evidence.

Omitted as scoped: whole test/formal/parity/runtime corpora, paid/live vendor probes, desktop input, live session activation, installation, publishing and cross-repository integration. Oracle owns independent review, pinning and activation. Source commit worktree state was clean; this indexed report is the only subsequent intended change.
