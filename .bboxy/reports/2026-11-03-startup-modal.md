---
id: 01M6H8FH43JGSQ53MKBYZ2PS55
number: 93377961359199
type: report
title: Cockpit startup modal and advancing history progress
created: "2026-11-03T17:49:29Z"
updated: "2026-11-03T20:19:08Z"
author: seat:01M6HG4J9DHG29WDG4S0WYQFZ1
correction_author: seat:01M6HG4J9DHG29WDG4S0WYQFZ1
original_attribution: "Boxy <bboxy450@gmail.com>; machine key 9B07E849480C68328CA20684844ECC0587DE7AB0; original seat unproven"
commits:
- 1affb9865d75d775a9a5f306c043da8a05bf8f7c
task: '[[tasks/2026-11-03-startup-modal-operator-brief]]'
branch: feat/startup-modal
base: 2d73acbd
verify:
  command: "make build && cargo test -p bboxy450-term startup_modal_tracks_progress_failure_retry_and_reattach --lib && make test-licenses"
  exit_code: 0
---
# Cockpit startup modal and advancing history progress

Implemented in `/Users/feral/mydev/bboxy450-term-startup-modal`, on
`feat/startup-modal`. No merge, rebase, push, deployment or installation.
The signed commit containing this report is the handoff; its message carries
`Report: [[reports/2026-11-03-startup-modal]]`.

## Outcome and source evidence

- `crates/bboxy450-term/src/cockpit/startup.rs:48` — the modal model retains the
  current cockpit phase, its start instant, run directory, reconnect attempt and
  reason, refresh reason and sticky failure. Progress counts preserve the phase's
  elapsed time. An attachment hides the modal; failure requires explicit Retry
  before later observations can dismiss it.
- `crates/bboxy450-term/src/cockpit.rs:930` — `start_with_progress` reports real
  supervisor startup/adoption (including PID), logic startup, schema/endpoint
  refresh reasons and GUI admission. Recovery reports reconnect attempts and
  errors, retains the existing owner/watch restoration path and permits a local
  retry after failed recovery (`:1197`). Join failures now follow that error path.
- `crates/bboxy450-term/src/startup_modal.rs:19` — the centered toolkit modal
  shows phase, elapsed seconds, recovery context, error text and the actual
  `<run>/app.log` and `<run>/logic.log` paths. Quit is always available; failure
  adds Retry. It reuses `scene.rs:2390`'s dialog layout and `scene.rs:2418`'s
  frosted, dimmed backdrop. The existing Modal preset, spacing/type tokens and
  theme foreground supply light/dark styling; no eyebrow text was added.
- `crates/bboxy450-term/src/main.rs:8183` — the modal paints last over the empty
  or retained scene. The one-second wake timer updates elapsed time while the
  socket is unavailable. Mouse, wheel, IME, keyboard and native menu actions
  cannot activate underlying content. Quit/Cmd-Q retain detach semantics;
  Cmd-Option-Q retains hard quit and Cmd-Shift-Q is not mapped to app quit.
  `pointer_cursor.rs:30` supplies modal-only hit cursors.
- `crates/bboxy450-term/src/live.rs:250` — startup errors retain the IO owner and
  await local Retry or Quit instead of ending the app. Quit can cancel an active
  bootstrap. Recovery Retry uses a Notify independent of the GUI socket/command
  queue; if the IO owner ended, `startup_modal.rs:104` starts a new owner with the
  existing GPU context. `BackendStopped(Err)` now remains visible in the window.
- `crates/bbt-mux/src/store/events.rs:242`, `bus.rs:856`, `logic.rs:1955` — the
  actual signature/chain verifier supplies per-stream row counts through an
  explicit bootstrap callback. A single SQLite read snapshot supplies the count
  and records; observations occur at zero, every 128 verified rows and completion.
- `crates/bboxy450-term/src/cockpit.rs:771` — logic flushes additive
  `{"startup_progress":{"stream":"account","verified":128,"total":512}}`
  lines on its existing stdout before the unchanged readiness record. These
  public observations contain no capability. At `:1489`, the parent consumes
  progress until readiness and renews its 30-second deadline only on advancement;
  duplicate/backward counts do not renew it. The recovery-wide 30-second timer
  was removed so it cannot override advancing verification; supervisor, GUI hello
  and recovery catalogue waits remain bounded.
- `.bboxy/interfaces/logic-gui.md:165` records the additive contract and deferred
  evidence. `ATTRIBUTION.md:126`, the new source headers and synchronized crate
  attribution copies retain canonical origin and reuse policy links.
- `headless.rs` consumes the same phase observations and paints the modal over
  its produced scene. Its separate finite render-readiness/smoke watchdogs, and
  ctl's launcher timeout, remain unchanged.

