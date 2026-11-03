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
updated: 2026-11-03T20:08:55.292Z
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
  digest: sha256:908fba1ddea06486292abf71fd5d804ab99d0dbcdb4ef52cd7a6cf59f031798a
  frontmatter_digest: sha256:9c01429720d1e35565de49733df9de88fc0a1d04f54e0f33b55a31e535140338
  signer: seat:01M6HF0S1BAJKHWT2N426W7328
  sig: WS7mFPHozhKoJH67u7WYqtSprQ6rOS8HPbPD+n5daTWmyAmkMa+11CBmJf4xAC3GpijBbrOma7mgR6mSGu58Cg==
  log: .bboxy/log/2026-11-03.part-86213.jsonl#95407
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
