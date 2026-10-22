# bbt-ui

L3 owns serialisable `Style { tokens: Preset, overrides: Overrides }`.
Token references stay symbolic in JSON and resolve against the compile-time
`bbt-ui-model::TOKENS` / `STYLE`, a theme, density and seat/swarm colours.
It has no renderer dependency, input, clock or window ownership.

The ten web presets plus `pane-glow` cover material name, independent fill and
border, per-corner radii, shadow, height, padding and text. Overrides replace a
whole field; a zero-width border, transparent fill or `Shadow::None` clears it.
Opacity and dim are explicit data. Invalid lengths, colours, alpha and unknown
Style/Overrides fields fail rather than silently falling back. L2 Glow is effect-only;
the `pane-glow` Style owns its surface fill and seat border. Modal likewise
owns its tint and ring separately from L2 Frosted.

`cargo test -p bbt-ui -p bboxybook --locked` checks round trips, late resolution
across densities/themes, override validation and native Style preview pixels.
`cargo run -p bboxybook --locked -- styles` writes the isolated L3 review at
`target/bboxybook/style/index.html` and the actual symbolic/resolved records in
`styles.json`. Canonical keys are `style/<state>-<density>-<theme>`.

The same low-level fixture paints each Style; it remains separate from the L6
Box/Text components and later module rebases. The gallery reuses the web's
outer fixture bounds and diagnostic backdrop, then draws material, fill,
border, shadow and text natively. It does not establish layout or app parity.
L1/L2 conformance is recorded in the bboxybook README; remaining raster differences stay visible under the unchanged tolerance. No FPS target or operator browser is involved.
The web's 1.35 text line-height and .22em uppercase tracking remain explicitly
cited Style geometry until the design sources name tokens for them.


## L5 atoms

`atoms` owns Hairline, Glyph, Icon, Badge, Chip, Pill, Swatch, Spinner and
Checkerboard. Each implements `Atom::paint(bounds, context, draw_list)`.
`AtomCtx` supplies the existing `DrawCtx`, an injected `TextMeasure` and device
scale. The library never selects a font or opens a GPU device. A caller supplies
spinner phase; repeated calls with the same inputs produce the same list.

Glyph uses the model's activity, grade and waiting kind as independent axes;
active uses the seat colour, other states use their tokens. Reported has no
ring, shim has an outer ring, inferred has a dashed ring at 72% opacity. Waiting
kind geometry follows `.glyph .kind` in lab.css. Icons carry a logical asset
name and current colour; the renderer reports missing assets. Hairlines stay
one physical pixel across scales. Pill shadow resolves through L3 Style.

`cargo run -p bboxybook --locked -- atoms` writes nine painters across four
densities and two themes, plus a separate glyph/harness-icon parity matrix.
These are isolated painters. Module rebases, focus, input and animated state
controllers remain later layers.

## L6 Box, Text, Stack, ScrollView

`components::Element` retains a tree. `Box` combines a resolved Style with
children, an axis, alignment and optional clipping. `Text` holds styled runs,
word wrapping, ellipsis and selectable metadata. `Stack` supplies axis/gap/
alignment; `Item::extent` selects intrinsic, fixed, fill or weighted flex size.
`ScrollView` supplies a viewport and explicit offset, clamps it to the content
range and clips its translated child. Wheel and selection controllers follow
in the input pass. There is no autonomous clock or scrolling behaviour.

The host implements `components::Measure` (width plus ascent/descent) and calls
`Element::layout(bounds, context, measure)`. Root width/origin are external;
height comes from content. Rows and columns allocate through the existing
`bbt-ui-model::layout::solve_box`. Shrunk text is remeasured for wrapping, mixed
font sizes share baselines, and line boxes use the CSS 1/64 px unit. No DOM child
rectangle enters native layout.

`components::paint(layout, context)` emits an ordered `Paint` plan: draw lists,
shadow underlays and named material surfaces. The host executes it on the L1/L2
renderer; it keeps a frosted material's own shadow out of the sampled backdrop.
Fill/border/radii stay Style-owned; no module is rebased in this layer.

`cargo run -p bboxybook --locked -- components` writes 24 paired box/text/stack
cases at `target/bboxybook/components/index.html`, diagnostic child rectangles
in `layouts.json`, and 64 native previews under `gallery/`: four Box materials
with radii 0/6/12/20, three ScrollView offsets and mixed/wrapped/ellipsized text,
each across four densities and two themes.

Limits: wrapping permits spaces and literal hyphens, with no automatic
hyphenation; an unbreakable word can overflow. `Text::max_lines` optionally
clamps content and ellipsizes the last visible line.
Selection is metadata until input is assigned. Draw lists and the material host preserve complete rounded ancestor clip
chains.
Opacity/dim follow the existing per-colour transform, not isolated CSS group
opacity. Those material/group cases are outside the paired d.4 references.

