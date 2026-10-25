---
id: "01M5TGNDBDE2QB8K3BHH69216M"
type: "task"
number: 336
title: "Make the headless frozen-clock pixel test independent of host load"
status: "open"
role: "executor"
owner: "Sable Embertail"
verify: "cargo test -p bboxy450-term --locked --test headless test_account_ui_clock_freezes_pixels_steps_focus_and_resumes_live_time"
created: "2026-10-25T21:49:58Z"
updated: "2026-10-25T22:35:54Z"
---
# Make the headless frozen-clock pixel test independent of host load

Opened by Vesper for the quarantine row (#324 needs a unique task; #327 is the umbrella).

## Evidence

- Seraph's land.sh presubmit for #320 cut 2c (sha 66855ed2) failed only here: tests/headless 19/1, `headless.rs:1102:5` 'wall time and capture work cannot advance a frozen animation', 1600x900 frames differ. Log: ~/.local/state/bboxy450-term/landing/presubmits/66855ed2e2dcf2d5f35072cd14577982662812e6-05297072/commands.log.
- Second loaded red: Seraph's 2b precheck at load 30. Alone on the same binary at load 24-28 it passes 3/3 (6.6-7.8 s). Cut 2c moves only ctl parse arms and does not reach the frame fence or the clock.

## Acceptance (#327 B4)

The frozen-clock assertion holds under load: 10/10 with two concurrent cargo builds, no wall-clock dependency in the assertion, and a mutation that lets wall time advance the animation fails it.

## Artifact verification (Sable)

- Read the cut 2c raw log directly: `commands.log:1207` names the failed test; `:1214-1215` names `tests/headless.rs:1102:5` and the frozen-animation assertion. SHA-256 `eb1de68c497f600cdd367fd3e72e1f77d3f4651ba77beafd9042032dc5dbbae3`, 49,306,919 bytes. The large size is the assertion's full frame-buffer dump; no transcript or image data is copied into this record.
- Seraph clarified that the three isolated outputs were only in its session, with no retained log files. Exact reported command: the same presubmit's `headless-436e36e5f068bfbb` binary with the full test name and `--exact`, using memory custody. Reported durations were 6.63, 6.81 and 7.84 seconds at load 24.46/27.32/28.40. These remain a peer report, not independently retained controls.
- L3 author cycle on `819fdca0b10b5e9eb42a7236bed5ad890bd89b81` later passed the full headless suite, 20/0 including this test, in 20.00 seconds. This is a passing control, not a cause or repair claim. Its raw log is `~/.local/state/bboxy450-term/landing/presubmits/819fdca0b10b5e9eb42a7236bed5ad890bd89b81-3ac6c645/commands.log`.

Cause remains unproven. Keep every pixel, explicit-step, focus and resumed-clock assertion; no tolerance or sleep is added here.
