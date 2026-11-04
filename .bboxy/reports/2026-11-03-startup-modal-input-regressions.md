---
type: report
title: Startup modal shared normalized-input admission and focused regression evidence
task: '[[tasks/2026-11-03-startup-modal-input-regressions]]'
commits: []
verify:
  command: make test-startup-input-regressions
  exit_code: 0
author: seat:01M6HGCVKFA0E7CEGE8E7Q5D5P
created: 2026-11-03T20:35:52.421Z
id: 01M6HJ05KBS4G52WE7B547NX6P
updated: 2026-11-03T20:35:52.421Z
number: 17620
created_by: seat:01M6HGCVKFA0E7CEGE8E7Q5D5P
updated_by: seat:01M6HGCVKFA0E7CEGE8E7Q5D5P
authorship:
  signer: seat:01M6HGCVKFA0E7CEGE8E7Q5D5P
  kind: seat
  public_key: 7az818RFHIs+iUEUkggIsxBJE6C9BR33qwcjDZteyJo=
  process_generation: '1'
provenance:
  digest: sha256:bc060a4a554a948e1709dae0f3dd5831cd745920d38fff609254a869e5384439
  frontmatter_digest: sha256:dbd722d4988ec6ff18691998e9df7e0f99f2f4df0cbc8fcdf74c294b5e363d7e
  signer: seat:01M6HGCVKFA0E7CEGE8E7Q5D5P
  sig: hg4zb2kZq4CoraiMT4Zwgz4PEuZsYU2aWc+lo/UiYV4KZNTfDM1uvcLxxwTGMa46YlptEbiXzKa9hpsz1nEjBQ==
  log: .bboxy/log/2026-11-03.part-86213.jsonl#100048
---
# Startup modal admission through the shared input route

Assignment: [[tasks/2026-11-03-startup-modal-input-regressions]] (#16417), including the 20:21 UTC amendment. Worktree: `/Users/feral/mydev/bboxy450-term-startup-input-fix`, branch `startup-input-fix`.

The original five-test run on adf9796b reproduced all four reported failures and passed the startup modal state test. During compilation, upstream 6f2eddce82536857f7a8561e65c9f8269b2e3c29 fixed the physical keyboard route and all three attached-state fixtures. I read that diff and rebased onto current origin/main, 6afd6314. The final cut leaves those upstream fixture changes intact and changes only the remaining normalized-input admission seam, its evidence, and the canonical focused target.

## Implementation and acceptance evidence

- `crates/bboxy450-term/src/main.rs:7139` — startup admission now runs in the shared normalized `key_input` method used by both winit and private ctl input. It retains Enter Retry, Command-Q Quit, Command-Option-Q hard quit, and consumes presses so held repeats/releases cannot become terminal input after attachment.
- `crates/bboxy450-term/src/main.rs:7069` — the shared IME handler traps commits while startup is visible, including ctl commits that never visit a native window callback.
- `crates/bboxy450-term/src/main.rs:9172` — the single physical keyboard arm adapts input once and reaches the shared handler. Capture shortcuts retain startup blocking. `crates/bboxy450-term/src/keyboard.rs:728` retains its original count-equals-one assertion and encoding/source assertions unchanged.
- `crates/bboxy450-term/src/main.rs:10488` — one isolated private-HOME/headless App regression retains the real initially visible modal and constructs an underlying live seat. It checks cold-start default cursor, failure remaining visible despite attachment, actionable Retry/Quit cursors, Retry without repeated activation, and Quit/hard-Quit without repeat/release activation.
- `crates/bboxy450-term/src/main.rs:10560` — the regression calls the production `App::apply_ui(UiRequest::Input)` dispatcher with a private test identity and disconnected command receiver. A cold-start structured press admits zero terminal commands. `:10606` checks that a structured IME commit leaves the covered composer unchanged. `:10623` checks production attachment, underlying pointer restoration, no held repeat/release leakage in Kitty all-key/event mode, and the fresh press's exact `ESC[97u` admission.
- The temporary broken implementation removed the two shared startup guards. The same API witness failed because the cold-start structured press queued one command instead of zero. The source was restored before the passing run. This demonstrates a remaining ctl admission defect in a physical-callback-only gate; it is not a claim about an external socket/process test.
- Upstream fixture transitions remain at `app_overlays.rs:1219`, `pointer_cursor.rs:318`, and `terminal_clipboard.rs:300`. The final diff does not modify these files. All original overlay, pointer, selection and link assertions remain and pass in the focused target.
- `Makefile:460` — `make test-startup-input-regressions` runs the four named regressions, the existing startup state test, and the added admission witness through nextest, with the worktree-local target and two Cargo jobs.
- `verification/contracts.toml:461` — maps the startup witness onto the existing NativeInputOwnership/BrokenNativeInputOwnership abstraction and names remaining implementation-only modal/IME/cursor coverage. The finite ownership model and broken control are unchanged; this cut adds no lifecycle redesign.
- The assignment is copied unchanged from the signed host record. Source origin headers and ATTRIBUTION.md are retained. No style, geometry, production startup default, desktop interaction, private-home isolation, or unrelated report repair was changed.

## Commands and actual outcomes

All commands ran unattended in this worktree. Rust checks used `scripts/cache-env.sh`, two Cargo jobs, and local `target/`.

| Command | Exit | Result |
| --- | ---: | --- |
| Initial `cargo nextest run -p bboxy450-term -E '<the four named regressions plus startup_modal_tracks_progress_failure_retry_and_reattach>'` | 102 | Bootstrap prerequisite missing: pinned harness submodule was not initialized. |
| `make init-submodules` | 0 | Initialized pinned harness 0c403165e815c2f7bf77ccac293784b021f42f96. |
| Same initial five-test command, before edits | 100 | Five tests ran: startup state test passed; all four reported regressions failed with the reported adapter-count/cursor assertions. |
| `make test-startup-input-regressions` before rebase | 0 | Six tests passed; superseded as final evidence by the stronger rebased API witness. |
| `cargo nextest run --locked -p bboxy450-term --target-dir target --bin bboxy450-term -E 'test(startup_modal_owns_normalized_keys_and_cursors_until_attachment)'`, shared guards temporarily removed | 100 | Expected negative control: structured cold-start input queued 1 command, expected 0. |
| `make test-startup-input-regressions`, restored rebased source | 0 | Six tests across two binaries passed; 436 outside the focused filter were not run. |
| `make test-licenses`, rebased source | 0 | Six licensing fixtures, 34-crate policy/notice checks, source notices (810 owned files, zero changes), and nine negative fixtures passed. |
| `git diff --check` | 0 | No whitespace errors. |

Raw logs are retained locally under `target/startup-input-*.log`; they are not committed. The containing signed commit carries the Report trailer; `commits: []` avoids a self-referential hash. The final signed-source replay, exact SHA, signature verification and clean-worktree result are supplied in task_report.

## Remaining integration obligations

The 20:08/20:21 amendments designate #16394 as the sole integration lander. No independent push or full-suite run was made here. Full `make test`, TLC/Yarn parity, all-target lint, process/e2e/smoke and documentation corpus checks on the combined source, and landing remain that integrator's obligations. This focused result claims none of those checks. No gate waiver, test exclusion, operator input, shared full-test lock claim, or main-green worktree write was used.