## L6 Button, List, Table, Input

`widgets` builds these controls from Box/Text/Stack/ScrollView and logical icons.
Button supplies primary/ghost/danger and idle/hover/pressed/focus/disabled
states. A pressed transform changes visual bounds while retaining its flow
slot. Input supplies empty/value/focus/invalid visuals and an error caption;
it does not own editing, validation or keyboard focus.

List and Table take caller-owned rows, selection and an optional `Viewport`.
Only rows intersecting that viewport become elements; fixed spacers preserve
the complete scroll extent. Table's header stays outside the scrolling body.
Columns have positive fractional weights and text cells. No autonomous scroll
loop or sorting model is added. An empty input retains its control height.

Style `border_sides` selects top/right/bottom/left layout borders; ordered
`rings` paint outside without consuming layout space. Transparent borders
reserve their inset while allowing the background to fill through the edge.
Inline header labels share the inherited line-height strut and baseline.

`cargo run -p bboxybook --locked -- widgets` writes 32 primitive comparisons
against `components.html` (four controls × four densities × two themes),
rect/baseline snapshots, and 48 native 1000-row previews at top/middle/end.
The paired reference's list is a four-row sample; the large-data previews and
GPU clipping/header checks independently exercise actual virtualization.
SeatRow and Settings module rebases follow in their assigned layer. Overflowing
wrapping control rows are explicitly rejected by the capture adapter until a
flow-layout pass; no DOM child bounds are used to position native controls.

## L6 Overlay and Modal

`components::Overlay` places one content subtree within an independently styled
backdrop. It centres by default, grows to its content with a 220px minimum,
and exposes alignment, insets and clipping. `overlays::Modal` composes a title,
body and trailing action row inside the shared frosted Style. Its 380px width
clamps to the available space; content reflows before height is calculated.
The host owns opening, focus, dismissal and input.

Style `width` constrains intrinsic Box width. `blur` resolves radius and
saturation independently of fill, border and shadow and passes them to the
existing frosted material. Defaults preserve the panel tokens; Overlay uses
the backdrop tokens. Blur samples previously painted content; Modal children
paint afterward, so foreground stays sharp. A shadow remains an underlay,
outside the material's sampled backdrop.

`cargo run -p bboxybook --locked -- overlays` produces 16 complete primitive
comparisons plus 16 explicitly marked surface-only comparisons from the
launcher and document-modal pages. Full module content is retained as context
only; those rebases follow in L7. The native gallery and GPU check use a sharp
striped backdrop to demonstrate blur, clipping and foreground separation.

## L7 TabRow and MiniGrid

`modules::tab_row::TabRow` composes a name, worst-first state roll-up, shortcut
and MiniGrid; expansion adds the accepted SeatRow/subagent/task/artifact
compositions. A single-seat tab reduces to that SeatRow with its shortcut.
`components::namespace` qualifies repeated subtrees for independent diagnostics.

MiniGrid composes flat clipped Boxes and optional first-word labels. It keeps
split ratios, the 3px big-root / 1px nested gap, 2px cell minima and zero
automatic minimum for clipped nested grids. Waiting/error colour overrides
focus colour. These modules own no input, clock, renderer or controller.

## L7 SwarmCard and Sidebar

SwarmCard composes the `SwarmCard`, `Band` and `CardRow` Styles, accepted
tab/seat subtrees, pending rows, shadows and a clipped identity bumper. Sidebar adds a
fixed viewport and caller-owned scroll offset. A collapsed thousand-seat card
builds only its bounded roll-up. Content keeps source order and supplied state.

`Element::Filter` applies the CSS sRGB luminance saturation matrix and opacity
to a retained subtree without affecting layout. It leaves the older dim
transform unchanged. Opacity remains per paint, with no isolated GPU group.
The host supplies all navigation, focus, lifecycle and input handling.

The 7.3b Style records match lab revision `759fdde`. Resolved `SwarmCard`
geometry supplies the inner/body padding and open/closed bumper heights;
`CardRow` supplies the external inset, row radius and padding; `NewSwarm`
supplies the action height and text. Band geometry and tracking also resolve
from its shared record. Standalone rows retain their small corners; rows in a
swarm use `CardRow`. No module reads descendant web rectangles to place content.

`atoms::KindColors` optionally sets a waiting annotation's foreground/background
without recolouring the main state glyph. `Element::Glyph` forwards those
colours; defaults preserve the existing appearance. Grade and context opacity
apply once, including on coloured surfaces with a transparent annotation base.
