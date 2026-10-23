# Terminal core — the stack, as decisions

Date: 2026-10-21. Derived from [[gpu-terminals-rust]], [[text-rendering]], [[cloud-and-mosh]], [[mux-daemon]], [[monorepo-weavie]]. Status: PROPOSED to the Oracle; operator decisions it implements are marked DECIDED; everything else is a recommendation with its reversal path.

Lens: minimise code we own that upstream already gets right; own every byte on the path that our memory model, crash model and agent protocol touch. Assumes the daemon-side VTE split in [[vision]]. Breaks when a seat needs a GUI-side terminal (it never does: the GUI holds a line cache).

## D1 — Fork base: `alacritty_terminal` at git HEAD, trimmed (DECIDED by operator; scope here)

Fork **alacritty/alacritty `5421470` (2026-10-16), `alacritty_terminal` 0.26.1-dev**, not the registry 0.25.1: HEAD carries the zero-width-char and Row-limit fixes and the kitty stack cap. Rename the package (weavie `[patch.crates-io]` would otherwise redirect every transitive `alacritty_terminal`, [[monorepo-weavie]] "Fork naming"). Apache-2.0 kept; `LICENSE-APACHE` and upstream attribution travel with the crate.

Scope (counted in [[gpu-terminals-rust]] §1):

| Keep (≈6.8k) | Cut (≈4.9k) | Change (≈0.4k) | Add (≈0.8k) |
|---|---|---|---|
| `grid/*`, `term/mod.rs`, `term/cell.rs`, `term/color.rs`, `index.rs`, `event.rs`, `sync.rs` | `tty/*` (portable-pty), `event_loop.rs` (mux owns the read loop), `thread.rs`, `vi_mode.rs`, `term/search.rs`, `selection.rs` | `term/mod.rs`: drop `selection` and `vi_mode_cursor` fields and their 113 references; drop `Config.vi_mode_cursor_style`; wire the new handler hooks | checkpoint export/import; OSC pass-through hook; seat-state hooks |

Reasons: selection and vi motions act on what the user sees, and under the daemon split the GUI sees its line cache, not a `Term` — so they move to the GUI over cached lines. Search runs over retained byte-ring history ([[mux-daemon]]) outside the viewport; evicted bytes are unavailable. The tty and event loop are replaced by D4 and the supervisor.

**Viewport-only grid is zero new code:** `Term::new` already builds the alt screen as `Grid::new(lines, cols, 0)` and `grid/mod.rs:209` has the `max_scroll_limit == 0` fast path. The daemon constructs the primary grid with `Config { scrolling_history: 0, .. }`. Keep `scrolling_history` configurable so a measurement harness can still run a 10k-history grid for parity tests against upstream ref-tests.

Reversibility: high while file layout matches upstream — `git diff` against `alacritty/alacritty_terminal/src` stays reviewable and upstream fixes to `grid/` and `term/mod.rs` cherry-pick. The alternative base, `rio-vt` 0.5.28 (MIT, 39k lines, same alacritty lineage plus images, sixel, OSC 133, single maintainer), loses on size and on carrying image protocols we defer; it stays the reference for OSC 133 and kitty graphics when images arrive. Writing our own VTE lost on time: 71 handler methods and the reflow code in `grid/resize.rs` are exactly the parts that take years to get right.

## D2 — Parser: fork `vte` 0.15.0 into the same crate (recommendation)

`alacritty_terminal` does not parse; `vte::ansi::Processor` does (`event_loop.rs:404`). Two requirements force the `vte` state machine into our tree:

1. **Checkpoints need parser state.** [[mux-daemon]] "Ring records" requires a checkpoint to carry "parser state including incomplete sequences". `vte` `Parser` (`lib.rs:55-70`) and `Processor`/`SyncState` (`ansi.rs:253-297`) are fully private, with no state accessor at all — not even "am I in Ground".
2. **OSC 7501/7502 must reach us** ([[subagent-detection]]). `ansi.rs:1523` drops unknown OSCs into a `debug!`; the `Performer` that could be wrapped is private (`ansi.rs:425`).

