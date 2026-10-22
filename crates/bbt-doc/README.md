# bbt-doc

Native CommonMark/GFM parsing and retained layout, above `bbt-ui`. The host
supplies font measurement and executes the component draw list. This crate
does no file, network, renderer or application IO.

`parse(source)` preserves byte ranges, YAML fields, nested blocks, styled
inlines, table alignment, footnotes and annotated wikilinks. Invalid metadata
and HTML stay readable as source. `body_element` feeds the `DocumentModal`
body seam; `modal_element` projects a real file's header and metadata too.
Metadata that resembles a signature is never promoted to verified evidence.

Mermaid from actual Markdown is a Source-only figure. Supplied 7.10 semantic
fixture figures remain available to the parity gallery; W3.5b supplies the
native Mermaid producer. Code is monospace without highlighting, and image
references retain their alt text and destination without loading images.

```sh
cargo test -p bbt-doc --locked
cargo test -p bbt-doc --locked --test corpus real_bboxy_frontmatter_parses -- --exact --nocapture
cargo run -p bboxybook -- documents --repository --out target/bbt-doc
```

The corpus gallery enumerates tracked `.bboxy` Markdown, lays out every file
with real font metrics in both themes, and renders top/middle/end viewports.
Its `report.json` records natural heights and unclipped horizontal overflow;
`index.html` opens the native images.

The corpus test reads every Markdown file under the real `.bboxy/` and fails
malformed or unclosed YAML with `path:line` from the same parser used by the
reader. The reader still shows broken metadata and the document body as source.
