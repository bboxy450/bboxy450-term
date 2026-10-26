---
id: "01M5TGNDBDE2QB8K3BHH69216M"
type: "task"
number: 336
title: "Make the headless frozen-clock pixel test independent of host load"
status: "done"
role: "executor"
owner: "Sable Embertail"
verify: "cargo test -p bboxy450-term --locked --test headless test_account_ui_clock_freezes_pixels_steps_focus_and_resumes_live_time"
created: "2026-10-25T21:49:58Z"
updated: "2026-10-26T00:15:51Z"
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

## Cause proven from the failed buffers (Sable)

- Parsed the exact left/right 1600×900 RGBA arrays in the retained failed log, without rerunning the test: 621 changed pixels, bounding box x=163..278, y=236..246. All pixels outside that sidebar text region are equal. Private `frozen-clock-buffer-diff.json` retains the hashes/counts; both reconstructed sidebar crops were read.
- The left crop says green Build box / 40 ms; the right says red fixture link dropped. `crates/bbt-mux/fixtures/providers.json:15` schedules that transition at 12000 ms. `provider/fake.rs:26` uses its own RealClock; `logic.rs:1227` supplies that clock to the fixture providers. Freezing the UI animation clock does not freeze scripted backend host state.
- This is a fixture-quiescence failure: a load-delayed capture crosses the provider's independent 12-second transition. It does not establish an animation-clock fault.
- Oracle approval `5158b93e-ed83-4f57-b25a-cacf882ec213`: add an own-test-account/debug-only fixture-host opt-out at `cockpit.rs::start_logic`, used only by this headless fixture. Its selector/parser must be absent in release; prove that with a release-mode check. Keep every pixel, explicit-step, focus and resumed-clock assertion, with no tolerance or deadline change.
- keeps: scripted-host transitions remain covered by `bbt-mux/tests/provider_fake.rs::all_requests_keep_their_reply_and_down_requests_never_execute_while_events_wait_for_rtt` and `headless_fixture_routes_real_ptys_survives_drop_and_recovers_a_down_host_on_restart`; both explicitly check fixture link dropped. Those tests are untouched.

private_root: /var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/sable-launch-night-iloqd6d6
## Repair and verification

- `cockpit/fixture_hosts.rs` gates the off selector under `debug_assertions` and a test-account Config; `start_logic` uses it for the existing bootstrap field. Only the frozen-clock headless fixture selects it. No provider or animation code change is retained.
- Compiled selector regression 1/0; actual module built in both modes, unset/off/unrecognized readback checked, selector bytes absent from release output. All existing frozen-clock assertions remain byte-identical except the added fixture setting/comment.
- Ten exact pixel tests passed with two independent Cargo release builders active before and after every run (2.151–3.242 s). Both builders were intentionally stopped after the tenth run and their recorded descendants were absent. Own load targets had no open files and were removed.
- A paint-only wall-clock mutation failed the exact frozen-pixel assertion, EXIT101; its source was restored byte-for-byte. A broader first mutation was caught earlier by the frame-time acknowledgement, retained as a separate negative control. Restored-source cargo test EXIT0, 1/0 in 2.89 s.
- Evidence and limits: `.bboxy/runs/headless-frozen-clock-load.log`; detailed commands, PIDs, results and hashes under private_root above. Canonical author validation and landing are reported with the handoff; no broad 327 audit claim.
