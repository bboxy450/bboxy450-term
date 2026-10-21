# GPU terminal stack in Rust — candidates, dated facts, file:line pointers

Date: 2026-10-21. Author: Oracle research subagent for Vesper Greywind <ORACLE>. Task: [[2026-10-21-gpu-terminal-research]].

Serves the operator's decisions (fork `alacritty_terminal`, `portable-pty`, wezterm mux read-only, raw wgpu, one binary two modes, macOS 14 floor, VTE in the mux daemon). Does not reopen them. Text rendering is covered in [[text-rendering]] (ghostty pipeline, Rust crate mapping); alacritty grid memory is measured in [[cloud-and-mosh]] §3; neither is redone here.

Clones (shallow, not committed): `scratchpad/clones/{alacritty,rio,wgpu,winit,wezterm,ghostty,zellij}`. Registry sources: `~/.cargo/registry/src/index.crates.io-*/{alacritty_terminal-0.25.1,vte-0.15.0,portable-pty-0.9.0}`. `alacritty` pointers below are from git HEAD `5421470` (2026-10-16, `alacritty_terminal` 0.26.1-dev) unless marked 0.25.1. Counts marked **counted** are `wc -l` / `grep -c` / `git log | wc -l`; dates marked **API** are from `gh api` or crates.io; anything else is read from source.

---

## 1. alacritty / alacritty_terminal

**License:** `alacritty_terminal/Cargo.toml:5` `Apache-2.0`; app crate `alacritty/Cargo.toml:5` `Apache-2.0`; `alacritty_config*` `MIT OR Apache-2.0`. Repo ships `LICENSE-APACHE` and `LICENSE-MIT`.

### Activity — dated facts (the "abandoned" question)

| Fact | Value | Source |
|---|---|---|
| Latest release | **v0.17.0, 2026-04-06** (`alacritty_terminal` 0.26.0 same day) | API: `releases/tags/v0.17.0`, crates.io versions |
| Previous releases | v0.16.1 2025-10-20, v0.16.0 2025-10-18, v0.15.1 2025-02-16, v0.15.0 2025-01-13 | API |
| `alacritty_terminal` on crates.io | 0.26.0 2026-04-06 · 0.25.1 2025-10-18 · 0.25.0 2025-02-16 | API |
| HEAD | `5421470` 2026-10-16 "Fix Kitty keyboard mode stack size limit" | `git log -1` |
| Commits, 2025-10-21 → 2026-10-21 | **37** (counted). Per month: 2025-10 7 · 11 7 · 12 3 · 2026-01 2 · 02 1 · 03 8 · 04 2 · 05 3 · 06 3 · 07 1 · 08 4 · 09 0 · 10 2 | `git log --since` on a `--shallow-since=2025-04-01` clone |
| Authors, same window | Christian Duerr 11, Kirill Chibisov 6, eight others 1–2 each | counted |
| Open issues / open PRs | **328 issues, 14 PRs** (repo `open_issues_count` 343 = both) | API search, 2026-10-21 |
| Archived | no; 66,015 stars | API |
| `vte` (alacritty/vte) | crates.io 0.15.0 **2025-02-02**; repo last push 2026-02-28; last GitHub release tag v0.13.0 (2023) — tags lag crates | API |
| `crossfont` | crates.io 0.9.0 2025-06-09; repo last push 2025-07-08 | API |

Read: **not abandoned, maintenance mode.** Two active maintainers, ~3 commits/month, a release roughly every six months, bug-fix only (every commit in the last 15 is a fix). Bus factor 2. A fork is justified by the direction we need (viewport-only grid, checkpoint export, OSC pass-through), not by death. Forking from HEAD rather than 0.25.1 picks up `ede2ac1` (zero-width char limit) and `d692748` (Row column limit) and the kitty stack fix.

### alacritty_terminal — crate layout (counted, HEAD)

```
src/lib.rs            20   pub mod event, event_loop, grid, index, selection, sync, term, thread, tty, vi_mode; pub use vte
src/grid/mod.rs      656   Grid<T>: raw: Storage<T>, cursor, saved_cursor, columns, lines, display_offset, max_scroll_limit (:110-138)
src/grid/storage.rs  769   ring of Row<T>; tests from :266
src/grid/row.rs      275   Row<T>{inner: Vec<T>, occ} (:17-25)
src/grid/resize.rs   389   reflow on resize
src/grid/tests.rs    390   tests
src/term/mod.rs     3302   Term<T>, Config, TermMode, Handler impl, RenderableContent; tests from :2507
src/term/cell.rs     331   Cell 24 B (0.25.1 :134-139, measured in [[cloud-and-mosh]])
src/term/color.rs     60   Colors([Option<Rgb>; COUNT])
src/term/search.rs  1251   regex search over grid; tests from :687
src/index.rs         467   Line/Column/Point; tests from :353
src/selection.rs     668   mouse selection model; tests from :395
src/vi_mode.rs       893   vi cursor motions; tests from :412
src/event.rs         110   Event enum + EventListener trait
src/event_loop.rs    486   PTY read loop + vte::ansi::Processor (:404)
src/sync.rs           49   FairMutex
src/thread.rs         11
src/tty/mod.rs       154   Options, Shell, EventedReadWrite (:65), EventedPty (:92), setup_env
src/tty/unix.rs      448   rustix_openpty::openpty (:21, :196), setsid (:251), TIOCSWINSZ (:414)
src/tty/windows/*    999   conpty
total              11728
```

