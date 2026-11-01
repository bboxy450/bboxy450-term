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

## Streaming and prepared code

`StreamingMarkdown::append` retains completed prefix blocks, reparses the open
tail and preserves absolute UTF-8 source offsets. An incomplete first line may
still join its predecessor (notably a table row), so that predecessor remains in
the tail until the line settles. EOF alone never freezes a block. Only reference
or footnote definition lines (`^\s{0,3}\[[^\]]+\]:`) introduce document-wide
dependencies; those documents retain the full parser's reference context.
Inline links, task lists, indexing and JSON brackets keep prefix reuse. The
definition scanner examines only the unfinished line and appended bytes.
YAML remains unsettled until its delimiter
closes. `reparsed_range()` exposes the work actually performed.

`highlight::Highlighter` is the owned-span hook. `RustHighlighter` uses syntect
5.3 with only parsing, bundled syntaxes/themes and `regex-fancy`. The SyntaxSet
and ThemeSet are shared through a process `OnceLock`, initialized on a preparation
worker. `Highlights::prepare` validates exact text coverage and
binds spans to the block offset, language and source; unsupported languages keep
plain readable text. `highlighted_body_element` only composes prepared runs.

`highlight::Work::prepare_off_thread()` schedules parsing and highlighting on
`bbt_rt` blocking workers. `Preparation::request` captures a generation and
revision; `install` accepts only the current request, retaining the previous
prepared body while work is pending. Call `retire` on view replacement/drop,
even for A → B → A. Hosts own task cancellation and wake delivery, as with
`documents::Work::prepare` and AsyncRenderPreparation. A streaming host can move
its retained `StreamingMarkdown` and highlighter to that same worker instead of
using `Work`'s one-shot full-document parse. No toolkit code knows a transport or
application protocol.