So: copy `vte/src/{lib,params,ansi}.rs` (4,144 lines, `Apache-2.0 OR MIT`) into `term-core/src/vte/`, keep its `Perform`/`Handler` contract, and make four edits: (a) `#[derive(Serialize, Deserialize)]` on `Parser`, `ProcessorState`, `SyncState` (the `State` enum is already plain data), (b) `Processor::{export, import}` returning/accepting that struct, (c) a `Handler::unhandled_osc(&mut self, params: &[&[u8]], bell_terminated: bool)` default no-op, called from the `_` arm at `:1523`, (d) a `Handler::sync_pending() -> bool` accessor so the mux scheduler can arm the 150 ms timeout (`ansi.rs:36`) without owning the `Timeout` impl. Checkpoint = `Processor` state + `Term` state + the ring cursor the checkpoint follows; a torn mid-escape sequence restores because the parser's `intermediates`, `params`, `osc_raw`, `partial_utf8` and the 2026 `buffer` are all in it.

Reversibility: medium. Upstream `vte` moves slowly (0.15.0 Feb 2025, repo quiet since Feb 2026), so drift is small; if upstream ever exposes state and an OSC fallback we delete the copy and re-depend. Keeping the crate layout (`vte/` subdir, same file names) makes that a `git diff`.

Alternative that lost: depend on `vte` and checkpoint only at Ground boundaries. It fails because Ground is not observable from outside, and because a seat in a long 2026 batch or a half-received OSC 52 payload would have no legal checkpoint for seconds.

## D3 — Renderer: fresh wgpu, lifting alacritty's design, built to the ghostty spec (recommendation; raw-wgpu DECIDED)

Costed both paths from source ([[gpu-terminals-rust]] §1 "The alacritty renderer"):

- **Port alacritty's renderer to wgpu**: 8,594 lines of `renderer/` + `display/` plus 407 lines of GLSL. Every struct holds GL object ids, every draw is a `gl::*` call, glutin owns the context, uniforms resolve by name. A port rewrites ≈4.5k of those lines and all shaders — it is a rewrite that keeps names. What it buys back is a 1024² single RGB atlas, no min-contrast, no linear-corrected blend, four-face font keys — the quality [[text-rendering]] rejected.
- **Fresh wgpu per [[text-rendering]] "What we write ourselves"**: ≈2.4k lines (run iterator 400, resolver 800, two atlases 300, cell contents 250, WGSL 600) + sprites 1–4k + frame scheduler. Same order of magnitude as the port, higher ceiling, no GL residue.

**Decision: fresh.** Lift from alacritty as design, not code: the 32-byte `InstanceData` layout (`glsl3.rs:284-300`), two passes over one instance buffer selected by a uniform (`text.v.glsl:26,55`), the 65,536-instance batch cap (`glsl3.rs:27`), corner-from-vertex-index (`text.v.glsl:36-37`), `RectKind {Undercurl, Dotted, Dashed}` procedural underlines (`rects.rs:46-50`), row damage tracking (`display/damage.rs`), and `RenderableCell` (`display/content.rs:189-198`) as the model→renderer contract — but ours is produced from the GUI's line cache, not from a `Term`. Everything else follows the ghostty shapes: `R8Unorm` mask + `Bgra8UnormSrgb` colour atlases grown ×2, `Bgra8UnormSrgb` surface (present on Metal, `wgpu-hal/src/metal/adapter.rs:487`), linear-corrected alpha, min-contrast, CoreText raster via `objc2`.

One `wgpu::Device` (cloneable handle, `wgpu/src/api/device.rs:21-22`) for the process; one `Surface` per window; per frame one encoder with ordered passes: cell backgrounds → cell text → chrome quads/text → effect passes (glow, blur) into offscreen textures → composite. Chrome shaders are additional pipelines on the same device, which is the whole point of raw wgpu. `wgpu 30.0.1` (MIT/Apache), `winit 0.30.13` (Apache-2.0) with `with_title_hidden + with_titlebar_transparent + with_fullsize_content_view` for the borderless title bar (as alacritty, `display/window.rs:348-355`). **Gap:** traffic-light repositioning needs `objc2-app-kit` `standardWindowButton` frame edits; not in winit (INFERRED from AppKit, unverified).