Deps (`Cargo.toml`): `vte 0.15.0 features ["std","ansi"]`, `unicode-width 0.2`, `regex-automata`, `polling 3.8`, `parking_lot`, `bitflags`, `base64`, unix: `rustix-openpty 0.2`, `rustix 1`, `signal-hook`. Feature `serde` (default on) derives Serialize on `Grid`/`Storage`/`Row`/`Cell`/`Line`/`Column`/`Point`.

### Where history is allocated (file:line)

- `term/mod.rs:410-415` `Term::new` → `let history_size = config.scrolling_history; Grid::new(num_lines, num_cols, history_size)`; `inactive_grid = Grid::new(num_lines, num_cols, 0)` (alt screen has no history). `Config::default().scrolling_history = 10000` (`:334-352`).
- `grid/mod.rs:141-144` `Grid::new(lines, columns, max_scroll_limit)` → `raw: Storage::with_capacity(lines, columns)` — **only the viewport is allocated up front**.
- `grid/storage.rs:67-75` `with_capacity` allocates `visible_lines` rows of `Row::new(columns)` (`row.rs:34-37`, `columns` default cells).
- History grows lazily: `grid/mod.rs:175-180` `increase_scroll_limit(count)` → `storage.rs:126-138` `initialize(additional_rows, columns)` reallocs by `max(additional_rows, MAX_CACHE_SIZE=1_000)` rows (`storage.rs:13`). Called from `scroll_up` (`grid/mod.rs:252-268`).
- Shrink: `grid/mod.rs:154-160` `update_history`, `:383-385` `clear_history` → `storage.rs:107` `shrink_lines`, `:118` `truncate`.
- **Zero-history mode already exists**: `grid/mod.rs:209` `if self.max_scroll_limit == 0 { /* rotate whole buffer */ }` fast path. A viewport-only grid is `Grid::new(lines, cols, 0)` — the same constructor the alt screen uses. No new grid mode needs writing; the byte ring replaces history outside the grid.

### Public API we depend on

`Term::new(config, &dyn Dimensions, event_proxy)` `:410`; `Term::resize` `:655`; `grid()/grid_mut()` `:645/650`; `renderable_content()` `:637` → `RenderableContent` `:2393`; `damage()/reset_damage()` `:458/489`; `mode()` `:709`; `swap_alt` `:714`; `colors()` `:952`; `cursor_style()` `:942`; `impl Handler for Term` `:1059` (71 `Handler` methods in vte `ansi.rs:495-`, counted). Feed bytes with `vte::ansi::Processor::advance(&mut term, bytes)` (`event_loop.rs:404` pattern). `Term<T: EventListener>` (`event.rs:103`) receives `Event::{Title, ClipboardStore, ClipboardLoad, ColorRequest, PtyWrite, TextAreaSizeRequest, Bell, Exit, ChildExit, Wakeup, …}` (`event.rs:14-58`) — `PtyWrite` is how responses (DA, DSR, kitty report, OSC 52 paste) get back to the PTY; our mux implements this listener.

### Fork surface — keep / cut / change / add (counted lines)

