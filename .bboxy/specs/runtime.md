# Runtime — threads, async IO, 120 Hz frame pacing

Owner: Vesper Greywind <ORACLE>. 2026-10-22. Status: DECIDED for v0.1. Operator: "IO async, if we run background commands or stream stdout or do io it should never block the UI thread, and we should render at 120hz." Builds on [[renderer]] §1 (retained, damage-driven), [[mux-daemon]] (supervisor), [[events]] (bus), [[terminal-core]].

Lens: macOS AppKit + winit + wgpu/Metal on Apple silicon. Assumes ProMotion or 60 Hz panels. Breaks when: a platform forces rendering off the main thread (none we target).

## 1. The one invariant

**The UI thread never waits on IO.** No file, socket, process, pipe, sleep, blocking channel `recv`, `block_on`, or lock that an IO thread can hold for longer than a copy. Everything the UI shows arrives as a message or a published snapshot; everything the UI asks for leaves as a message. A frame is built only from memory the UI thread already owns.

Enforced three ways, cheapest first:
1. `clippy.toml` `disallowed-methods` for the UI crates (`bbt-ui-model`, `bbt-ui`, `bbt-render`, the app's `ui` module): `std::fs::*`, `std::process::*`, `std::thread::sleep`, `std::sync::mpsc::Receiver::recv`, `Mutex::lock` (use `try_lock`), `tokio::runtime::Handle::block_on`.
2. Debug builds: `rt::assert_not_ui_thread()` at the top of every blocking API in the IO crates; panics with the call site.
3. The stall test (§6) runs in CI.

## 2. Threads

| Thread | Owns | Never does |
|---|---|---|
| **UI (main)** | winit/AppKit loop, input, model, layout, draw list, wgpu encode and present | IO of any kind |
| **IO runtime** | one tokio multi-thread runtime, 2 workers + blocking pool: quinn, MCP server, bus, store, file IO, background commands (`tokio::process`), doc loading | touch UI state directly |
| **Supervisor** | existing `bbt-mux` mio thread: PTYs, leases (W2.1, keep as built) | parse terminal bytes |
| **Parsers** | N = cores/2 threads; each owns the `Term` of a shard of seats (seat id hash); vte parse, grid, damage | render |

Tokio because quinn and the MCP stack already need it; one runtime, not one per subsystem. The supervisor stays mio: it works and is tested.

## 3. Data into the UI

- **Wake, coalesced.** Producers set a per-source dirty bit; only the 0→1 transition sends `EventLoopProxy::send_event(Wake)`. A flood of 1000 seats costs at most one wake per frame.
- **Messages.** One bounded `UiMsg` queue. The UI drains it at frame start under a 1 ms budget; the rest waits for the next frame. Producers never block on it: state uses latest-value slots (`watch` semantics), streams use bounded rings that drop oldest *from the view only* (the byte ring and store keep everything, never-delete rule).
- **Terminal grids.** Each parser publishes damaged rows into a per-seat double buffer. The UI takes the front buffer with `try_lock`; if a parser holds it, the UI draws last frame's rows and retries next frame. Only on-screen seats are read; 1000 off-screen seats cost zero UI time.
- **Background commands.** `rt::spawn_command(argv, cwd) -> TaskId` streams stdout/stderr as line-chunked `UiMsg::Output{task, bytes}` into a view ring, exit status as `UiMsg::Exited`, cancel by `TaskId`. Same path for harness tools, `code_search`, git, anything.

## 4. Frame pacing — 120 Hz

- **On demand.** A frame is drawn only when something changed (damage, input, message) or an animation is live. Idle draws zero frames.
- **Display-locked when animating.** Drive frames from the display link (`CAMetalDisplayLink` on macOS 14+, `CVDisplayLink` fallback) with `preferredFrameRateRange` 80–120–120; without the explicit range ProMotion parks Metal apps at 60. Animation time comes from the link's target timestamp, never wall-clock deltas.
- **Present.** `PresentMode::Fifo`, `maximumDrawableCount` 3, frame latency 2. Never `Immediate`.
- **Budget at 120 Hz: 8.33 ms.** UI-thread CPU ≤ 4 ms (drain, layout, draw list, encode), GPU ≤ 6 ms. Retained geometry and damage per [[renderer]] §1 are what make this cheap; no other optimisation until the meter says so.

## 5. Hitch meter

Always on, cheap: a ring of the last 1024 frame intervals and UI-thread CPU times, tagged by phase (drain, layout, draw, encode, present wait). Any frame over 1.5× the display interval logs the phase that overran. bboxybook and the app expose it as an overlay (`Cmd+Opt+F`) and over MCP (`perf_frames`). This is the instrument for every later performance decision.

## 6. Stall test (acceptance for the runtime)

Headless and windowed variants. Load: 1000 seats running `yes`-speed output, a background command producing 1 GB of stdout, a file read stalled 2 s (fault-injected), a quinn peer that never answers, the bus at 10k events/s. Assert while it runs: UI-thread CPU p99 ≤ 4 ms, zero UI frames waiting on a lock or IO (from the hitch meter tags), the 15 visible seats keep updating, input-to-photon for a keystroke within 2 frames. Windowed on a ProMotion Mac: measured cadence 120 Hz while the glow animates.

## 7. Order

R1 `bbt-rt` crate (runtime, wake, UiMsg, spawn_command, lint config) → R2 app shell (winit loop on bbt-rt, renderer, model, hitch meter) → R3 terminal data path (parsers, double buffers) ∥ R4 display-link pacing → R5 stall test. Briefs: [[2026-10-22-r-runtime]].

## Gaps

- **Gap:** does winit 0.30 expose enough to attach `CAMetalDisplayLink` to its view, or does R4 need an objc2 shim on the raw NSView? — no data; R4 measures first.
- **Gap:** parser thread count for 1000 seats on an M1 base — no data; R5 measures.

Links: [[renderer]] [[mux-daemon]] [[events]] [[terminal-core]] [[vision]]
