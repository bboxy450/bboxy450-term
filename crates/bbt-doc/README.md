# bboxy450-doc

Native CommonMark/GFM parsing and retained layout, above `bbt-ui`. The host
supplies font measurement and executes the component draw list. This crate
does no file, network, renderer or application IO.

`parse(source)` preserves byte ranges, YAML fields, nested blocks, styled
inlines, table alignment, footnotes and annotated wikilinks. Invalid metadata
and HTML stay readable as source. `body_element` feeds the `DocumentModal`
body seam; `modal_element` projects a real file's header and metadata too.
Metadata that resembles a signature is never promoted to verified evidence.

Mermaid uses `bbt-mermaid`'s public geometry with native font measurement,
retained shapes, filled/open arrowheads, clusters and sequence frames. The
shared figure card draws at true size and scrolls sideways. Unsupported syntax
shows Source with its error. Document parity consumes the real engine; diagram
bodies follow renderer §4.4c while the figure chrome stays gated. Code is
monospace without highlighting, and image
references retain their alt text and destination without loading images.

```sh
cargo test -p bboxy450-doc --locked
cargo test -p bboxy450-doc --locked --test corpus real_bboxy_frontmatter_parses -- --exact --nocapture
cargo run -p bboxy450-book -- documents --repository --out target/bbt-doc
```

The corpus gallery enumerates tracked `.bboxy` Markdown, lays out every file
with real font metrics in both themes, and renders top/middle/end viewports.
Its `report.json` records natural heights and unclipped horizontal overflow;
`index.html` opens the native images.

The corpus test reads every Markdown file under the real `.bboxy/` and fails
malformed or unclosed YAML with `path:line` from the same parser used by the
reader. A metadata block also requires `id` and `type` for the work-item index.
The reader still shows broken metadata and the document body as source.

## Dependency name

The Rust library name remains `bbt_doc`. Use the existing import name with a versioned package alias:

```toml
bbt-doc = { package = "bboxy450-doc", version = "0.1.0" }
```

## Respectful slopping

Project-owned contributions use [Respectful Slop License 1.0-Draft](LICENSE).
Retained upstream material keeps its original terms; earlier grants are not
withdrawn. [ATTRIBUTION.md](ATTRIBUTION.md) identifies the scope, source and reuse
examples; [NOTICE](NOTICE) preserves factual upstream notices.
Slop respectfully. Keep the credit attached.