| Bucket | Files | Lines | Note |
|---|---|---|---|
| **Keep** | grid/* (2,479), term/mod.rs (3,302), term/cell.rs (331), term/color.rs (60), index.rs (467), event.rs (110), sync.rs (49) | **6,798** (≈1,450 of them tests) | the model |
| **Cut** | tty/* (1,601; `portable-pty` replaces), event_loop.rs (486; the mux owns the read loop and the ring), thread.rs (11), vi_mode.rs (893), term/search.rs (1,251), selection.rs (668) | **4,910** | selection and vi are GUI concerns on the GUI's line cache under the daemon-VTE split; search runs over the byte ring text, not the grid |
| **Change** | term/mod.rs: remove `selection` field (64 refs, counted), `vi_mode_cursor` (49 refs), search hooks (1 ref); drop `Config.vi_mode_cursor_style`; add the `unhandled_osc` handler | ≈300–500 | the 201 `damage` refs stay |
| **Add** | checkpoint export/import (see below); OSC 7501/7502 pass-through; `Handler` hooks for seat-state (OSC 133 later) | ≈600–1,000 | `Grid`/`Row`/`Storage`/`Cell` already derive serde; `Term` does not |

Estimate: **≈6.8k kept, ≈4.9k cut, ≈0.4k changed, ≈0.8k added** → fork ≈8k lines against 11.7k upstream. Upstream fixes to `grid/` and `term/mod.rs` remain cherry-pickable while the file layout is preserved.

**Checkpoint export/import — what must be serialised** (`term/mod.rs:268-331` fields): `grid`, `inactive_grid` (serde-capable, but `Grid.cursor`/`saved_cursor` are `serde(skip)` at `grid/mod.rs:112,116` — must be un-skipped in the fork), `active_charset`, `tabs: TabStops`, `mode: TermMode` (bitflags, serde via feature), `scroll_region`, `colors`, `cursor_style`, `title`, `title_stack`, `keyboard_mode_stack`, `inactive_keyboard_mode_stack`, `config`. Plus **parser state**, which lives in `vte`, not in `alacritty_terminal`: `vte/src/lib.rs:55-70` `Parser { state, intermediates, intermediate_idx, params, param, osc_raw, osc_params, osc_num_params, ignoring, partial_utf8, partial_utf8_len }` — all private; `vte/src/ansi.rs:271-273` `Processor { state: ProcessorState, parser }` private, `ansi.rs:253-259` `SyncState { timeout, buffer }` private. The spec requirement "parser state including incomplete sequences" ([[mux-daemon]] Ring records) therefore **needs the `vte` state machine forked too** (4,144 lines, counted: `lib.rs` 1,542 + `ansi.rs` 2,458 + `params.rs` 144; `Apache-2.0 OR MIT`). Alternative without forking: checkpoint only when the parser is in Ground with no partial UTF-8 and no pending 2026 buffer — but `Parser` exposes no state accessor, so even that needs a fork. Decision in [[terminal-core]].

### Protocol feature coverage — where each lives

| Feature | vte 0.15.0 | alacritty_terminal HEAD | Who else must act |
|---|---|---|---|
| 24-bit colour | SGR 38/48;2 parsing in `ansi.rs` `Attr` | `Term::set_color` `:1662`, `Colors` | renderer |
| OSC 8 hyperlinks | `ansi.rs:1393` arm `b"8"` → `Handler::set_hyperlink` | `:1874` `set_hyperlink`, `Cell.extra: Option<Arc<CellExtra>>` | GUI: hover/open |
| OSC 52 clipboard | `ansi.rs:1483` arm `b"52"` | `:1705` `clipboard_store`, `:1726` `clipboard_load`; gated by `Config.osc52: Osc52 {Disabled, OnlyCopy (default), …}` `:372-378` | mux → GUI clipboard; policy per seat |
| Bracketed paste 2004 | `NamedPrivateMode::BracketedPaste = 2004` `ansi.rs:966` | `TermMode::BRACKETED_PASTE` `:61`; set `:1970`, reset `:2029`, DECRQM `:2082` | writer wraps paste in `ESC[200~ … ESC[201~` when mode set (alacritty does it in `alacritty/src/input/mod.rs:140` `paste`) |
| Mode 2026 synchronized output | `ansi.rs:36` `SYNC_UPDATE_TIMEOUT = 150 ms`, `:45-48` exact `ESC[?2026h/l` match, `Processor::advance` `:298-312` buffers while pending, `:370` `advance_sync`, `:967` `SyncUpdate = 2026`, `Timeout` trait `:478`, `StdSyncHandler` `:443` | nothing to do | **mux must drive the timeout** (alacritty's event loop polls `sync_timeout()`; our per-seat scheduler needs a 150 ms timer) |
| Kitty keyboard protocol | `ansi.rs:739` `KeyboardModes` bitflags, CSI `u` dispatch `:1748,1758` | `:1275` report, `:1288` push, `:1308` pop, `:1323` set, stacks `:320-323`; gated by `Config.kitty_keyboard` `:339`; stack cap fix in `5421470` | **key encoding is in the app**, `alacritty/src/input/keyboard.rs:295` `build_sequence(key, mods, mode)` (718-line file) — GUI must port it |
| OSC 7501/7502 (Claude subagent markers, [[subagent-detection]]) | **not handled**: `ansi.rs:1523` `_ => unhandled(params)` → `debug!` only; `Performer` is private (`:425`) so no wrapper can intercept | — | fork `ansi.rs`: add arm + `Handler::unhandled_osc(&[&[u8]])` |
| OSC 133 semantic prompts (future seat-state) | not handled | — | same fork point; rio-vt has it (`rio-vt/src/performer/handler.rs:1182`) |
| Images (deferred) | none | none | rio-vt has sixel + kitty graphics (`rio-vt/src/ansi/kitty_graphics_protocol.rs` 2,376 lines) |

### The alacritty renderer — entry points (for question 3)

App crate: **20,944 lines** (counted); renderer + display = **8,594** (`renderer/` 3,789 incl. 1,032 of builtin box-drawing font; `display/` 4,805) + **407 lines GLSL** (`res/glsl3/text.{v,f}.glsl` 77+73, `res/gles2/` 46+57, `res/rect.{v,f}.glsl` 16+138).

- GL context: `display/mod.rs:11-16` glutin imports, `:436` `gl_context.make_current(&surface)`, `:439` `Renderer::new(&context, …)`, `:481` `swap_buffers`, `:556-563` `make_current`. `renderer/mod.rs:127` `gl::load_with`, `:133` shader-version sniff, `:147-160` picks `Glsl3Renderer` or `Gles2Renderer` (`TextRendererProvider` `:84-85`).
- Cell instancing: `renderer/text/glsl3.rs:284-300` `#[repr(C)] struct InstanceData { col: u16, row: u16, left/top/width/height: i16, uv_left/uv_bot/uv_width/uv_height: f32, fg + flags, bg }`; `:27` `BATCH_MAX = 0x1_0000` instances; `:81` one VBO of `BATCH_MAX * size_of::<InstanceData>()`; `:100` `VertexAttribDivisor(index, 1)`; `:243,251` `DrawElementsInstanced`; `:321` `Batch { tex, instances }`. Vertex shader `res/glsl3/text.v.glsl:1-16` locations 0–4, `:26,55` `renderingPass` uniform — pass 0 draws cell backgrounds, pass 1 glyphs, same instance buffer; `:36-37` corner from `gl_VertexID`.
- Dual-source blending (subpixel): `res/glsl3/text.f.glsl:27-28` `layout(location=0,index=0) color; index=1 alphaMask`; GLES2 path gated at `renderer/text/gles2.rs:40-51` `allow_dsb && GL_EXT_blend_func_extended`.
- Atlas: `renderer/text/atlas.rs:12` `ATLAS_SIZE = 1024`, `:33` `Atlas` (row packer: `:225` `room_in_row`, `:234` `advance_row`), `:119/147` `insert`, `:251-268` `load_glyph` opens a new 1024² GL texture when full; GL texture ids leak into the API (`active_tex: &mut u32`).
- Glyph rasterization: `renderer/text/glyph_cache.rs:4` `use crossfont::{…}`, `:46` `GlyphCache`, `:82` `new(Rasterizer, &Font)`, `:200` `get(glyph_key, loader)`, `:127` `compute_font_keys` (regular/bold/italic/bold-italic only — no per-codepoint fallback list, crossfont does fallback internally on macOS). `crossfont` 0.8.1 pinned (`alacritty/Cargo.toml:30`); backends `src/{darwin,directwrite,ft}` (API: repo contents) — CoreText on macOS.
- Rect pass (underlines, cursors): `renderer/rects.rs:46-50` `RectKind {Normal, Undercurl, DottedUnderline, DashedUnderline}`, `:248` `RectRenderer`; `res/rect.f.glsl` 138 lines does undercurl/dots/dashes procedurally.
- What the renderer consumes: `display/content.rs:189-198` `RenderableCell { character, point, fg, bg, bg_alpha, underline, flags, extra }`; damage: `display/damage.rs` (387 lines) `DamageTracker`, `display/mod.rs:509` constructed per size, `Term::damage()` feeds it.
- Window: `display/window.rs:348-355` macOS `Decorations::Transparent/Buttonless` = `with_title_hidden(true).with_titlebar_transparent(true).with_fullsize_content_view(true)` on winit 0.30.9.

GL dependence is total: every struct in `renderer/` holds GL object ids and every draw is a raw `gl::*` call; glutin owns the context; uniforms are looked up by name (`glsl3.rs:431`). A "port to wgpu" replaces every line of `renderer/` and the GL half of `display/` (≈4.5k lines) and the 407 GLSL lines become WGSL. What survives a port is the **design**: 32-byte cell instance, two-pass bg/text draw over one instance buffer, 64k-instance batch cap, row-damage tracking, `RectKind` procedural underlines, `RenderableCell` as the model→renderer contract. Those are the same shapes [[text-rendering]] §5 lifts from ghostty with min-contrast, linear-corrected blend and two atlases added; alacritty's version is the strict subset.

---

## 2. vte (alacritty/vte)

**License** `Apache-2.0 OR MIT` (`Cargo.toml:41`). crates.io **0.15.0, 2025-02-02** (0.14.x Jan 2025, 0.13.0 2023-11); repo pushed 2026-02-28. 4,144 lines (counted). Features: `ansi` (the `Processor`/`Handler` layer alacritty uses), `std`, `serde`, `no_std` with `ArrayVec` OSC buffer.

- State machine: `src/lib.rs:55-70` `Parser` (fields above), `:109` `advance`, `:140` `advance_until_terminated`, `:761` `trait Perform` (raw actions). Table-driven, UTF-8 inlined (`partial_utf8`).
- ANSI layer: `src/ansi.rs:271` `Processor`, `:298` `advance` (sync-aware), `:425` private `Performer<'a, H, T>` implements `Perform` `:1284`; `:495` `trait Handler` (71 methods, counted); OSC arms `:1350` 0/2 title, `:1366` 4 palette, `:1393` 8 hyperlink, `:1422` 10/11/12 dynamic colours, `:1456` 22 cursor shape, `:1465` 50 font (ignored), `:1483` 52 clipboard, `:1496` 104, `:1515-1521` 110/111/112, `:1523` default → debug log. Private modes enum `:950-968`.
- Used by: alacritty_terminal (above), zellij (`zellij-server/src/panes/grid.rs:5027` `impl Perform for Grid`, raw `Perform` not the `ansi` layer; zellij 0.45.1 2026-08-28 MIT), rio-vt (forked copy, `rio-vt/src/performer/parser/mod.rs:7` "Forked from Alacritty's VTE; previously the standalone `copa` crate").

---

## 3. portable-pty (wezterm/wezterm `pty/`)

**License** `MIT` (`Cargo.toml:26`). crates.io **0.9.0, 2025-02-11**. In-tree at wezterm HEAD `e2a98e1` (2026-10-10) still `0.9.0`; only `cmdbuilder.rs` differs (839 vs 823 lines, counted) and Windows files — no unreleased unix changes. 2,462 lines (counted): `cmdbuilder.rs` 823, `lib.rs` 407, `unix.rs` 414, `serial.rs` 305, `win/*` 513. Deps: `nix 0.28`, `libc`, `filedescriptor 0.8.3`, `shell-words`, `serial2`, `anyhow`, `downcast-rs`; feature `serde_support`.

API (`src/lib.rs`): `:63` `PtySize { rows, cols, pixel_width, pixel_height }`; `:88` `trait MasterPty: Downcast + Send { resize :92, get_size :94, try_clone_reader :97 → Box<dyn Read + Send>, take_writer :102 → Box<dyn Write + Send> (one-shot), process_group_leader :107, as_raw_fd :114, tty_name :117, get_termios :122 }`; `:130` `trait Child { try_wait, wait, process_id }`; `:150` `ChildKiller { kill, clone_killer }`; `:163` `trait SlavePty { spawn_command(CommandBuilder) :165 }`; `:253` `PtyPair { master, slave }`; `:263` `trait PtySystem { openpty(PtySize) -> PtyPair :267 }`; `:400` `native_pty_system()`. `cmdbuilder.rs:203` `CommandBuilder`: `new :215`, `new_default_prog :254`, `arg/args :272/280`, `env/env_remove/env_clear :299-324`, `cwd :342`, `set_controlling_tty :244`, `get_shell :544`.

Unix path (`src/unix.rs`): `:22` `fn openpty(size)` → `:36` **`libc::openpty(&mut master, &mut slave, null, null, &size)`** (not `posix_openpt`, not rustix); `:71` `impl PtySystem for UnixPtySystem`; `:152` `close_random_fds`; `:180-197` `resize` → `ioctl(TIOCSWINSZ)`; `:228-276` `spawn_command`: `pre_exec` does `setsid()` `:257`, `ioctl(0, TIOCSCTTY)` `:271`, `close_random_fds()` `:276`; `:352` `try_clone_reader` dups the fd; `:357` `take_writer`.

**ssh backend is not in this crate.** It is `wezterm-ssh/src/pty.rs:26` `SshPty`, `:44` `impl portable_pty::MasterPty for SshPty`, `:93` `SshChildProcess`, `:119` `impl portable_pty::Child`, `:229` `channel.request_pty(&newpty)` over libssh2/libssh. Our remote seats run a remote supervisor ([[mux-daemon]]), so we do not need it.

Known issues: 25 open wezterm issues mention "portable-pty" (API search, no dedicated label; not triaged here). Design notes from source: reader is a blocking `Read` on a dup'd fd — at 1000 seats use `as_raw_fd()` (`lib.rs:114`) with our own poller (`polling`/mio/tokio `AsyncFd`) rather than a thread per seat; `take_writer` can be called once per master.

---

## 4. wezterm — mux and codec, reference only

**License** MIT (`LICENSE.md`; GitHub reports `NOASSERTION`). Last tagged release **20240203-110809** (2024-02-03); nightlies since; repo pushed 2026-10-19; 1,927 open issues (API). `termwiz` 0.24.0 in tree (crates.io 0.23.3, 2025-03-20), `wezterm-term` 0.1.0 unpublished, `vtparse` 0.7.0 MIT.

Model types (`mux/src/`): `lib.rs:102` `Mux { tabs: RwLock<HashMap<TabId, Arc<Tab>>>, panes: HashMap<PaneId, Arc<dyn Pane>>, windows: HashMap<WindowId, Window>, default_domain, domains, domains_by_name, subscribers: HashMap<usize, Box<dyn Fn(MuxNotification) -> bool>>, clients, identity, num_panes_by_workspace, … }`. `pane.rs:25` `PaneId = usize`; `:172` `trait Pane` (36 methods in `:172-300`, counted): `get_changed_since :188`, `get_lines(Range<StableRowIndex>) :205`, `with_lines_mut :207`, `get_logical_lines :215`, `get_dimensions :237`, `writer :249`, `resize :250`, `key_down :254`. `tab.rs:17` `Tree = bintree::Tree<Arc<dyn Pane>, SplitDirectionAndSize>`, `:21` `TabId`, `:52` `Tab`, `:58` `PositionedPane`, `:96` `SplitDirection`, `:110` `SplitSize`, `:122` `SplitRequest`, `:193` `PositionedSplit`, `:564` `set_zoomed`, `:576` `iter_panes`, `:740` `split_and_insert`. `window.rs:7` `WindowId`, `:9-11` `Window { tabs: Vec<Arc<Tab>> }`. `domain.rs:28` `DomainId`, `:31` `DomainState`, `:50` `trait Domain` (`split_pane :74`, `spawn_pane :144`), `:202` `LocalDomain`. `localpane.rs:124-128` `LocalPane { pane_id, terminal: Mutex<Terminal>, process, pty: Mutex<Box<dyn MasterPty>> }`.

Mapping to our vocabulary ([[vision]]): wezterm `Window` ≈ our **space**, `Tab` ≈ **tab**, `Pane` ≈ **seat**, `Domain` ≈ where a seat's PTY lives (local supervisor / remote supervisor). Binary split tree is the same shape as our split tree.

Codec (`codec/src/lib.rs`): `:93-97` frame = leb128 `tagged_len` (msb = zstd), leb128 `serial`, leb128 `ident`, bincode body; `:445` `CODEC_VERSION = 46`; **53 PDU idents** (counted) `ErrorResponse 0 … AdjustPaneSize 62` including `ListPanes 3, SpawnV2 36, WriteToPane 9, SendKeyDown 11, SendPaste 13, Resize 14, SetClipboard 20, GetLines 22/23, GetPaneRenderChanges 24/25, GetTlsCreds 28, SearchScrollbackRequest 31, SetPaneZoomed 33, SplitPane 34, KillPane 35, SetClientId 40, GetClientList 41, SetFocusedPane 45, MovePaneToNewTab 48, ActivatePaneDirection 50, PaneFocused 53, TabResized 54, TabAddedToWindow 55, TabTitleChanged 56, EraseScrollbackRequest 59`. Structs: `ListPanesResponse :647`, `SpawnV2 :679`, `WriteToPane :708`, `Resize :854`, `GetPaneRenderChangesResponse :915`, `GetLines :933`, `SerializedLines :976`. Server dirty-line flow: `wezterm-mux-server-impl/src/sessionhandler.rs:89-93` old/new `seqno`, `:108-137` `bonus_lines`. Client side and ssh bootstrap are already in [[cloud-and-mosh]] §2.

Parser and GUI (for the acceptance pointer): `term/src/terminal.rs:85` `Terminal`, `:164` `advance_bytes`; `terminalstate/mod.rs:255` `TerminalState`; `terminalstate/performer.rs:31` `Performer` implements `vtparse::VTActor`; `wezterm-escape-parser/src/parser/mod.rs:67` `Parser`, `:81` `VTParser::new()` (`vtparse/src/lib.rs:363`). GUI: `wezterm-gui/src/termwindow/webgpu.rs` (wgpu path), `quad.rs:33` `Vertex`, `:62` `VertexStepMode::Vertex` — wezterm draws quads, not instanced cells. Text path critique is in [[text-rendering]] "wezterm, for contrast".

---

## 5. rio / sugarloaf / rio-vt — second wgpu reference

**License** MIT (`LICENSE`, workspace `Cargo.toml:26`). **v0.5.28, 2026-09-17** (`sugarloaf`, `rio-backend`, `rio-vt` all 0.5.28 on crates.io the same day); repo pushed 2026-10-18; 339 open issues. HEAD `59203cc` 2026-10-03.

**Finding: sugarloaf no longer uses wgpu by default on macOS or Linux.** `sugarloaf/Cargo.toml:40-45`: "`wgpu` and the `librashader-*` filter chain are gated behind the `wgpu` feature. Default-off on macOS + Linux (the native Metal / Vulkan backends are preferred and skip the wgpu translation layer). Windows + WASM enable it"; features block confirms `wgpu = ["dep:wgpu", …librashader]`. Native backends: `sugarloaf/src/grid/metal.rs` (918 lines, `metal` crate), `grid/vulkan.rs` (1,778), `renderer/vulkan.rs`, `context/vulkan.rs`; wgpu backend `grid/webgpu.rs` (841), `context/webgpu.rs` (424). sugarloaf total 35,662 lines (counted).

- Cell instancing (wgpu backend): `sugarloaf/src/grid/webgpu.rs:807-811` `VertexBufferLayout { array_stride: size_of::<CellText>(), step_mode: VertexStepMode::Instance, attributes }`, attributes up to `@location(6)` with `Uint8` packed flags at offsets 28–29 (`:795-805`) — a 32-byte-class instance like alacritty's. Also `text.rs:1498` and `renderer/mod.rs:2887,3007` (rich-text and quad pipelines, instanced).
- Atlas: `sugarloaf/src/renderer/image_cache/cache.rs:38-54` `Atlas { alloc: AtlasAllocator /* guillotiere 0.6.2 */ }`, `:63` `SIZE: u16 = 4096`, `:65` `ImageCache`, `:283` `allocate(AddImage)`. One 4096² guillotine-packed cache for glyphs and images.
- Font stack (`sugarloaf/Cargo.toml`): `swash 0.2.10`, `skrifa 0.37.0`, `core-text 21.0.0`, `font-kit 0.14.3`; `font/macos.rs` 1,838 lines, `font/mod.rs` 2,845, `font/metrics.rs` 799; sprites `sprite/box_drawing.rs` 622, `sprite/branch.rs` 517.
- Versus ghostty per [[text-rendering]]: one RGBA guillotiere atlas instead of two typed atlases (R8 mask + BGRA colour) grown ×2; `font-kit` as a thick discovery layer; no min-contrast or linear-corrected blend found in the WGSL (`text_shader.wgsl` not inspected line by line — **Gap**); its best path on macOS is now hand-written Metal, so as a *wgpu* reference it is the Windows/WASM fallback, not the primary. **Gap:** whether macOS raster goes through CoreText (`font/macos.rs` suggests discovery at least) or swash — not verified.
- `rio-vt` 0.5.28 (MIT): "Embeddable terminal core extracted from Rio: VT state machine, grid, PTY driver, selection, search" (`rio-vt/Cargo.toml:9`), **39,238 lines** (counted); `crosswords/mod.rs:13` "originally taken from alacritty_terminal/src/term/mod.rs" (10,533 lines now); parser forked from vte (`performer/parser/mod.rs:7`, `handler.rs:587` `Processor`); has 2026 sync (`handler.rs:39-52,503`), kitty keyboard (`handler.rs:422`), OSC 133/777/1337/9 (`handler.rs:1163-1260`), sixel and kitty graphics (`ansi/kitty_graphics_protocol.rs` 2,376, `ansi/graphics.rs` 2,009). PTY via `teletypewriter` using `forkpty` (`teletypewriter/src/unix/mod.rs:41,796`). It is the alternative fork base: 3.3× the code, images we defer, single maintainer (Raphael Amorim). See [[terminal-core]] for why it lost.

---

## 6. wgpu + winit on macOS

**wgpu** `MIT OR Apache-2.0` (`Cargo.toml:77`); crates.io **30.0.1, 2026-08-22**; trunk `8e4718d` 2026-10-21, `rust-version 1.95`; 1,312 open issues (API). Metal backend `wgpu-hal/src/metal/` **8,596 lines** (counted: adapter 1,876, command 1,988, device 2,265, mod 1,427, surface 432, conv 522). Built on `objc2-metal`/`objc2-quartz-core` (`wgpu-hal/Cargo.toml:85-86`).

- Surface from `CAMetalLayer`: `wgpu-hal/src/metal/surface.rs:36` `Surface::new(Retained<CAMetalLayer>)`, `:46` `from_layer`; `mod.rs:144` `create_surface_from_layer`, `:159` `create_surface` (raw-window-handle path inserts a layer into the NSView); public: `wgpu/src/api/surface.rs:376` `SurfaceTargetUnsafe`, `:429` `CoreAnimationLayer(*mut c_void)`; `instance.rs:188` `create_surface<'window>`. Minimum: `mod.rs:134` `available!(macos = 10.13)` — macOS 14 floor is far above it; all `MTLFeatureSet::macOS_GPUFamily1/2_*` gates (`adapter.rs:564-628`) pass on any Sonoma Mac.
- Surface formats: `adapter.rs:485-489` `Bgra8Unorm`, **`Bgra8UnormSrgb`**, `Rgba16Float` (+ `Rgb10a2Unorm` when supported `:490-495`); `:457-474` `Rgba16Float` enables EDR. Composite alpha `:521-522` `Opaque`, `PreMultiplied`. `present_modes` `:515` keyed on display-sync. The `Bgra8UnormSrgb` target [[text-rendering]] needs for linear-corrected blending is there.
- **Sharing one Device across grid and chrome passes**: `wgpu/src/api/device.rs:21-22` `#[derive(Debug, Clone)] pub struct Device` — `Device` and `Queue` are cloneable refcounted handles; `surface.rs:118` `Surface::configure(&Device, &SurfaceConfiguration)` binds any number of surfaces to one device. The pattern: one `Instance`→`Adapter`→`Device`; one `Surface` per window; one `CommandEncoder` per frame with N `RenderPass`es (cell bg, cell text, chrome quads, glow/blur post-process into offscreen textures, composite). Chrome shaders are just more pipelines on the same device; no toolkit boundary to cross, which is why "raw wgpu" closes question 4b of the task.
- Instancing: `wgpu-types/src/vertex.rs:75` `VertexStepMode::{Vertex, Instance}`; `wgpu/src/api/render_pass.rs:246` `draw_indexed(indices, base_vertex, instances: Range<u32>)`. WGSL via naga (`naga/src/front/wgsl/`), vertex-index corner trick (`@builtin(vertex_index)`) works as in `text.v.glsl:36-37`.

