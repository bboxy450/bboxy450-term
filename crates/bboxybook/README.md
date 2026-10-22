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

Comparison uses `bbt_render::chrome::diff` with tolerance 8 per RGBA channel.
`parity::CasePolicy` supplies each case's gate: native text requires every rect
and Text baseline within 0.5 px plus a 2% changed-pixel budget; pixel-only cases
keep 0.1%. The report prints the policy, geometry and status for every case and
writes the same fields in `report.json`. Missing/incomplete geometry is unproven,
never a pass under a looser pixel threshold. Review remains manual; the loop
never updates a golden. Read the
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
  Group opacity/saturation now composite isolated L1 layers in CSS sRGB;
  CoreText/Chrome glyph-mask and edge-antialiasing differences remain.
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

## L6 controls

```sh
cargo run -p bboxybook --locked -- widgets
```

This writes `target/bboxybook/widgets/index.html`: Button, List, Table and Input
from `components.html`, each across four densities and two themes. Capture
records public constructor inputs and diagnostic rectangles/baselines; the
native side consumes those inputs and the external host origin/width only.
Anonymous Button labels use an inline baseline marker. Input's otherwise
opaque UA text baseline is measured by an invisible probe with the control's
computed font and padding, identified as `input-css-font-probe` in the fixture.
The renderer never consumes these diagnostic child measurements for layout.

`target/bboxybook/widgets/gallery/index.html` contains 48 native supplements:
1000-row List and Table at top/middle/end, selection at row 500, across every
density/theme. `virtualized.json` records actual layout-node counts, full
scroll extents and offsets. GPU tests check clipping, different rows at each
offset, a fixed Table header and fill through a transparent layout border.
Primitive layout and paint are covered here; input controllers and SeatRow/
Settings module rebases remain in their later briefs.

## L6 floating surfaces

```sh
cargo run -p bboxybook --locked -- overlays
```

`target/bboxybook/overlays/index.html` pairs 16 complete Overlay/Modal primitive
cases from `components.html` with native retained compositions. It also pairs
16 **container surfaces only** from `launcher.html` and
`documents.html?state=as-modal`: module children are hidden, while the original
web context remains in `context/` and is linked from each surface triplet.
The native side computes panel width and placement from the page's declared
constraints. Unrendered module content occupies a recorded external height
slot; it is not a claim of native module layout or content parity.

`surface-layouts.json` records independent panel rectangles. Surface cases
keep the 0.1% pixel budget and declare their scope in HTML and JSON; text-bearing
primitive cases retain the measured geometry plus 2% pixel policy. The capture
fixes floating entrance animations at their final state.

`gallery/index.html` renders eight native examples with patterned backgrounds:
sharp outside the Overlay, blurred and dimmed behind it, and separate sharp
Modal foreground. `overlays --native-only --out DIR` renders just that gallery.
The GPU test checks reduced background contrast, unchanged outside pixels and
an opaque foreground that remains sharp. Keyboard/dismissal/focus behaviour,
Launcher and DocumentModal bodies, and later module rebases are untouched.
# L2 materials

`cargo run -p bboxybook --locked -- materials` writes the 32 canonical cases and
`target/bboxybook/materials/index.html`: flat, gloss, frosted, glow × density × theme, each at radii 0, 6, 12, 20.
The provider is `bbt_render::chrome::materials::Pass`; it takes the caller's
device, target and distinct sampled backdrop. Legacy module drawing is unchanged.

Gloss follows the lab band's sRGB gradient (12% white → base at 55% → 18% black)
with top highlight and bottom inset shade. Frosted runs horizontal/vertical
Gaussian passes over sRGB values in float textures, with mirror edges at the
border box, then saturation. Style owns tint. Glow follows CSS source-over
compositing on both themes and preserves destination alpha. The capture harness
fixes its clock at 450 ms and supplies the same counter-hash seed to native and
web emitters, including particle radius, life, twinkle and motion.

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
matrix at `target/bboxybook/materials-effect-only/index.html` was 28/32 over
tolerance at the L3 handoff. The conformance pass below addresses its
compositing, filtering and glow gaps.

## L3 Style

```sh
cargo run -p bboxybook --locked -- styles
```

This writes `target/bboxybook/style/index.html`, with 112 canonical pairs:
the original styles, `pane-glow`, `swarm-card`, `card-row` and `new-swarm`, each
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
maps its blur radius to half that value as Gaussian sigma. The conformance pass
below replaces the distance falloff with a blurred box mask.

