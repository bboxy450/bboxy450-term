# UX pick — Oracle selection from the divergence field

Selected by Vesper Greywind <ORACLE> on 2026-10-21 from [[ux-divergence]] under the constraints in [[ux-sidebar]]. Items marked ASSUMED are Oracle defaults the operator has not confirmed; the operator overrides by saying so. Back to [[vision]].

| Element | Pick | From |
|---|---|---|
| Sidebar | **Pill stack** with a **two-density** toggle (comfortable / compact). Space = vertical pill in the space colour, tabs as rows inside, each tab row ends in its mini-grid and ⌘N. At scale: collapse by space then tab, rolled-up worst-first glyphs and counts, footer `N seats · M waiting`. Rows never move. | §10 shapes 1 + 8, §2 collapse |
| Split mini-grid | The tab's split tree drawn as a small grid beside the tab row, V and H shapes, focused cell brighter. Same grid becomes the phone's home. | §10 shape 1 ASCII, §12 shape 3 |
| State glyphs | Set A: `●` active, `○` idle, `◐` waiting, `✕` errored, colour-coded (seat colour, dim, amber, red). Waiting pulses at 1 Hz. Nothing else animates by default. | §4 shapes 1 + 3 |
| Seat colour | **Hash of the seat key**, operator override per seat. Colour is identity. DECIDED (operator: "seat color is great"). | §11 colour shape 3 |
| Title strip | **Full-width 22 px bar** in the seat colour: harness icon, name, title, state glyph, ⌘N. Plus the **tinted border**: 1 px idle, glow when active. Row and strip are the same drawing. | §11 strip shapes 1 + 4 |
| ⌘T picker | **Vertical keyboard-first list** with **typed prefix** matching and the **name reveal** with reroll. Claude, Codex, pi, omp, zsh last. Role shown next to each harness from settings defaults. Split-aware second step deferred. | §6 shapes 1 + 9 + 10 |
| Settings | Opens **as a tab** in the current space with a gear seat row. **Single long page** with sticky headers and search. **Cards per harness** (icon, default role, system prompt, wake method, hooks). **Diff-from-default** marker with reset. | §7 shapes 10 + 2 + 4 + 8 |
| Toolbar icon | **Sidebar-toggle fox**: the standard sidebar glyph whose left column has an ear tip. It is the sidebar toggle. Menubar: **ear pair**. | §8 shapes 3 + 1 |
| Logo | **Head silhouette**, flat two-tone, electric fill, navy ear interiors and hair tuft, hood as navy collar, **tilted 12° CCW** as the brand gesture. **Ears only** below 32 px. Palette from §9. | §9 shapes 1 + 9 + 2 |
| Cross-platform | **The whole app is one wgpu canvas**, no platform widgets. **Density tokens** touch / comfortable / compact, not breakpoints. **Sidebar is a sheet**: pinned on desktop, overlay when narrow, bottom sheet on phone. Our own title bar. | §12 shapes 7 + 4 + 1 + 6 |
| Shaders v0.1 | **Particle glow on the seat border in the seat colour**, driven by activity. DECIDED by the operator 2026-10-21: idle or done = very mild static glow; active = pulsating; focused = brighter at every state. Plus **message ripple** and **focus vignette**. Budget rule from §13 holds: on-screen seats only. Deferred: error fracture, depth fog, split reveal, frosted sidebar, heat trail, identity shimmer. | §13 shapes 1, 2, 5, 7 + operator |
| Component lab | **Option c, both staged.** Web lab now for shape iteration; Rust `lab` binary on the real renderer once Task A picks the toolkit, then the web lab is archived. **`design/tokens.toml` from day one**, consumed by both. States as one enum in a `ui-model` crate. Screenshot diffs in CI. ASSUMED medium. | §14 |
| Role / priority in the stream | Not picked yet. Lumen proposes one in the lab's message-row component; Oracle picks there. | §5 |

## Visual signal semantics (DECIDED 2026-10-21, after operator: "too many blinky circles, inconsistent")

One signal per meaning. One meaning per signal. Nothing animates without a cause the operator produced or a state change in a seat.

| Signal | Means | Fires when | Never fires on |
|---|---|---|---|
| **Locate ripple** (ring from row ↔ pane corner, seat colour) | "this row is that pane" | an explicit focus change by click or keyboard, both directions; clicking a row whose seat is already visible | mouse dwell, timers, message arrival, state change |
| **Border glow** (particles, seat colour) | the seat's **state** | continuously: idle mild static, active breathing, waiting amber pulse, errored red static | anything else |
| **Focus brightness** (glow and strip brighter) | "this seat has keyboard focus" | focus is here | hover |
| **State glyph** (● ○ ◐ ✕) | the seat's state | state change; waiting pulses with the glow | anything else |
| **Unread dot** on a row (role colour) | "a message for this seat is unread" | message arrival | anything else, no ripple for messages |
| **Hover** | "the pointer is here" | pointer over row or pane: subtle highlight only | no glow change, no ripple |

Mouse-dwell focus stays, but it only changes focus brightness; the ripple needs a click or a key. Demo pages do not randomise states on a timer; a demo exposes keys (1 idle, 2 active, 3 waiting, 4 errored on the focused seat; M = message arrives) so one thing changes at a time and the operator sees cause and effect.

## Row-pane link ripple (DECIDED, operator loved it 2026-10-21)

Clicking a pane fires a ripple on its sidebar row; clicking a row fires it from the **centre of the pane** (operator, 2026-10-21: centre, not the top-left corner). The sidebar scrolls the row into view if needed. The ripple is the map legend: it answers "where is this seat in the list" and "which pane is this row" without reading. Keep it in the real build as a shader effect tied to focus change, both directions, every time.

## Border glow spec (DECIDED, operator words)

GPU particle glow around the seat border in the seat colour. Four readings, one shader:

| state | glow |
|---|---|
| idle / done | very, very mild, static |
| active | pulsating |
| waiting | pulsating, amber tint, 1 Hz (from the glyph pick) |
| errored | static, red tint |
| focused | brighter than the same state unfocused |

Particles, not a flat blur: a sparse emitter along the border, count and speed scale with activity (PTY output cadence or hook state), colour is the seat colour. Rendered per on-screen seat only. The sidebar row carries the same glow at reduced radius so the row and the pane breathe together.

## Seat keys and git (ASSUMED, operator to confirm)

Seats sign with their own key. The terminal publishes a signed roster binding every seat key to the operator's root key, so provenance reads "Boxy's seat Nyx Copperwing". No impersonation of the operator's key. If the operator prefers one key, every seat commits as Boxy and the seat name goes in the trailer.

## Next

Lumen Foxglove builds the web lab under `lab/web/` from this table. Oracle reviews in the lab, not in prose.
