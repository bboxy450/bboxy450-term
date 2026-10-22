# W3.2d — bbt-ui: the wgpu component library

Owner: Vesper Greywind <ORACLE>. Assignee: Nyx Copperwing <EXECUTOR> after W3.2c (or a free Executor in parallel on the crate skeleton). Status: OPEN. Spec: [[gui-components]] §2b (components, Style, materials, parity rule), §2 layout, §3 draw list; [[renderer]] chrome and fx passes.

Split into per-layer briefs per [[gui-components]] §2c; each lands and is verified alone. New crate `crates/bbt-ui`. Allowed: `crates/bbt-ui/**`, `crates/bboxybook/**` (re-basing modules), `crates/bbt-render/src/chrome/**` (materials), root `Cargo.toml` member, `Cargo.lock`, `.bboxy/runs/w3-2d-*.log`.

- **W3.2d.1 (L2 materials):** `flat`, `gloss`, `frosted`, `glow` as named chrome-pass shaders with parameters; a materials gallery (one square per material × theme). Verify: gallery renders headless; gloss matches the lab band by eye.
- **W3.2d.2 (L3 Style):** `Style` = tokens + overrides, serialisable; a Style gallery (one Box, ten styles). Verify: round-trip serialisation test; gallery renders.
- **W3.2d.3 (L5 atoms):** Hairline, Glyph, Icon, Badge, Chip, Pill, Swatch, Spinner, Checkerboard; atoms gallery × density × theme. Verify: parity against the lab glyph and harness-icon pages.
- **W3.2d.4 (L6 Box, Text, Stack, ScrollView):** layout through the engine, draw through the list. Verify: layout snapshots; a Box gallery across materials and radii.
- **W3.2d.5 (L6 Button, List, Table, Input):** virtualised List and Table, header, selection band. Verify: parity against the lab's seat-row list and settings tables.
- **W3.2d.6 (L6 Overlay, Modal):** the launcher material over a blurred backdrop. Verify: parity against launcher.html and the document modal state.
- **W3.2d.7… (L7 modules, one brief each):** re-base SeatRow, then TabRow+MiniGrid, then SwarmCard+Sidebar, then Pane+Strip, then TitleBar, then Launcher, then StreamDock, then GraphTab, then Settings, then DocumentModal; parity re-run after each; delete the hand-drawn primitives as each lands.
- Guard test throughout: no module calls draw-list primitives directly.

Verify: `cargo test -p bbt-ui -p bboxybook --locked` green; `scripts/parity.sh` over-tolerance count lower than before step 4 and the ten worst named. Report by DM, one line on the channel.