Reversibility: the renderer only consumes `RenderableCell` + cache rows, so it can be swapped whole; the alacritty port remains possible later but nothing in D3 depends on it. rio's `sugarloaf` is not a wgpu reference on macOS any more (wgpu is default-off there, native Metal preferred, `sugarloaf/Cargo.toml:40-45`); read its `grid/webgpu.rs:807-811` instance layout once and move on.

## D4 — PTY: `portable-pty` 0.9.0, held by the supervisor (DECIDED by operator; mechanism here)

MIT, 2,462 lines, `libc::openpty` at `unix.rs:36`, `setsid` + `TIOCSCTTY` in `pre_exec` (`unix.rs:257,271`), `TIOCSWINSZ` resize (`unix.rs:189`). The **supervisor** process ([[mux-daemon]]) owns `PtyPair`s so a logic-process crash never SIGHUPs a harness. Use `MasterPty::as_raw_fd()` (`lib.rs:114`) with one `polling` poller for all seats — never `try_clone_reader` threads at 1000 seats. `take_writer()` is one-shot per master; the supervisor takes it once and serialises writes from logic. `CommandBuilder` (`cmdbuilder.rs:203`) carries the harness argv, `env`, `cwd` and the role-prompt flags from the harness adapter. The ssh `MasterPty` impl in `wezterm-ssh` is not used: remote seats are a remote supervisor over our QUIC transport.

Reversibility: high. The trait surface is five methods; `rustix-openpty` (what upstream alacritty uses, `tty/unix.rs:21`) is a drop-in if portable-pty stalls (last release 2025-02-11, no unreleased unix changes at wezterm HEAD).

## D5 — Protocol features for v0.1 and where each is implemented

