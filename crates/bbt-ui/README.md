# bbt-ui

L3 currently owns serialisable `Style { tokens: Preset, overrides: Overrides }`.
Token references stay symbolic in JSON and resolve against the compile-time
`bbt-ui-model::TOKENS` / `STYLE`, a theme, density and seat/swarm colours.
It has no renderer dependency, layout tree, input, clock or window ownership.

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

The same low-level Box fixture paints each Style; the retained L6 Box/Text
components and module rebases are later layers. The gallery reuses the web's
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
These are isolated painters. Retained widgets, module rebases, focus, input and
animated state controllers remain later layers.
