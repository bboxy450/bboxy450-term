# W3.2b — bboxybook: our storybook on the native wgpu renderer

Owner: Vesper Greywind <ORACLE>. Assignee: free Executor, after W3.2a (needs `bbt-ui-model` types and layout). Status: OPEN. Spec: [[gui-components]] §3 Draw list, §6 Lab, §7 steps 4–7 and 11; [[renderer]] §4.5 showcase parity. Operator: "our own storybook, call it bboxybook; take showcase.html and its components and build previews in our native wgpu component system; make sure design translates from the mockup."

Crate `crates/bboxybook` (renamed from bbt-lab). Allowed: `crates/bboxybook/**`, `crates/bbt-render/src/chrome/**`, `crates/bbt-ui-model/src/draw/**`, `Cargo.lock`, `.bboxy/runs/w3-2b-*.log`.

1. Draw-list types and atoms in ui-model (`draw/{prim,ctx,list}.rs`): rounded rect with per-corner radii, hairline, gloss band, text run with style scale, icon tile, badge, state glyph, role mark, where mark, swatch, pill, chevron; the dim colour transform.
2. Chrome pass in bbt-render (`chrome/{mod,pipeline,icons,text}.rs`, `chrome.wgsl`): SDF rounded rect with stroke, inner highlight and soft shadow; hairline; gloss gradient; text runs through the existing atlas; renders a draw list headless.
3. Sidebar modules: seat row, tab row, mini-grid, band, bumper, swarm card, new-seat row, closed strip, stream dock, sidebar.
4. Stage modules: pane, strip, close dot, mute toggle, divider, title bar, window; scene 3 and scene 15 chrome with blank pane bodies.
5. **bboxybook** binary: a windowed gallery (winit, our own title bar) with a left list of components, a state strip across the top, density and theme toggles, the component on a checkerboard; and a headless mode `bboxybook render --all` that writes `target/bboxybook/<component>/<state>-<density>-<theme>.png` for every component × state × density × theme, plus `target/bboxybook/parity/index.html` putting each PNG beside the matching web lab screenshot (captured with a local Chromium when available, else the committed reference shots).
6. Tests: every component renders headless without error; the gallery's component list equals the design system's built elements and modules (parse `design/bboxy-design-system.json`, fail on any built element with no gallery entry).

Verify: `cargo test -p bboxybook --locked` green; `cargo run -p bboxybook -- render --all` writes the matrix; one parity page for the operator. Report with the parity index path and the list of components whose render does not yet match the mockup.
