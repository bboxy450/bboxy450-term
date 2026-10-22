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

Limits: word wrapping has no hyphenation; an unbreakable word can overflow.
Selection is metadata until input is assigned. Draw lists preserve rounded clip
chains, while the current material host intersects ancestor clips as rectangles.
Opacity/dim follow the existing per-colour transform, not isolated CSS group
opacity. Those material/group cases are outside the paired d.4 references.