This is an isolated L3 gallery, not the retained L6 Box/Text widget or a module
rebase. Legacy previews retain their v22 coverage registry. Remaining border/particle AA and text-mask differences stay visible
in the report. Full group dim/opacity behaviour is deferred to components;
L3 stores and resolves those fields. No new input, layout or focus work lands.


## L1/L2 conformance pass (W3.2d.1b)

Chrome/fx use UNORM views to blend gamma-encoded sRGB values like CSS. Text
keeps an sRGB view and luminance-corrected coverage, with a snapshot of the
preceding background for each contiguous text run. Painter order is preserved.
The window and headless targets expose both compatible views; a three-frame
window smoke checks the surface path. No performance claim is made.

`chrome/shapes.wgsl` is shared by the L1 shadow and L2 glow halo. It integrates
the Gaussian-blurred rounded box mask, with the original border-box knocked out
for an outer shadow. CSS blur radius maps to sigma = blur/2 in L1. Backdrop blur
uses sRGB values and border-box mirror edges, per the
[CSS filtering operation](https://drafts.csswg.org/filter-effects-2/#backdrop-filter-operation).
Background paint bounds snap to device pixels before corner AA; text and the
canvas particle coordinate frame retain their subpixel positions.

Review the final paired outputs at:

- `target/bboxybook/d1b-edges/materials/index.html`: 8/32 over tolerance (was 28).
- `target/bboxybook/d1b-edges/style/index.html`: 56/88 over tolerance (was 76).

Both use the unchanged tolerance 8 and 0.1% changed-pixel budget. Flat/gloss/
frosted are within; the remaining material cases are canvas/SDF particle edge
rasterisation (about 0.2%). Style's remaining text-mask/edge differences are
listed separately, not hidden by a threshold or golden update. Exact phase and
random state are fixtures; the web emitter code/design is not rewritten.
`w3-2d-1b-*` logs record each before/after phase and reference revision.


## L5 atoms (W3.2d.3)

```sh
cargo run -p bboxybook --locked -- atoms
```

`target/bboxybook/atoms/index.html` shows all nine `bbt-ui` atom painters across
four densities and two themes (72 native previews). Its linked
`parity/index.html` pairs the native Glyph and Icon with all seven states of
each current committed lab page (112 pairs), using canonical keys such as
`glyph/active-cozy-dark` and `harness-icon/codex-cozy-dark`.

The L5 comparison shares only the web's outer placement, diagnostic labels and
checker backdrop. The capture hides `.glyph` and `.hicon` before saving that
backdrop; native code paints those elements through `bbt-ui::atoms`. Activity,
grade, waiting kind and identity are data. Native atoms own their colour rules,
rings, marks and tint. CSS pulse is fixed at its initial phase. The legacy
module previews and their coverage registry are untouched.

`layouts.json` records atom geometry too. Outer rectangles are the caller-owned
slots; symbol advances, waiting-kind box/text positions and baselines derive
from the exact native draw primitives and CoreText font metrics. Browser child
rectangles and inline baselines remain diagnostics. Capture probes the main
symbol and its annotation separately without changing the reference image.

The separate native-only previews establish rendering/coverage for Hairline,
Badge, Chip, Pill, Swatch, Spinner and Checkerboard; they do not claim CSS parity
for those seven atoms. No L6 widgets or module composition are built here. The
initial L5 comparison keeps font-mask/outline AA and nearest-sampled icon
differences visible under unchanged thresholds. GPU text and asset ownership
remain in L1; d.3 does not change its shaders. The separate d.1c pass below
corrects icon sampling.


## L1 icon filtering (W3.2d.1c)

`target/bboxybook/d1c-icons/index.html` pairs the L5 matrix after Wren's
`5561530` reference correction (inferred opacity multiplies the pulse).
`before/index.html` compares the same corrected references with the d.3 native
PNGs; atom bounds and hidden-atom backdrop bytes are checked equal before reuse.

Chrome images/icons use a linear sampler, clamped inside each atlas asset.
`IconAtlas` converts straight RGBA inputs to premultiplied RGB before storage;
the shader interpolates that data and returns straight colour to source-over.
This prevents hidden RGB in transparent pixels or adjacent atlas assets from
colouring a filtered edge. Text masks and terminal cell sampling are unchanged.
The GPU regression in `tests/icon_filter.rs` checks half-blue interpolation
beside transparent red, alpha preservation on the fixture, and no green bleed
from a neighbouring asset. This pass makes no performance claim.

## L6 retained components (W3.2d.4)

`cargo run -p bboxybook --locked -- components` renders the Box/Text/Stack web
references plus a 64-image native gallery of Box materials/radii, ScrollView
offsets and wrapped/ellipsized Text. Open
`target/bboxybook/components/index.html`; the gallery and `layouts.json` are
linked there. Canonical keys are `components/<state>-<density>-<theme>`.

The capture hook records public `UI.Box/Text/Stack/Spacer` constructor inputs
and the external host width/origin. Native components consume those inputs,
compute their child layout with the shared engine and emit ordered Style/
material/draw-list operations. Child DOM rectangles and browser font metrics
are diagnostic data only; native font metrics come from CoreText. The hidden
component backdrop retains common diagnostic labels and outer chrome.

This is L6 verification, not a module rebase or app input pass. Scroll offsets
are explicit; selection is metadata. Material surfaces and ordinary draw lists retain rounded ancestor clip
chains.

L6 rendering writes `layouts.json` beside its native PNGs before comparison,
including first-line Text baselines in the same coordinate system as the rects.
The report copies that file for inspection. The L3 Style fixture likewise records box/text rects and baselines from the same
paint-frame helpers and CoreText metrics. Legacy module and glyph cases without
those snapshots remain explicitly unproven under the text policy. CSS
diagnostic labels shared in an L2 backdrop do not make a material-only case text.

## L7 TabRow and MiniGrid (W3.2d.7.2)

`cargo run -p bboxybook --locked -- tabs` writes
`target/bboxybook/tabs/index.html`: 104 current lab pairs and the same 104
legacy pairs from `ux-prototype-v2`. Keys use `tab-row/` and `mini-grid/` for
current cases; underscore ids retain the legacy registry. Each pair shares
public constructor data, parent flow bounds and grid viewport sizes. Native
composition computes all children; DOM part rectangles and baselines only
feed `layouts.json` diagnostics under the existing per-case policy.

Both old TabRow/MiniGrid hand painters delegate to these retained
compositions. Expansion/focus flags are fixture data; input controllers follow
the module rebases.

## Rounded material ancestors (W3.2d.7.0)

The material pass keeps a variable-length chain of ancestor border boxes and
per-corner radii. Flat, gloss, frosted and glow intersect every mask; a square
inner clip cannot erase an outer rounded corner. Empty masks preserve the
destination. Backdrop filtering still samples the original border-box image.

`cargo test -p bboxybook --test clipping --locked` checks both independently
excluded corners, retained Box-to-material propagation and per-surface ranges
on Metal, then writes 16 native previews (four materials × theme × 1x/2x) to
`target/bboxybook/clipping/index.html`. This gallery is a clipping check, not
a claim of new web module parity.

## L7 SwarmCard and Sidebar (W3.2d.7.3)

`cargo run -p bboxybook --locked -- swarms` writes
`target/bboxybook/swarms/index.html`: 72 current header/sidebar pairs and 64
legacy pairs. Current keys use `space-header/` and `sidebar/`; legacy keys use
`band/` and `sidebar-v2/`. `capture.json` pins each reference revision.

SwarmCard composes the accepted TabRow/SeatRow trees, roll-up, pending rows,
shadows and identity bumper. Sidebar adds its bounded ScrollView and phone
handle. Public constructor arguments, parent bounds and caller scroll offsets
are inputs. The phone fixture also supplies its external padding-box clip,
which the host forwards to every draw/material operation. DOM child
rectangles/fonts remain diagnostics only. Pane previews
are shared web context, outside this module's parity claim. The old card and
sidebar painters now delegate to the retained compositions.

The current card uses the SwarmCard Style: token radius, translucent surface
and outside hairline. Unfocused contents use CSS luminance saturation; the
bumper retains its identity colour. This filter now retains one isolated GPU group (7.4b); the layer is filtered
and source-over composited once. Text supports min-content width, literal-hyphen
wrap points and a caller-supplied line clamp for pending captions. Visibility,
focus, pending outcomes, lifecycle and scrolling remain caller data; there is
no input controller or autonomous state change in this step.

Frozen `ux-prototype-v2` module cases are historical diagnostics once the design
has moved on. Capture preserves that reference identity in the fixture; reports
label it, retain the raw pixel/geometry result, and exclude it from the current
lab gate. Current cases keep the existing pixel and geometry policies.


## Isolated opacity and dim (W3.2d.7.4b)

Retained `Paint::Group` and flat `Prim::Group` preserve isolation through both
host paths. Chrome paints each group to transparent RGBA on the same device,
then a premultiplied source-over pass applies CSS sRGB saturation and opacity
once. Nested groups keep their painter order and ancestor clips. Layout and
baseline records do not change. A Box isolates automatically for opacity/dim;
its `group` flag forces an otherwise identity layer.

Offscreen text stores gamma-premultiplied RGBA: grayscale coverage over empty
pixels, the existing linear-corrected blend over covered pixels. The ordinary
opaque-target text pass is unchanged. Glow inside an offscreen group retains
coverage for the later composite; ordinary glow still preserves destination
alpha. Pane identity glow and border remain outside its strip/term filters.

The regression uses the 1,200 accepted layer/module cases and frozen web
captures from 7.3b, L5 snapshots and 7.4. It includes historical v2 diagnostics,
which remain labelled and do not gate current lab cases. See
`target/bboxybook/group-opacity/regression/after/index.html` and its adjacent
`report.md`; `before/index.html` records the same cases before this change.

## L7 TitleBar (W3.2d.7.5)

`cargo run -p bboxybook --locked -- title-bars` writes the current five-state
matrix (40 cases) plus 24 historical v2 cases to
`target/bboxybook/title-bars/index.html`. Constructor strings, sidebar/window
state and parent bounds enter native layout. Child rectangles and baselines
are independent diagnostics, not positioning inputs. The external window clip
and blank content remain shared fixture context.

The module uses retained L6 Image/Box/Text/Stack/Icon elements. The isolated
host executes Frosted and all group filters; the old flat gallery delegates to
the same composition with backdrop blur disabled because its DrawList cannot
sample previous surfaces. It keeps the Style fill and other chrome. Input and
window-control callbacks are not implemented in this module pass.

The current matrix passes 40/40 under the existing text policy: maximum rect
error 0.45px, baselines exact, worst changed fraction 1.208%. Historical cases
keep the older geometry and are explicitly unproven/diagnostic, not a gate.
Text/icon masks and corner-edge antialiasing remain visible in the triplets;
the deferred L1 raster work is unchanged. See `.bboxy/runs/w3-2d-7-5-summary.log`.

## L7 Launcher (W3.2d.7.6)

`cargo run -p bboxybook --locked -- launchers` captures and renders 176 current
cases (14 Spotlight states and eight compact picker states, four densities,
both themes), plus 48 separately named historical v2 cases. Its output is
`target/bboxybook/launchers/index.html`. The legacy hand painter delegates to
the retained composition; its flat host omits backdrop sampling.

Constructor item trees include evaluated local child providers, aliases and
facets. Native code resolves filtering, virtual descent, breadcrumbs, cap
selection and layout independently. Shared input includes rolled name/colour,
public caption/action strings and outer bounds. DOM child rectangles/fonts
never place native content. The external overlay backdrop remains common
fixture context; L6 separately exercises that material. Input controllers and
launching remain later work.

`launchers --geometry-only --manifest CAPTURED_CASES --out DIR` writes independent
`layouts.json` without rendering pixels. Inline-flex number badges use an
actual text-range probe, because a marker becomes a separate centered flex
item. Placement-caption raw text also uses ranges, preserving the original
inline spaces. Native range diagnostics aggregate its shaped runs and native
font metrics. Missing geometry remains unproven under the existing policy.

The 7.6 result is 144/176 current cases within policy: compact picker 64/64,
Spotlight 80/112. All 176 geometry records pass (11,328 rect pairs, 6,336
baselines; maxima 0.2464px and 0.0079px). The 32 remaining pixel reviews are
Go to, both placement presets and the cap, across themes/densities; worst
2.3384%. Text/icon masks, rims and frosted edges remain visible. Pixel location
analysis suggests the deferred raster work; it does not prove every cause.
The 48 historical cases are diagnostic (24 within policy, 24 unproven geometry).
The merged review is `target/bboxybook/launchers/final/index.html`; its capture
metadata records the eight recaptured empty-result variants after lab fix
`8636a28`. No review budget or golden changed.