**winit** `Apache-2.0` (`Cargo.toml:8`); stable **0.30.13** (GitHub release 2026-03-02); crates.io top non-yanked is **0.31.0-beta.3** (2026-09-04), trunk `d559b05` 2026-10-21 reorganised into `winit-core`/`winit-appkit`/…; 721 open issues. Borderless custom title bar (0.31 tree, `winit-appkit/src/lib.rs`): `:425` `with_titlebar_transparent`, `:432` `with_titlebar_hidden`, `:439` `with_titlebar_buttons_hidden`, `:446` `with_title_hidden`, `:453` `with_fullsize_content_view`, `:407` `with_movable_by_window_background`, `:465` `with_has_shadow`; `winit-core/src/window.rs:366` `with_decorations`. Same methods exist on 0.30 (`WindowAttributesExtMacOS`), as alacritty uses them (`display/window.rs:348-355`). **Gap:** repositioning the traffic lights into our sidebar header is not a winit API; needs `objc2-app-kit` `NSWindow.standardWindowButton(_:)` frame edits (INFERRED from AppKit, not read in winit). Pick 0.30.13 for v0.1 unless the 0.31 beta lands stable first; wgpu 30 supports `rwh_06` which both expose.

---

## 7. Excluded or reference-only candidates — license and last release (API, 2026-10-21)

