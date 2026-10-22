# Renderer — architecture and automated render testing

Owner: Vesper Greywind <ORACLE>. 2026-10-21. Status: DECIDED for v0.1, built by the renderer Executor. Sources: [[text-rendering]] (ghostty pipeline), [[terminal-core]] D3 (lift alacritty's instancing design), [[ux-prototype]] and `design/tokens.toml` + `design/style.json` (chrome), [[vision]] Stack. Operator rule: "build terminal rendering, run it through smoke and render tests automatically, render many versions."

## 1. Shape

One `wgpu::Device` per process, Metal on macOS. Everything is drawn into one surface by a fixed sequence of passes:

| Pass | Draws | Input |
|---|---|---|
| `cells.bg` | cell backgrounds, one full-screen fragment reading a rows×cols colour buffer (ghostty) | `CellBuffer` |
| `cells.text` | glyphs, one instanced draw, 32-byte instance (alacritty layout: cell col/row, glyph uv, fg, bg, flags) | `GlyphAtlas`, `InstanceBuffer` |
| `cells.deco` | underlines (straight, double, curly, dotted, dashed), strikethrough, cursor styles, procedural box drawing and powerline sprites | `DecoInstances` |
| `chrome` | rounded rects, hairlines, text runs, icon tiles, tiles from `style.json` radii/type scale | `ChromeList` |
| `fx` | border particle glow, locate ripple, unfocused dim, frosted sidebar | `FxUniforms` per on-screen seat |

All geometry is retained between frames; only changed buffers upload. Damage tracking per seat row (alacritty) decides whether a frame is drawn at all. The glow is the only continuous animator and only for on-screen seats ([[ux-pick]] budget rule).

## 1a. Blending (decided 2026-10-21 after W3.2d.2)

Chrome and fx passes blend in sRGB (gamma) space so the native UI matches the CSS reference; text keeps linear-corrected blending ([[text-rendering]]). Shadows use a Gaussian kernel with sigma = blur/2 to match CSS box-shadow.

## 1b. Window transparency (operator, 2026-10-21, "semi transparency like iTerm2; our renderer supports it")

The window is composited over the desktop with a per-window alpha and an optional background blur, like iTerm2's transparency and blur. Mechanism on macOS: a non-opaque `CAMetalLayer` (`isOpaque = false`, `wantsExtendedDynamicRangeContent` off), surface format `Bgra8UnormSrgb` with alpha, the window's `backgroundColor` clear and `isOpaque = false`, and `NSVisualEffectView` behind the layer for blur when the operator enables it (winit exposes transparency and the AppKit handle; the effect view is one AppKit call). The pane background pass writes alpha = the seat's background opacity token; chrome and text stay opaque so text contrast is never lost. Consequences the design must honour: **subpixel text is impossible over transparency**, which is already the grayscale decision in §2; the unfocused dim and the glow composite correctly over alpha because the fx pass runs last; the particle glow stays additive on dark and uses the darken path on light ([[ux-prototype]] §glow). Settings: window opacity (0.5–1.0), blur on/off and radius, per-swarm override. Tokens: `window.opacity`, `window.blur`. Linux: Wayland alpha surfaces work; blur is compositor-specific and off by default. bboxybook gets a `--transparent` flag that renders over a checkerboard so parity shots show the alpha.

The ghostty pipeline on Rust crates, per [[text-rendering]] Recommendation: CoreText discovery, rasterization and shaping on macOS (via `objc2-core-text`), grayscale alpha atlas, fractional bearing baked per glyph and placed on integer pixels, linear-corrected blending as default, no hinting on macOS, `font-thicken` option, emoji into a premultiplied BGRA atlas. Atlas grows ×2, never evicts in v0.1. Procedural sprites outrank fonts for box drawing, braille, blocks, powerline.

## 3. Crate layout

`crates/bbt-render`: `device.rs` (adapter/device/surface), `atlas.rs`, `text/` (font discovery, shaping, raster per platform), `cells/` (buffers, instancing, WGSL), `chrome/` (primitives, tokens include), `fx/` (WGSL for glow, ripple, dim), `frame.rs` (pass sequence, damage), `headless.rs` (render to texture, readback to RGBA8). No windowing in this crate; `bbt-app` owns winit. `bboxybook` and the tests use `headless.rs`.

## 4. Automated render testing (the point)

Every renderer change is proven by pictures, not prose. Three layers, all run by `cargo test -p bbt-render` and in CI on macOS.

**4.1 Smoke.** Create a device, render one frame of a 80×24 grid with the fixture text, read back, assert non-black pixels in known cells. Runs in seconds, fails on any pipeline or shader compile error.

**4.2 Terminal conformance corpora.** Feed byte streams through `bbt-core` into the renderer and compare against goldens:
- alacritty's `tests/ref/` reference dumps (grid state), ported with the fork.
- `vttest` and `esctest` sequences captured as byte fixtures under `tests/vt/`.
- our own fixtures: `lab/web/logs/*.txt`, the OSC 7501 captures from task H, SGR-across-checkpoint from Nyx's probe.
Each fixture renders to a PNG under `target/render/<fixture>/<variant>.png` and compares to `tests/golden/<fixture>/<variant>.png`.

**4.3 Variant matrix.** Every fixture renders across a matrix, not once:

| Axis | Values v0.1 |
|---|---|
| font | JetBrains Mono, SF Mono, Menlo, a fallback-heavy CJK+emoji line |
| size | 11, 13, 16 px |
| scale | 1×, 2× |
| theme | dark, light |
| seat state | idle, active, waiting, errored; focused / unfocused |
| cursor | block, bar, underline; blink on/off |
| decorations | each underline style, strikethrough |

The matrix is generated, so 4 fonts × 3 sizes × 2 scales × 2 themes × 8 states is the baseline; CI renders all of them for the five core fixtures (~1k images), and the full corpus nightly.

**4.4a Acceptance policy (decided 2026-10-21 after W3.2d.4).** Text-bearing cases are accepted on geometry: every layout rect and baseline within 0.5 px of the captured CSS metrics, plus a 2% changed-pixel budget, because CoreText and Chrome rasterise glyphs differently. Non-text cases keep the 0.1% budget. Fractional edges paint partial coverage like CSS; they are not rounded.

**4.4 Comparison.** Perceptual diff (`dssim` or a per-channel tolerance plus a 0.1% changed-pixel budget), never exact bytes: Metal and CoreText output drifts across OS versions. A failing compare writes a side-by-side `diff.png`. Goldens update only via an explicit `UPDATE_GOLDENS=1 cargo test` with the diff attached to the PR.

**4.5 Showcase parity, scripted.** Parity target is tag `ux-prototype-v2`. `scripts/parity.sh` captures the web lab headless, renders the same matrix through bboxybook, perceptual-diffs every pair, and writes `target/parity/index.html` (web | native | diff, worst first) plus `report.md` for the next iteration; it runs in CI. The improvement loop is: render, read the report, fix the worst, re-run. **Harnesses look at the images** (operator, 2026-10-21): the perceptual score ranks, but the judgement of what is wrong in a triplet is made by a harness reading the three PNGs, the Executor during the loop and the Oracle at review. A score alone never closes an iteration; an image read does. The Rust `bboxybook` renders the same scenes as the web showcase (`?scene=3`, `?scene=15`, every component page) to PNG; a parity report puts web and Rust side by side for the operator. Not a pass/fail in v0.1; a review artifact.

**4.6 Performance smoke.** Headless frame time for the 15-seat scene at 2×, logged, not asserted (no pre-emptive optimisation rule). Asserted only against a generous ceiling to catch a 10× regression.

## 5. Order of work

1. `headless.rs` + smoke test first. A renderer with no test harness is not started.
2. `cells.bg` + `cells.text` with the atlas, ASCII only, one font: smoke passes with text.
3. Conformance corpora wired, goldens generated, matrix generator.
4. Decorations, sprites, cursor.
5. Chrome primitives from tokens; lab scenes.
6. fx pass.

## Gaps

- **Gap:** which perceptual diff crate; evaluate `dssim-core` vs a hand-rolled tolerance.
- **Gap:** CI macOS runner with Metal: GitHub-hosted macOS runners do support Metal headless; verify in the first CI run.
- **Gap:** golden storage size; consider git LFS or regenerating goldens from a pinned OS in CI.

Links: [[vision]] [[terminal-core]] [[text-rendering]] [[ux-prototype]] [[plan]]
