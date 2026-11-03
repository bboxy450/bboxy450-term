---
type: task
title: Fix startup-modal keyboard routing and attached-state UI fixture regressions
role: executor
verify: make test-startup-input-regressions
allowed_paths:
- crates/bboxy450-term/src/main.rs
- crates/bboxy450-term/src/keyboard.rs
- crates/bboxy450-term/src/app_overlays.rs
- crates/bboxy450-term/src/pointer_cursor.rs
- crates/bboxy450-term/src/terminal_clipboard.rs
- crates/bboxy450-term/src/startup_modal.rs
- crates/bboxy450-term/src/cockpit/startup.rs
- verification/contracts.toml
- .bboxy/tasks/2026-11-03-startup-modal-input-regressions.md
- .bboxy/reports/2026-11-03-startup-modal-input-regressions.md
- Makefile
id: 01M6HGCSS1W74GRFPQW3V7RAVY
status: claimed
delegated_by: seat:01M6HF0S1BAJKHWT2N426W7328
swarm: 01M6CQARWH5A79TE3H4NNWJ4XG
updated: 2026-11-03T20:21:30.097Z
created: 2026-11-03T20:07:48.593Z
number: 16417
created_by: seat:01M6HF0S1BAJKHWT2N426W7328
updated_by: seat:01M6HF0S1BAJKHWT2N426W7328
authorship:
  signer: seat:01M6HF0S1BAJKHWT2N426W7328
  kind: seat
  public_key: NJfxjXALG4vfTtn9XxMx7XCD2GZh2WlrBe8FFTqIaLM=
  process_generation: '1'
assignee: seat:01M6HGCVKFA0E7CEGE8E7Q5D5P
verify_revision: 1
provenance:
  digest: sha256:16f4fd5976be99b0e57f411b6d3a0204b969e9059e3bf084b3d4bfbe739cfbfb
  frontmatter_digest: sha256:a076641d7e88b0663ad0fb63d381e96acc605ff574b1dfa967e63583f312e4ad
  signer: seat:01M6HF0S1BAJKHWT2N426W7328
  sig: c/vX0jFWkT3hr5i8jUqpY9J910CQJrqomwDwatskyIixr3unnkoGlFB+p3iNf+Bxa3oqQIHPf9skZ9i6uQcYCQ==
  log: .bboxy/log/2026-11-03.part-86213.jsonl#98129
---
# Restore the startup modal's input gates without weakening existing assertions

ROLE: EXECUTOR. Worktree `/Users/feral/mydev/bboxy450-term-startup-input-fix`, branch `startup-input-fix`, from adf9796b. Work only there. Read AGENTS.md, ~/.agents/AGENTS.md, ATTRIBUTION.md and [[tasks/2026-11-03-oracle-handoff-codex]].

The active main-green run on adf9796b (its target/ci-local.log) found these failures:
- keyboard_adapter_uses_neutral_core_and_live_modes, keyboard.rs:728: main.rs now has two WindowEvent::KeyboardInput arms (8928 and 9164), violating the single neutral-adapter route assertion (2 vs 1).
- app_overlays::tests::arbitrary_dialog_traps_keys_and_resolves_only_its_control_cursors at app_overlays.rs:1275: cursor Default instead of Text.
- pointer_cursor::tests::pointer_cursor_follows_real_controls_overlay_order_resize_and_layout_changes at pointer_cursor.rs:322: Default instead of Pointer.
- terminal_clipboard::tests::selection_pointer_owns_mouse_mode_and_command_link_uses_displayed_frame at terminal_clipboard.rs:333: Default instead of Pointer.

Startup-modal 1affb986 introduced the extra keyboard route and initially-visible startup state. The three fixtures build App::owner and manually attach scene/live model while omitting the real startup attachment transition. Verify this hypothesis. Consolidate keyboard admission into the existing single neutral adapter/event path while retaining startup Retry/Quit shortcuts and trapping underlying input. Do not evade the assertion with a string trick or relax it to count=2. For attached-state UI fixtures, drive the production attachment transition before asserting interactive underlying cursors; retain real cold-start/failure modal blocking in a focused regression. Do not globally disable the startup modal or set a production default to invisible just to pass tests. Preserve headless/private-home test behavior and every existing pointer/overlay/selection assertion. No UI style, geometry, desktop interaction or lifecycle redesign is authorized.

Run the four focused regressions and the startup model test first. The existing main-green run is still running: do not write its worktree or repeat its full suite. A separate real bboxy Executor repairs the unrelated malformed startup-modal report under #16394; do not duplicate that patch. Add a canonical `make test-startup-input-regressions` target in Makefile for these four regressions, the startup model test, and any added focused modal admission regression. Run that focused target on your final signed source. Do NOT wait for #16394 to land: it cannot land until your fixes are integrated. Once focused checks pass, commit and task_report the implementation to this Oracle; #16394's Executor is the designated integrator and will cherry-pick your signed cut, run the full make test on the combined source, and land it. Do not run another full make test or independently push this branch. Keep final task reporting honest: focused target is your verify; full integration is #16394's obligation. No gate waivers, exclusions or operator input. Use existing Make targets, two Cargo jobs/worktree-local target. Full make test/test-formal currently use `/Users/feral/mydev/.bboxy-fulltest.lock`; release only your own claim. Targeted checks need no lock.

Your implementation is authorized for the integration landing through #16394; send the signed commit/report instead of independently landing. Copy this task record unchanged into the branch. Allocate your full report number through knowledge_write/Index, then copy the signed report into this worktree at .bboxy/reports/2026-11-03-startup-modal-input-regressions.md. Signed commit + Report trailer and task_report to this Oracle with actual commands/exits, source lines, remaining gaps and worktree state. Preserve origin headers.


## Upstream changed during your run (2026-11-03 20:21 UTC)

origin/main now includes signed 6f2eddce82536857f7a8561e65c9f8269b2e3c29, `Repair startup-modal regressions blocking the launch fix gate` (committed 12:11:52 PST after this task began). It already consolidates the native keyboard arm and attaches all three cursor fixtures. Rebase onto current origin/main immediately, keeping only justified additional normalized-input/repeat/IME correctness and the stronger focused modal regression/Make target. Do not duplicate or reimplement the four upstream fixes, and do not repeat a baseline full suite. Read the actual 6f2eddce diff: its startup gate is in the physical KeyboardInput arm; your normalized key_input/ime_input guard may cover a distinct real ctl seam, which must be demonstrated by your added test before retaining it. If no meaningful residual change remains, report the upstream commit and focused evidence instead of creating redundant code. #16394 remains the sole integration lander for any real residual cut. Notify it/the Oracle of your signed result through task_report as already instructed.