| Candidate | License | Last release | Why excluded / what to take |
|---|---|---|---|
| ghostty | MIT | v1.3.1 tag; repo pushed 2026-10-21; macOS min 13.0 (`src/build/Config.zig:795-798`) | Zig. libghostty embedding ABI `include/ghostty.h` (1,287 lines: `ghostty_app_new :1162`, `ghostty_surface_new :1178`, `ghostty_surface_draw :1188`) is the only non-text thing worth reading: a C surface API with the host owning the window. Text pipeline taken per [[text-rendering]]. |
| warp | closed | — | not inspectable; public architecture only (Rust, own UI framework). Nothing to lift. |
| zellij | MIT | 0.45.1 2026-08-28 | raw `vte::Perform` on its own `Grid` (`zellij-server/src/panes/grid.rs:5027`), WASM plugins; TUI mux, no GPU. Reference for server/client split already in [[cloud-and-mosh]]. |
| gpui (Zed) | Apache-2.0 | 0.2.2 2025-10-22; Zed v1.24.4 2026-10-20 | GUI toolkit; operator decided raw wgpu. Its Metal text system is contrasted in [[text-rendering]]. |
| iced | MIT | 0.14.0 2025-12-07 | GUI toolkit; excluded by decision. Has custom `Shader` widget sharing the wgpu device — the fallback if raw UI stalls. |
| egui | MIT OR Apache-2.0 | 0.37.0 2026-10-15 | immediate-mode; paint callbacks share device; excluded by decision. |
| glyphon | MIT/Apache/Zlib | 0.12.0 2026-07-09 | cosmic-text + etagere + wgpu; wrong shape for a grid ([[text-rendering]] table). Chrome paragraphs only, later. |
| cosmic-text | MIT OR Apache-2.0 | 0.19.0 2026-04-22 | paragraph layout; not for the grid. |
| termwiz | MIT | 0.23.3 2025-03-20 | wezterm's TUI/escape toolkit; parser now in `wezterm-escape-parser` (unpublished). Not needed. |
| glutin | Apache-2.0 | 0.32.3 2025-04-30 | GL context layer alacritty uses; no Metal; irrelevant under wgpu. |
| crossfont | Apache-2.0 | 0.9.0 2025-06-09 | alacritty's rasterizer wrapper (CoreText/FreeType/DirectWrite). Superseded by the direct `objc2-core-text` + `swash` plan in [[text-rendering]]. |

---

## Verification

```
ls .bboxy/research/gpu-terminals-rust.md .bboxy/specs/terminal-core.md
grep -c 'License' .bboxy/research/gpu-terminals-rust.md
```

Commands run: `git clone` (alacritty `--shallow-since=2025-04-01`; rio, winit, wgpu `--depth 1`) exit 0; `git log --since=2025-10-21 --oneline | wc -l` → 37; `gh api` for releases, repo metadata and issue search; `curl crates.io/api/v1/crates/<name>` for versions/licenses; `wc -l`, `grep -n` on the clones and registry sources as cited. Not checked: portable-pty issue triage, sugarloaf WGSL internals, winit 0.30 vs 0.31 API drift beyond the macOS attributes, rio-vt test coverage.

[[vision]] [[text-rendering]] [[cloud-and-mosh]] [[monorepo-weavie]] [[mux-daemon]]
