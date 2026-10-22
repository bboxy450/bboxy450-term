# bboxybook

Native component previews on the same wgpu chrome pass used by headless tests.
The model's draw list remains std-only. This crate owns fixtures, gallery
controls, winit and the comparison loop; it does not implement product input.

```sh
cargo run -p bboxybook --locked                         # windowed gallery
cargo run -p bboxybook --locked -- gallery --smoke       # three surface frames, exits
cargo test -p bboxybook --locked
cargo run -p bboxybook --locked -- render --all
scripts/parity.sh
```

The gallery has a component list, state strip, density and theme controls, and
a checkerboard. Arrow keys select components/states, T changes theme; the
title bar's density/theme labels are clickable. It redraws on change. It never
attaches to, opens or reloads the operator's browser.

`render --all` writes `target/bboxybook/<component>/<state>-<density>-<theme>.png`
and `target/bboxybook/parity/index.html`. Existing `target/parity` web captures
are explicitly reused when available; run `scripts/parity.sh` to refresh them.
Without that cache it captures references through local Chromium. An explicit
`--out` renders only native images, useful for an iteration:

```sh
cargo run -p bboxybook --locked -- render --all --component toolbar-icon \
  --manifest target/parity/cases.json --out target/parity/native
cargo run -p bboxybook --locked -- compare --manifest target/parity/cases.json \
  --out target/parity
```

## Parity loop and evidence

`scripts/parity.sh` builds the binary, asks its registry for the complete
matrix, captures each web reference, renders native images at the measured
reference dimensions, then compares them. Outputs:

- `target/parity/index.html`: summary and web/native/diff triplets, worst first.
- `target/parity/report.md`: every case over the review budget and image paths.
- `target/parity/cases.json`: dimensions and all component/state/density/theme
  combinations actually requested, rather than a glob of possibly stale PNGs.
- `target/parity/capture.json`: reference revisions, browser, source hashes,
  capture policy and exceptions. Missing references are explicitly unproven.

The base web pages are extracted from the exact `ux-prototype-v2` tag. Built
post-v2 entries use a separately frozen committed HEAD copy and explicit
fixtures calling the lab's exported components. Post-v2 states such as
Settings / new harness use the frozen HEAD page; capture rejects states absent
from the page's state strip. Captures use an isolated
temporary Chrome profile, a 1x device scale and paused CSS animation playback.
Original demo text/FX remain in reference shots; native pane bodies are blank.
No screenshot is substituted for native component implementation.

Comparison uses `bbt_render::chrome::diff`: tolerance 8 per RGBA channel and a
0.1% changed-pixel budget, the renderer spec's tolerance alternative to DSSIM.
The score ranks review, not acceptance, and never updates a golden. Read the
triplet before changing code. The first loop found an inverted checkerboard;
after correcting its phase and the toolbar fixture, the same light open-toggle
case improved from 99.99% changed to 0.98%. The images were inspected.

CI runs the loop when the push/PR changes the lab, native component/renderer
crates, design sources or script; it uploads `target/parity`. Visual differences
are review artifacts; capture errors and rendering failures fail the command.

## Implementation

- `bbt-ui-model/src/draw`: typed retained commands, clip validation, atoms,
  grade/kind glyphs, token colours and the dim transform. Identity strokes and
  bumpers bypass content dimming.
- `bbt-render/src/chrome`: instanced SDF rounded rectangles with per-corner
  radii, strokes, highlights, shadows, gloss, lines and clipping. Icon/image
  commands sample an RGBA atlas. CoreText shapes Unicode UI lines and puts
  grayscale masks in the existing growing glyph atlas. The same pass targets
  offscreen RGBA and the window surface on one caller-owned device.
- `build.rs`: extracts the lab's SVG dictionary and bakes 64px masks with resvg;
  the portrait is the repository's copy of the operator's identity PNG.
- `src/modules.rs`: atom/module compositions, sidebar and stage fixtures.
  Panes and mini-grids use ui-model's ratio solver and the pinned scene-15 tree.
- `src/gallery.rs`: explicit component/state registry. Its test fails when a
  built, non-deferred v22 design entry has no gallery entry. Renderer smoke tests
  exercise every registered entry; the script exercises the full matrix.

The shell loop consumes ordinary commands and produces PNG/Markdown artifacts.
Tasks can invoke it, invariants can check the supplied metric, artifacts can
be claimed, and the specified event/provenance primitives can record/wake the
next iteration. This pass adds no separate delegation or event path.

## Limits carried into review

This is the first native preview implementation, not pixel parity. The report
is the authoritative remaining-case list. In particular:

- Settings, launcher/banner, graph/provenance/subscription cards, work-item and
  artifact fixtures are representative draw compositions; their full content,
  form anatomy and all keyboard/input behaviour are not yet faithful.
- Sidebar 100/1000 demos and the phone sheet still use the bounded showcase
  fixture, not the web generator's complete seeded model. Product virtualised
  layout is tested separately in ui-model; the demo does not prove 1000 seats.
