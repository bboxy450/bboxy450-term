# bboxy450-ui-core — W3.2a

The first three steps of [[gui-components]]: compile-time tokens, renderer-free
model/tree edits and logical-pixel layout. Reference: `ux-prototype-v2`, before
the later graph/task/artifact UI passes. No window, GPU or runtime dependency.

- `build.rs` reads `design/tokens.toml` and `design/style.json`, emits typed
  structs and `TOKENS`/`STYLE` into `$OUT_DIR/tokens.rs`, and tracks both source
  files with Cargo. TOML/JSON libraries are build dependencies only.
- `src/tokens.rs` exposes those constants, `Theme` and `Density`. The parity
  test parses `window.TOKENS`/`window.STYLE` with a regex and JSON parser (no
  Node) and walks the actual generated fields: **210 paths**, including object
  keys, array lengths and every scalar value. No copied JSON comparison.
- `src/model/ids.rs` holds distinct 128-bit ULID ID types and a canonical
  Crockford-base32 codec. The app/mux supplies IDs; this crate has no RNG.
- `src/model/{seat,swarm,tab,split,window}.rs` carries open harness names,
  `{name,class}` roles, activity/grade/lifecycle as separate axes, owned remote
  location, mute/unread/task references, sidebar order and split-tree order.
  Window/launcher/settings/pane/mini-grid/stream shapes contain data only.
- `src/event.rs` separates app intents from authoritative mux updates.
- `src/layout/{size,solve,split,sidebar}.rs` implements fixed/text/flex/fill
  rows and columns, ratio splits, divider hit zones, virtual sidebar rows,
  local scroll-to-row and phone-sheet geometry. Gutters/density use tokens;
  glow occupies no layout space.

Tree algebra returns a new model/tree, leaving its input unchanged. `prune`
collapses empty branches; `parent_of` records sibling subtrees; `replace_node`
uses stable branch paths. `detach` collapses an empty tab; `place` recalculates
the target after detaching and uses 50/50 placement on each of four sides.
`close` enters grace; `undo` restores the tab index, split side and ratio while
preserving a sibling's later resize. Settings is a local view and closes
directly. Generation replacement, expired undo and lost sibling layouts return
explicit errors. Expiring the undo affordance does **not** prove termination:
only `SeatClosedByDaemon` changes it to `ClosedRetained`.

The 15-seat fixture was extracted from tag blob
`d08c931983547f774e95d04690987c4aa52108d4` (`showcase.js`). Its checked-in
reference rectangles use independent decimal CSS-flex arithmetic. Tests match
all 15 panes and 14 two-pixel gutters, without outer margins, and reuse the
solver at mini-grid/banner scales. At 1000 seat rows a 240 px viewport allocates
**10** row rectangles; a separate test skips offscreen collapsed swarm cards.

Verified locally, 2026-10-21: **28 model/layout/token tests**, **162 workspace
tests**, `cargo build --workspace --locked`, clippy with `-D warnings`, fmt and
whitespace checks all passed. `cargo tree -p bboxy450-ui-core --edges normal
--locked` lists only this crate. Actual output: `.bboxy/runs/w3-2a-final-*.log`.

Lab differences and remaining behaviour:

- No rendering, native window, full-window component assembly, browser-pixel
  comparison, device-pixel rounding, text shaping/ellipsis/wrapping, CSS
  intrinsic sizing, frosted blur, glyphs/icons, glow, ripple or animations.
  `TextMeasure::width` is for shrinkable single-line row labels; column rows
  use fixed token heights. Fixed children may overflow the caller's clip.
- Pointer/keyboard routing, dwell focus, drag/resize event handling, OS theme
  observation and persistence are later steps. Phone geometry/snap is pure;
  the 600 px breakpoint remains the spec's assumption, not a measured lab
  transition. The 62% sheet, 44 px handle and 320 ms duration follow its frame.
- Launcher search/nesting/semantic matching, settings form behaviour, streams,
  shadow interactions, unread-priority decisions and task/graph/artifact rows
  are data shells/references only. Nine untyped modules and six signals listed
  in the spec remain deferred. No native bboxybook previews in this brief.
- `style.json` now supplies `spacing.title_bar` and `spacing.band` (Wren,
  f63e37b). Bumper and some other chrome dimensions remain CSS text. W3.2a
  `CardChrome` accepts caller metrics; bboxybook composes the new style tokens.
  Shadow-group header and footer/dock chrome remain caller composition.
- Unlike the old JS's silent failed sibling replacement, undo rejects a layout
  conflict. Cross-swarm placement is rejected per the current grant contract;
  the old lab permitted it. Grace expiry retains the seat and waits for mux
  evidence, rather than the lab timer deleting it or pretending to kill it.
  Native process control, durable undo and restart recovery belong to the mux.
- Colours use the lab's UTF-16 FNV hue/band derivation with an explicit HSL
  override. CSS colour-string parsing and showcase palette/theme overrides
  belong to the renderer/fixture caller. No real-font, phone-device, Linux or
  hosted-CI check was performed by this pass.

## Dependency name

The Rust library name remains `bbt_ui_model`. Use the existing import name with a versioned package alias:

```toml
bbt-ui-model = { package = "bboxy450-ui-core", version = "0.1.0" }
```
