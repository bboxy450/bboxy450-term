# W3.2d — bbt-ui: the wgpu component library

Owner: Vesper Greywind <ORACLE>. Assignee: Nyx Copperwing <EXECUTOR> after W3.2c (or a free Executor in parallel on the crate skeleton). Status: OPEN. Spec: [[gui-components]] §2b (components, Style, materials, parity rule), §2 layout, §3 draw list; [[renderer]] chrome and fx passes.

New crate `crates/bbt-ui`. Allowed: `crates/bbt-ui/**`, `crates/bboxybook/**` (re-basing modules), `crates/bbt-render/src/chrome/**` (materials), root `Cargo.toml` member, `Cargo.lock`, `.bboxy/runs/w3-2d-*.log`.

1. `Style` struct resolved from tokens + overrides, serialisable; materials `flat | gloss | frosted | glow` as named chrome-pass shaders (frosted and glow may stub to flat until the fx pass lands, with the material name retained).
2. `Box`, `Text`, `Stack`, `Hairline`, `Glyph`, `Icon`, `Badge`, `Chip`, `Pill`: build, lay out, draw; bboxybook gets a "components" gallery section that renders each across density × theme × material.
3. `Button`, `List` (virtualised), `Table` (virtualised, header, selection), `Input`, `ScrollView`, `Overlay`, `Modal`.
4. Re-base every existing bboxybook module onto the library; delete hand-drawn primitives; parity re-run after each module; the per-component diff should move whole groups of rows at once.
5. Tests: layout snapshots per component; every component renders headless on the GPU; a guard that no module calls draw-list primitives directly.

Verify: `cargo test -p bbt-ui -p bboxybook --locked` green; `scripts/parity.sh` over-tolerance count lower than before step 4 and the ten worst named. Report by DM, one line on the channel.
