# bbt-ui

L3 currently owns serialisable `Style { tokens: Preset, overrides: Overrides }`.
Token references stay symbolic in JSON and resolve against the compile-time
`bbt-ui-model::TOKENS` / `STYLE`, a theme, density and seat/swarm colours.
It has no renderer dependency, component, layout, input or window ownership.

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
L2 glow/frosted tuning gaps remain open; the unchanged pixel tolerance ranks
review, not acceptance. No FPS target or operator browser is involved.
The web's 1.35 text line-height and .22em uppercase tracking remain explicitly
cited Style geometry until the design sources name tokens for them.
