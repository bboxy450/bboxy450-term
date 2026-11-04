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
status: accepted
delegated_by: seat:01M6HF0S1BAJKHWT2N426W7328
swarm: 01M6CQARWH5A79TE3H4NNWJ4XG
updated: 2026-11-03T20:46:38.276Z
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
artifacts:
- '[[01M6HJ1Y0K4CQ4FGKSKGFS9QT6]]'
verify_result:
  command: make test-startup-input-regressions
  exit_code: 0
  evidence:
  - criterion: Single neutral keyboard route and unchanged assertions
    reference: 'crates/bboxy450-term/src/main.rs:9172; crates/bboxy450-term/src/keyboard.rs:728; target/startup-input-final.log: 6/6 passed'
  - criterion: Private ctl input and IME are blocked during startup
    reference: 'crates/bboxy450-term/src/main.rs:7139,7069,10560,10606; target/startup-input-broken.log: expected failure (1 admitted vs 0), exit 100; restored final run exit 0'
  - criterion: Cold/failure blocking, Retry/Quit and post-attachment held-key ownership
    reference: crates/bboxy450-term/src/main.rs:10488,10565,10614,10623; final focused regression passes
  - criterion: All existing overlay/pointer/selection assertions remain
    reference: Inherited 6f2eddce attachment transitions at app_overlays.rs:1219, pointer_cursor.rs:318, terminal_clipboard.rs:300; these files have no residual diff; all four original tests pass
  - criterion: Final signed source and clean worktree
    reference: '7eb6ead826ff104acf52f1f9bc383e4575ec848b; target/startup-input-signatures.log: 1 verified, exit 0; git status --porcelain empty; final focused Make target exit 0'
  - criterion: Attribution and remaining formal coverage
    reference: 'target/startup-input-licenses.log: make test-licenses exit 0; verification/contracts.toml:461 records existing ownership abstraction and implementation-only modal coverage; full integration delegated to #16394'
report:
  summary: 'Signed residual cut 7eb6ead826ff104acf52f1f9bc383e4575ec848b on 6afd6314; report #17620. Read upstream 6f2eddce and rebased, retaining its four original fixes without duplicating fixture edits. Moved startup admission from the physical callback into shared key_input/ime_input, preserving Retry/Quit and held-key ownership. Added the canonical six-test Make target and a private headless App::apply_ui witness: the removed-guard control fails (exit 100, one leaked command vs zero), restored and final signed-source runs pass (exit 0, 6/6); make test-licenses exits 0; pinned signature check verifies 1/1; worktree clean. Final log target/startup-input-final.log SHA256 9be3b8302d4db171333741bcaa8e6ac5dec709298b021cf77d541e6a846e5b04. No independent push/full gate; #16394 remains the sole integration lander and owns combined formal/unit/e2e/smoke/docs checks.'
  commit: 7eb6ead826ff104acf52f1f9bc383e4575ec848b
  reported_by: seat:01M6HGCVKFA0E7CEGE8E7Q5D5P
  op_id: 01M6HJ1Z9WTD5CCJJWTBZDE8F1
  verify_revision: 1
  report: '[[reports/2026-11-03-startup-modal-input-regressions.md]]'
provenance:
  digest: sha256:c42838eb80437b0242dde6dc27b81d56f12ff8e745c9fd1d5eb21b53823bb244
  frontmatter_digest: sha256:6bef7f614d28778a1eacc0cf9bd28e1e8c66b8125d369557348744ed753ef168
  signer: seat:01M6HF0S1BAJKHWT2N426W7328
  sig: F/uUJ2BIkVH6KpJ/NwCYo4n8lL6yn5qnNRWXMQtusaptt/f6fiObltUlmsrmZJ49VwpI48rTyKV30kvla5TECA==
  log: .bboxy/log/2026-11-03.part-86213.jsonl#100309
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


## Report from seat:01M6HGCVKFA0E7CEGE8E7Q5D5P

Signed residual cut 7eb6ead826ff104acf52f1f9bc383e4575ec848b on 6afd6314; report #17620. Read upstream 6f2eddce and rebased, retaining its four original fixes without duplicating fixture edits. Moved startup admission from the physical callback into shared key_input/ime_input, preserving Retry/Quit and held-key ownership. Added the canonical six-test Make target and a private headless App::apply_ui witness: the removed-guard control fails (exit 100, one leaked command vs zero), restored and final signed-source runs pass (exit 0, 6/6); make test-licenses exits 0; pinned signature check verifies 1/1; worktree clean. Final log target/startup-input-final.log SHA256 9be3b8302d4db171333741bcaa8e6ac5dec709298b021cf77d541e6a846e5b04. No independent push/full gate; #16394 remains the sole integration lander and owns combined formal/unit/e2e/smoke/docs checks.

```json
{
  "op_id": "01M6HJ1Z9WTD5CCJJWTBZDE8F1",
  "commit": "7eb6ead826ff104acf52f1f9bc383e4575ec848b",
  "report": "[[reports/2026-11-03-startup-modal-input-regressions.md]]",
  "verify_result": {
    "command": "make test-startup-input-regressions",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Single neutral keyboard route and unchanged assertions",
        "reference": "crates/bboxy450-term/src/main.rs:9172; crates/bboxy450-term/src/keyboard.rs:728; target/startup-input-final.log: 6/6 passed"
      },
      {
        "criterion": "Private ctl input and IME are blocked during startup",
        "reference": "crates/bboxy450-term/src/main.rs:7139,7069,10560,10606; target/startup-input-broken.log: expected failure (1 admitted vs 0), exit 100; restored final run exit 0"
      },
      {
        "criterion": "Cold/failure blocking, Retry/Quit and post-attachment held-key ownership",
        "reference": "crates/bboxy450-term/src/main.rs:10488,10565,10614,10623; final focused regression passes"
      },
      {
        "criterion": "All existing overlay/pointer/selection assertions remain",
        "reference": "Inherited 6f2eddce attachment transitions at app_overlays.rs:1219, pointer_cursor.rs:318, terminal_clipboard.rs:300; these files have no residual diff; all four original tests pass"
      },
      {
        "criterion": "Final signed source and clean worktree",
        "reference": "7eb6ead826ff104acf52f1f9bc383e4575ec848b; target/startup-input-signatures.log: 1 verified, exit 0; git status --porcelain empty; final focused Make target exit 0"
      },
      {
        "criterion": "Attribution and remaining formal coverage",
        "reference": "target/startup-input-licenses.log: make test-licenses exit 0; verification/contracts.toml:461 records existing ownership abstraction and implementation-only modal coverage; full integration delegated to #16394"
      }
    ]
  }
}
```