| Feature | Fork (daemon) | Mux | GUI |
|---|---|---|---|
| 24-bit colour, 256 palette, OSC 4/10/11/104 | upstream `Term` handlers | palette in checkpoint | renderer |
| OSC 8 hyperlinks | upstream `set_hyperlink` `term/mod.rs:1874`, `CellExtra` | serialise hyperlink ids in line frames (wezterm `SerializedLines{hyperlinks}` does the same) | hover, Cmd-click |
| OSC 52 clipboard | upstream, `Config.osc52` default `OnlyCopy` `:372-378` | per-seat policy set by the Oracle; `Event::ClipboardStore` → GUI | system clipboard |
| Bracketed paste 2004 | upstream mode bit `:61,1970,2029` | mode bit in the state stream | wrap paste in `ESC[200~ … ESC[201~` when set |
| Mode 2026 synchronized output | upstream `vte` buffering (`ansi.rs:298-312`) | **arm the 150 ms timeout per seat** from `sync_pending()`; suppress frames while pending | nothing |
| Kitty keyboard protocol | upstream stacks `:1275-1323`, `Config.kitty_keyboard = true` | mode flags in the state stream (`Screen.modes`) | **core invariant [[kitty-keyboard]] (decision #32, operator 2026-10-23):** one app encoder, `crates/bboxy450-term/src/keyboard.rs` (port of `alacritty/src/input/keyboard.rs:295` `build_sequence`, Apache-2.0), encodes every key against the focused seat's live mode with the full progressive-enhancement stack (flags 1/2/4/8/16, push/pop/query) and legacy encoding when no flags are set; no second key path to the PTY. K1 (Sable) |
| OSC 7501/7502 pass-through | **new** `Handler::unhandled_osc` → `Event::Osc(Vec<Vec<u8>>)` | subagent detection ([[subagent-detection]]), seat state | sidebar |
| OSC 133 semantic prompts | same hook, parsed later | waiting-on-input state (v0.2) | — |
| Images (kitty graphics, sixel) | deferred; `rio-vt/src/ansi/*` is the reference | — | — |

Gate: a `vttest`-style ref-test run. Keep upstream's `alacritty/tests/ref/*` harness working against the fork (`Term::new` with history for parity, viewport-only in production).

## D6 — Crate layout (matches [[monorepo-weavie]] tree)

```
crates/term-core/          the fork. src/{grid,term,index,event,sync}.rs as upstream; src/vte/{lib,params,ansi}.rs
                           pub: Term, Grid, Cell, Config, Processor{export,import}, Handler, Event, EventListener
                           no_std-free, no PTY, no threads. Deps: bitflags, serde, unicode-width, base64, log, parking_lot
crates/mux/                supervisor + logic: portable-pty, polling, byte ring, checkpoints, state stream, quinn, MCP
                           depends on term-core
crates/renderer/           raw wgpu: atlases, run iterator, WGSL, sprites, chrome pipelines. depends on ui-model, never on term-core
crates/ui-model/           seat/swarm/tab/split state enums shared with lab
crates/app/                bin bboxy450-term: GUI mode = winit + renderer + mux client; --daemon = mux
crates/lab/                component gallery on the real renderer
```

Dependency direction `term-core ← mux ← app`; `ui-model ← renderer ← {app, lab}`. The GUI renderer never depends on `term-core`: it renders cached lines received from the mux. The shared app executable includes the daemon dependency; this does not put a `Term` in the GUI rendering path. **Gap:** crates.io package names (`bboxy450-term-core`, …) — decide once in `Cargo.toml` ([[monorepo-weavie]]).

## D7 — Daemon↔GUI protocol versus wezterm's codec

Our state stream ([[mux-daemon]] "Logic ↔ GUI") is wezterm's `GetPaneRenderChanges`/`GetLines` model turned push-side: `Delta{base_seq, new_seq, rows}` replaces polled `GetPaneRenderChangesResponse{dirty_lines, seqno}` (`codec/src/lib.rs:915`), lazy `GetLines{Range<StableRowIndex>}` (`:933`) survives as history fetch by ring row index, and `Applied{seq}` is our ack. wezterm's 53-PDU surface (`:451-505`) is the checklist of what a mux client eventually asks for: spawn, split, zoom, focus direction, move-pane-to-tab, palette, alerts, client list, workspace rename. Frame it as `postcard`/`bincode` over length-prefixed streams as wezterm does (`:93-97`); the version handshake (`CODEC_VERSION`, `:445`) becomes our `Hello` capabilities. Not reused: wezterm's TLS bootstrap (we have key identity), its `Line` serialisation (ours is cell runs over our `Cell`).

## Order of work

1. `term-core` fork + `vte` copy, builds with upstream ref-tests green, selection/vi/search/tty removed. Executor task.
2. `Processor::{export,import}` + `Term` serde; property test: split any ref-test byte stream at every offset, checkpoint, restore, compare grids.
3. `unhandled_osc` + `Event::Osc`; test with the OSC 7501 capture from [[subagent-detection]].
4. `mux` supervisor on `portable-pty` + `polling`; viewport grids; ring.
5. Renderer per [[text-rendering]] with the D3 lifts; chrome on the same device.
6. GUI: kitty key encoding port, bracketed paste, selection over the line cache.

## Gaps

- **Gap:** checkpoint size and frequency per seat at 1000 seats — `Term` + two viewport grids ≈ 100 KB measured ([[cloud-and-mosh]]), serialised size not measured.
- **Gap:** whether `Grid.cursor`'s `template: Cell` with `Arc<CellExtra>` round-trips through serde with hyperlink identity preserved (upstream skips the cursor; we must not).
- **Gap:** traffic-light repositioning on a `fullsize_content_view` window — AppKit call, not winit.
- **Gap:** winit 0.30.13 vs 0.31 (beta since 2026-09) — pick at implementation time; wgpu 30 supports both via `rwh_06`.
- **Gap:** Linux: `portable-pty` and `wgpu` Vulkan are ready; font raster parity is the open question in [[text-rendering]].
- **Gap:** package names on crates.io.

[[vision]] [[text-rendering]] [[cloud-and-mosh]] [[monorepo-weavie]] [[mux-daemon]]
