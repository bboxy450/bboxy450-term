# bboxy450-mermaid

Our own Mermaid engine: parse a diagram and lay it out to plain geometry. No rendering, no workspace dependencies.

```rust
let g = bbt_mermaid::render(src, &my_measure, &bbt_mermaid::Style::default())?;
// g.nodes, g.clusters, g.edges, g.lines, g.size: true size, never scaled
```

- **Text measurement is the caller's.** Implement `Measure` (one line plus a `TextRole` gives a width and line height) with the real type scale. `MonoMeasure` is a fixed-width estimate used by the tests and the debug dump. Debug SVGs are only as accurate as that estimate, so a label pill can be narrower or wider than its rendered text there. That is the estimate's limit, not a layout bug.
- **Errors mean "show Source".** Unsupported syntax returns `Error { line, message }`. The caller shows the raw block with the message, never a broken diagram.

## Supported

| Diagram | Syntax |
|---|---|
| `flowchart` / `graph` | TB/TD/BT/LR/RL; 14 node shapes; `A[[slug]]` sets `Node::link`; `-->` `---` `-.->` `==>` `~~~` `<-->` `o--o` `--x`, longer links (`--->`), labels (`-- t -->`, `-->\|t\|`); `&` groups; `:::class`, `class`, `classDef`, `style`; nested `subgraph` with its own `direction` (kept only while no edge crosses its border, as in Mermaid) |
| `sequenceDiagram` | `participant`/`actor` `as` (with `:::class`); all ten arrows; `+`/`-` and `activate`/`deactivate`; notes left/right/over; `loop` `alt`/`else` `opt` `par`/`and` `critical`/`option` `break` `rect`; `autonumber` |
| `stateDiagram`, `stateDiagram-v2` | states, `state "name" as id`, `id : description`, transitions with labels, `[*]` start/end per scope, composite states (nested), `<<choice>>` `<<fork>>` `<<join>>`, notes left/right of a state, `direction`, classes |

Not supported (errors): `@{ }` node metadata, sequence `box`/`create`, state concurrency `--`, other diagram types.

## Layout

Flowcharts and state diagrams use a layered (Sugiyama-lite) layout, `src/layered.rs`. Each subgraph is laid out inside out as one box, and edges that cross a border go through ports pinned to it. Ranks are doubled so edge labels get their own rank. Barycentre ordering is followed by transposes, and x comes from exact isotonic placement. Ports flip sides for back edges. Sequence diagrams use columns and rows, `src/sequence.rs`.

## Fixtures

- `tests/fixtures/bboxy`: every Mermaid block in `.bboxy`. `every_bboxy_mermaid_block_has_a_fixture` fails when a new block has no fixture, so copy it in as `<spec>-<n>.mmd`.
- `tests/fixtures/docs`, `tests/fixtures/state`: examples from the Mermaid docs.

Debug dump, one SVG per fixture:

```sh
cargo run -p bboxy450-mermaid --example dump_svg -- OUT_DIR crates/bbt-mermaid/tests/fixtures/*
```

## Dependency name

The Rust library name remains `bbt_mermaid`. Use the existing import name with a versioned package alias:

```toml
bbt-mermaid = { package = "bboxy450-mermaid", version = "0.1.0" }
```

## Respectful slopping

Project-owned contributions use [Respectful Slop License 1.0-Draft](LICENSE).
Retained upstream material keeps its original terms; earlier grants are not
withdrawn. [ATTRIBUTION.md](ATTRIBUTION.md) identifies the scope, source and reuse
examples; [NOTICE](NOTICE) preserves factual upstream notices.
Slop respectfully. Keep the credit attached.
