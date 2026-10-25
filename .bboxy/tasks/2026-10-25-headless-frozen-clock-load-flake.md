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
updated: "2026-10-25T21:49:58Z"
---
# Make the headless frozen-clock pixel test independent of host load

Opened by Vesper for the quarantine row (#324 needs a unique task; #327 is the umbrella).

## Evidence

- Seraph's land.sh presubmit for #320 cut 2c (sha 66855ed2) failed only here: tests/headless 19/1, `headless.rs:1102:5` 'wall time and capture work cannot advance a frozen animation', 1600x900 frames differ. Log: ~/.local/state/bboxy450-term/landing/presubmits/66855ed2e2dcf2d5f35072cd14577982662812e6-05297072/commands.log.
- Second loaded red: Seraph's 2b precheck at load 30. Alone on the same binary at load 24-28 it passes 3/3 (6.6-7.8 s). Cut 2c moves only ctl parse arms and does not reach the frame fence or the clock.

## Acceptance (#327 B4)

The frozen-clock assertion holds under load: 10/10 with two concurrent cargo builds, no wall-clock dependency in the assertion, and a mutation that lets wall time advance the animation fails it.
