# bboxy450-term — R2 app shell

```sh
cargo run -p bboxy450-term --locked
```

The winit 0.30.13 window uses wgpu 30.0.1/Metal and the existing bbt-render
chrome/material passes. The embedded scene-15 split tree supplies 15 native
bbt-ui panes, one sidebar/tab and the title bar. Pane bodies are empty until
R3 connects terminal grids. The loop waits for input, damage or a coalesced
runtime wake; it has no polling/redraw timer or live animation yet.

Window/surface creation and presentation stay on the UI thread. Adapter/device
creation, asset decoding, initial text measurement and first-use renderer
preparation run on the shared bbt-rt runtime. Frame construction reads embedded
data and retained memory. Font widths/metrics are retained across layouts;
adjacent chrome draw operations are batched between material/group boundaries.
Dirty bits and bounded messages are drained at frame start under a 1 ms budget;
remaining messages request another frame. Updates use latest-value slots.

Cmd+Opt+F toggles the hitch meter. Its 1,024-sample memory ring retains frame
intervals plus drain, layout, draw, encode and present-wait timings. The overlay
shows the latest phase times and CPU p99. Frames exceeding 1.5 display intervals
enqueue the slowest phase and all CPU times to the bounded logger; stderr IO
runs asynchronously. Idle time between demanded frames is visible in the ring
and is not reported as CPU work.

Verification flags:

```sh
cargo run -p bboxy450-term --locked -- --smoke --meter --capture .bboxy/runs/r2-scene15.png
```

`--fake-source` publishes 20 finite updates to Nyx's title/activity through the
runtime's latest slot and bounded UiMsg queue. After update 20 is presented,
the app measures a full ten-second idle window. `--smoke` enables that source
and exits after the report is written. The idle check requires zero additional
presented frames and update 20 applied; it does not assert the later R5 CPU
budget. `--meter` starts with the overlay visible.

`--capture PATH` queues a copy of the exact composition submitted to the
surface (after update 20, or the first frame without the fake source). GPU
waiting, PNG encoding and writing all run off UI. Capture completion is logged.
Capture work is verification overhead outside the frame's recorded CPU sample.

```sh
cargo test -p bbt-rt --locked
cargo clippy -p bboxy450-term -p bbt-rt --locked --all-targets --no-deps -- -D warnings
scripts/ui-lint.sh
```

The scoped app/runtime Clippy check passes. The UI-wide hard gate currently
stops at three pre-existing bbt-ui-model build-script filesystem calls; R2 does
not edit those sources. The app's own build script has a reasoned build-time
exception. Full warning output and other existing renderer violations are in
the R1/R2 run logs.

R2 does not deliver terminal input, parser shards, window actions, display-link
pacing, 120 Hz animation or the R5 stall test. CPU timings remain above the R5
4 ms goal and are reported as measured. This step is the app shell and its
instrumentation, not a performance pass.