- Some standalone fixtures use different names, role marks, colours, split
  shapes or arrangement from the web page. Expanded/collapsed anatomy, labels
  and exact spacing still differ. The parity output exposes these differences.
- Pane terminal text, particle glow, ripples, frosted blur, cursor animation,
  input/focus/drag controllers and product settings persistence belong to the
  later renderer/app briefs. The glow gallery entry previews chrome only.
- CoreText line masks are grayscale, including fallback symbols; colour emoji
  is absent. Ellipsis cuts at Unicode scalar boundaries, not grapheme clusters.
  CSS filters, whole-group opacity and alpha blending can differ from the
  draw-time HSL transform and the renderer's linear blending.
- A few dimensions remain the cited CSS literals where no token exists
  (band type size, bumper, close dot, sample bounds); title-bar and band heights
  now use Wren's `STYLE.spacing` tokens. No design files were modified here.
- macOS/Metal is the implemented backend. Linux, mobile, macOS 14 and hosted CI
  were not independently exercised by this pass. No FPS target was measured.

The Oracle froze this handoff at design-system v22. The coverage guard parses
`fixtures/design-system-v22.json`, pinned with revision and digest beside it.
Later lab entries belong to W3.2c; update that pin and its registry deliberately. A preview does not imply implemented protocol state or an operational
permission; all data shown in gallery fixtures is sample data.

Strict all-target Clippy also reports pre-existing findings outside this brief:
`bbt-render/src/headless.rs:364` (`map_entry`) and
`bbt-render/tests/smoke.rs:15` (`chunks_exact_to_as_chunks`). The scoped lint
verification excludes those two diagnostics explicitly; those files were not
changed. See `.bboxy/runs/w3-2b-clippy-*.log` for actual command results.
# L2 materials

`cargo run -p bboxybook --locked -- materials` writes the 32 canonical cases and
`target/bboxybook/materials/index.html`: flat, gloss, frosted, glow × density × theme, each at radii 0, 6, 12, 20.
The provider is `bbt_render::chrome::materials::Pass`; it takes the caller's
device, target and distinct sampled backdrop. Legacy module drawing is unchanged.

Gloss follows the lab band's sRGB gradient (12% white → base at 55% → 18% black)
with top highlight and bottom inset shade. Frosted runs horizontal/vertical
Gaussian passes into linear float textures, with 3σ support, then saturation and
tint. Glow has activity/focus/clock/particle parameters, adds on dark, multiplies
on light, and preserves destination alpha. The native glow frame captures 450 ms.

Pairs use the web page `materials.html` and keys such as
`materials/gloss-cozy-dark`. The harness captures the page with its material
squares hidden to make a common backdrop texture, including diagnostic labels.
Native shaders paint only the captured square rectangles over that texture.
This isolates material pixels from the later component/layout layers. The
backdrop fixture and crop are recorded in `cases.json`; Chromium is required.

`tests/materials.rs` checks GPU pixels: exact flat colour, gloss gradient,
backdrop smoothing, dark/light glow blend, static idle/error and changing
active/waiting frames. This layer does not establish CSS pixel parity, desktop
blur support or performance. No higher component layer consumes it yet.


The L2 provider is effect-only. The revised web reference (3afef72) also
separates glow's fill/border and frosted's tint/ring into Style. The refreshed
matrix at `target/bboxybook/materials-effect-only/index.html` is 28/32 over
tolerance: edge AA/compositing, frosted colour/filtering and glow
strength/particles remain open. Their shaders were not tuned in L3.

## L3 Style

```sh
cargo run -p bboxybook --locked -- styles
```

This writes `target/bboxybook/style/index.html`, with 88 canonical pairs:
the original ten styles plus the explicit filled `pane-glow` Style, each
across four densities and two themes. Keys are `style/<state>-<density>-<theme>`.
`styles.json` records the actual serialised token references/overrides and
resolved numeric values. `bbt-ui` has no renderer dependency; it uses the
existing compile-time tokens, with late theme/density/identity resolution.

The native review fixture draws the same low-level box for every Style.
Only its external origin/width and the CSS diagnostic backdrop are shared;
height, padding, fill, border, shadow and text resolve through Style. Shadows
are destination underlays, kept out of the material's sampled backdrop.
Fill/border/text are separate foregrounds; the effect-only glow is last.
The [CSS shadow contract](https://www.w3.org/TR/css-backgrounds-3/#shadow-blur)
maps its blur radius to half that value as Gaussian sigma. The existing SDF
shadow approximation still differs from CSS's blurred box mask.

This is an isolated L3 gallery, not the retained L6 Box/Text widget or a module
rebase. Legacy previews retain their v22 coverage registry. L2 glow/frosted,
shadow-kernel, border AA, text-mask and colour/compositing gaps remain visible
in the report. Full group dim/opacity behaviour is deferred to components;
L3 stores and resolves those fields. No new input, layout or focus work lands.
