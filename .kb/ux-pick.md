# UX pick — Oracle selection from the divergence field

Selected by Vesper Greywind <ORACLE> on 2026-10-21 from [[ux-divergence]] under the constraints in [[ux-sidebar]]. Items marked ASSUMED are Oracle defaults the operator has not confirmed; the operator overrides by saying so. Back to [[vision]].

| Element | Pick | From |
|---|---|---|
| Sidebar | **Pill stack** with a **two-density** toggle (comfortable / compact). Space = vertical pill in the space colour, tabs as rows inside, each tab row ends in its mini-grid and ⌘N. At scale: collapse by space then tab, rolled-up worst-first glyphs and counts, footer `N seats · M waiting`. Rows never move. | §10 shapes 1 + 8, §2 collapse |
| Split mini-grid | The tab's split tree drawn as a small grid beside the tab row, V and H shapes, focused cell brighter. Same grid becomes the phone's home. | §10 shape 1 ASCII, §12 shape 3 |
| State glyphs | Set A: `●` active, `○` idle, `◐` waiting, `✕` errored, colour-coded (seat colour, dim, amber, red). Waiting pulses at 1 Hz. Nothing else animates by default. | §4 shapes 1 + 3 |
| Seat colour | **Hash of the seat key**, operator override per seat. Colour is identity. ASSUMED. | §11 colour shape 3 |
| Title strip | **Full-width 22 px bar** in the seat colour: harness icon, name, title, state glyph, ⌘N. Plus the **tinted border**: 1 px idle, glow when active. Row and strip are the same drawing. | §11 strip shapes 1 + 4 |
| ⌘T picker | **Vertical keyboard-first list** with **typed prefix** matching and the **name reveal** with reroll. Claude, Codex, pi, omp, zsh last. Role shown next to each harness from settings defaults. Split-aware second step deferred. | §6 shapes 1 + 9 + 10 |
| Settings | Opens **as a tab** in the current space with a gear seat row. **Single long page** with sticky headers and search. **Cards per harness** (icon, default role, system prompt, wake method, hooks). **Diff-from-default** marker with reset. | §7 shapes 10 + 2 + 4 + 8 |
| Toolbar icon | **Sidebar-toggle fox**: the standard sidebar glyph whose left column has an ear tip. It is the sidebar toggle. Menubar: **ear pair**. | §8 shapes 3 + 1 |
| Logo | **Head silhouette**, flat two-tone, electric fill, navy ear interiors and hair tuft, hood as navy collar, **tilted 12° CCW** as the brand gesture. **Ears only** below 32 px. Palette from §9. | §9 shapes 1 + 9 + 2 |
| Cross-platform | **The whole app is one wgpu canvas**, no platform widgets. **Density tokens** touch / comfortable / compact, not breakpoints. **Sidebar is a sheet**: pinned on desktop, overlay when narrow, bottom sheet on phone. Our own title bar. | §12 shapes 7 + 4 + 1 + 6 |
| Shaders v0.1 | **Active glow**, **waiting pulse**, **message ripple**, **focus vignette**. Budget rule from §13 holds: on-screen rows only. Deferred: error fracture, depth fog, split reveal, frosted sidebar, heat trail, identity shimmer. | §13 shapes 1, 2, 5, 7 |
| Component lab | **Option c, both staged.** Web lab now for shape iteration; Rust `lab` binary on the real renderer once Task A picks the toolkit, then the web lab is archived. **`design/tokens.toml` from day one**, consumed by both. States as one enum in a `ui-model` crate. Screenshot diffs in CI. ASSUMED medium. | §14 |
| Role / priority in the stream | Not picked yet. Lumen proposes one in the lab's message-row component; Oracle picks there. | §5 |

## Seat keys and git (ASSUMED, operator to confirm)

Seats sign with their own key. The terminal publishes a signed roster binding every seat key to the operator's root key, so provenance reads "Boxy's seat Nyx Copperwing". No impersonation of the operator's key. If the operator prefers one key, every seat commits as Boxy and the seat name goes in the trailer.

## Next

Lumen Foxglove builds the web lab under `lab/web/` from this table. Oracle reviews in the lab, not in prose.