## Verification and limits

`make build` — exit **0**, final source, dev profile optimized, 1m09s.
Log: `/tmp/bboxy-startup-modal-build.log`, SHA256
`a77194ef7b234f4513c7f2f4575620b27f5a0aebb0a703d6b2cb8274c197b7a6`.
The first attempt exited 2 because the worktree's pinned harness submodule was
uninitialized. `make init-submodules` then exited 0, pin
`131a2e0cc2c08ee862b570760d3e5ce44821f8cb`. A subsequent compile caught the new
SQLite count being read directly as u64; that conversion was corrected to read
i64 and checked-convert. The final build above passed after that correction.

`cargo test -p bboxy450-term startup_modal_tracks_progress_failure_retry_and_reattach --lib`
— exit **0**, **1 passed, 0 failed, 216 filtered out**, test runtime 0.00s,
optimized compilation 2m54s. Log: `/tmp/bboxy-startup-modal-unit.log`, SHA256
`ca418f64775d5d5e21d58478bcb098a87bfecbba4f71add8060d4e58f1cc627a`.
The test at
`cockpit/startup.rs:140` covers startup/adoption, phase elapsed time through
advancing history, sticky failure, retry, attachment, reconnect attempts/reasons,
refresh context and duplicate/backward progress admission. The first test compile
exited 101 with ENOSPC before any test ran. Only artifacts created by that failed
run inside this worktree's own `target/` were removed: 4,399 files, 1,674,005,624
bytes. The retry uses the Makefile's profile settings (DEV/TEST_DEBUG=0 and
DEV/TEST_OPT_LEVEL=1), two Cargo jobs and BBOXY_CACHE_MODE=off; no shared cache,
peer process, target or worktree was deleted or reconfigured. Original failure:
`/tmp/bboxy-startup-modal-unit-initial.log`.

`make sync-licenses` — exit **0**, canonical attribution copies synchronized for
the workspace's 34 crates (the independent embedded harness retains its own
attribution). No dependencies, Cargo.lock or submodule pin were changed.

`make test-licenses` — exit **0**, license fixtures and source-notice controls
passed; 34 workspace crates and source notices held. Log:
`/tmp/bboxy-startup-modal-licenses.log`, SHA256
`ddd1e4b26315a728ae1b788255eec260b784e47cc1ec16d369c91edcf9769b52`.
`git diff --check` — exit **0** before committing.

The ten changed Rust paths above plus `Cargo.toml`, `Cargo.lock` and
`rust-toolchain.toml` have aggregate SHA256
`7f28b48e6c38ddf96c082ea51563378d3755db70c0d4c9134c95aefef9c2829d`
(sorted relative path, NUL, file bytes, NUL, concatenated). These source bytes
were unchanged between the successful build and focused test. The signed
handoff commit binds those paths and this report; no formal receipt is claimed.

**Operator waiver, 2026-11-03:** “land loading modal really quickly while we are
building, skip tests while merging.” Formal and parity evidence is waived by the
operator for this merge and is still owed. `make test`, `make test-parity` and
`make test-formal` were not run. Formal models/negative controls and current-source
receipts, visual light/dark parity, boundary/process smoke and end-to-end Retry/
Quit/reconnect witnesses remain unproven. Source reuse of the existing themed
modal/backdrop is the present styling evidence. No operator input, Keychain,
desktop capture or OS input injection was used.

Follow-up: validate the new startup lifecycle/deadline with its formal mapping,
broken controls and real process witnesses; run visual parity in both themes.
The separate headless render-readiness watchdog still has its existing 30-second
total budget; aligning that fixture budget with history progress is also owed.

The pre-existing untracked `.bboxy-brief-startup-modal.md` is deliberately left
untouched and excluded from the commit. Only this task's exact paths are staged;
generated build outputs remain ignored in this writing worktree.

## Historical metadata correction, 2026-11-03

Corrected by seat:01M6HG4J9DHG29WDG4S0WYQFZ1 under
[[decisions/d16393-hash-pinned-report-corrections]]. The frontmatter author names
this correcting seat, not the original implementation author. Original attribution
remains Boxy <bboxy450@gmail.com>, signed with machine key
9B07E849480C68328CA20684844ECC0587DE7AB0 in implementation commit
1affb9865d75d775a9a5f306c043da8a05bf8f7c. The original seat identity is unproven;
no original seat is asserted. The evidence and waiver body above is retained
verbatim as historical testimony. This correction supplies metadata and a
resolvable retained operator brief; it claims no fresh implementation, formal,
parity, smoke or process verification of the startup modal.
