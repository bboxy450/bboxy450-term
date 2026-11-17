---
id: 7GC9MD2N0P0ZDQ7NPD6D4KZJEM
type: spec
number: 118
title: Document viewer — native Markdown and Mermaid
created: 2026-10-22T00:22:14Z
links:
- to: '[[d52-brand-and-face]]'
  rel: decision
intent: Cmd-clicking a Markdown path in any pane opens a native modal that renders the Markdown and Mermaid with the app's own typesetting, and clicking outside it closes it and returns focus.
updated: 2026-11-17T00:52:04.715Z
created_by: seat:01M79HGAW321GYNK8468FYN4K9
updated_by: seat:01M79HGAW321GYNK8468FYN4K9
authorship:
  signer: seat:01M79HGAW321GYNK8468FYN4K9
  kind: seat
  public_key: O/GLv3wEDMQc2X2CJwZN8QH5avRWsmx3x4SdZZC6lKU=
  process_generation: '1'
provenance:
  digest: sha256:cf34bc5906b903194dcac2de91857b6a530f6eee3cef67dd86d1a2e29d7c7326
  frontmatter_digest: sha256:12994b3f3536ef826dba02eb68de4b14ac4274459cdb84c237ada2aa5ce23ad7
  signer: seat:01M79HGAW321GYNK8468FYN4K9
  sig: KXDM9mQVYZEL118+m18yQMezgCvkyausS9aehu2MsaCvkmXfdSBkKQUIa6qWlvaUu7I8thDC3wbYY4DzAcy0Bg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151819
  event:
    id: 01M7KFTNJKV6G1AFKRZMJGG7V7
    sequence: 151819
    hash: 235f7cb3b31b50ab43063862292d3e95544be5782792a7e0ee826c55fd234d4b
---
# Document viewer — native Markdown and Mermaid

Owner: Vesper Greywind <ORACLE>. 2026-10-21. DECIDED by the operator: "native markdown viewer in our Rust app, with nice mermaid rendering, built ourselves; a harness gives me a markdown path, I ⌘-click, I see a nice document." Back to [[vision]] · [[renderer]] · [[gui-components]] · [[work-items]] · [[ux-prototype]] (documents.html is the design).

## What it is

A **document modal** (operator, 2026-10-21: "it should be a modal; the usage pattern is open, scroll, copy maybe, close, feed into the harness"): ⌘-click on a path ending in `.md` in any pane (or an OSC 8 hyperlink, or a wikilink in the stream, or a work item in the sidebar and Graph tab) opens the file rendered natively by our wgpu renderer in a centred modal over the blurred window, the launcher's material and size class, with the frontmatter header, provenance strip and graph rail from the lab's documents page when the file is a `.bboxy/` item, or a plain document view for any other Markdown. Keyboard: scroll with arrows, space and page keys, ⌘F find, ⌘C copies the selection as Markdown source (not rendered text), ⌘A selects all, ⌘⇧C copies the path, **⏎ or ⌘⏎ feeds the document (or the selection) into the focused seat's harness as a message through the adapter and closes**, Esc closes. The modal never steals the seat's keyboard focus for longer than it is open; closing returns focus exactly where it was. A "pin as tab" action exists for the rare long read, but the modal is the default. No webview, no browser, no JavaScript.

Operator, 2026-10-29: "clicking outside of it would close the viewer". A primary click on the backdrop outside the painted modal panel closes it and returns focus to the previous seat. The dismissal consumes the click; it never activates the obscured application underneath. Inside whitespace, selection, links and footer controls retain their own behavior. This applies to the modal presentation; an embedded or pinned document does not claim clicks elsewhere in the window.

## Path detection in panes

Like iTerm2's semantic history: the terminal recognises `path`, `path:line`, `path:line:col` tokens in cell text, resolves them against the seat's cwd (from OSC 7) and the worktree, and underlines them on ⌘-hover. ⌘-click on `.md` opens the document tab; on code opens the seat's editor command (setting); on a directory opens a listing. OSC 8 hyperlinks are honoured as is. Wikilinks `[[…]]` anywhere in pane text resolve through the `.bboxy` index.

## Markdown

Parse with `pulldown-cmark` (MIT; CommonMark + GFM tables, task lists, footnotes, strikethrough). Our own typesetting on the renderer's text pipeline: headings on the type scale from `style.json`, paragraphs with proper wrapping and hyphenless justification off, lists, block quotes, tables (our layout engine's row/column solver), images (PNG/JPEG via `image` crate into the colour atlas or a texture), links, footnotes, YAML frontmatter rendered as the header grid, wikilinks as chips with `{rel:w}` badges, code blocks highlighted with tree-sitter (grammars shared with the code graph) and copied with one click, math deferred.

Operator, 2026-10-29: "tables dont fit into the markdown viewer view, lets improve that". Tables use the full available document-body column while ordinary prose keeps its readable measure. Text stays at the normal Body type size. Long cells wrap within fitted columns, preserving rich formatting, alignment and the original Markdown. Nested tables use their actual enclosing column. A table whose glyphs, atomic chips and padding still cannot fit keeps the existing horizontal pan rather than losing content or shrinking type.

Operator, 2026-10-29: provenance in tasks is "too verbose of a noise" and should be hidden behind a link. Nested `provenance` and `authorship` metadata appear behind a compact **Provenance details** disclosure, collapsed whenever a document opens. Clicking reveals the full semantic values in the existing horizontally scrollable code view; clicking again hides them. Other metadata and malformed-source diagnostics retain their normal presentation. This is display state only: source bytes, stored metadata, hashes, signatures and exports are not rewritten, and displaying a signature never claims verification. The control uses the document's clipped hit regions and actionable pointer cursor.

## Images ([[d52-brand-and-face]], operator 2026-10-23)

Images live in `.bboxy/images/` as files. `![[bboxy.png]]` renders the image inline at true size ([[renderer]] §4.4b, never scaled; wider than the column scrolls), `[[bboxy.png]]` is a link to it; both resolve by file name in that one folder and nowhere else, so a link cannot rot ([[images-resolve]]). An image is not a record: no frontmatter, no number; one a seat produces is an artifact of kind image ([[work-items]]). The face `bboxy.png` is the first image and is the identity file byte for byte.

## Mermaid, built ourselves

A Mermaid subset parser and layout engine in Rust, rendering to our draw list (no browser):

| Diagram | v1 | Layout |
|---|---|---|
| flowchart / graph (TD, LR; nodes, edges, labels, subgraphs, shapes) | yes | layered Sugiyama: rank assignment, crossing minimisation (barycentre), coordinate assignment (Brandes-Köpf-lite), orthogonal-ish edge routing with rounded bends; our own ~1.5k lines |
| sequence (participants, messages, activations, notes, loops/alt) | yes | lifeline columns, time rows; trivial |
| state (states, transitions, start/end, composite) | yes | reuses the layered engine |
| class, ER | later | layered |
| gantt, pie, mindmap, timeline | later | special-cased |

Rendering uses the chrome primitives (rounded rects, hairlines, text runs, arrowheads as a new primitive) in the seat's colours, with the tokens' type scale; diagrams draw at true size and scroll sideways when wider (renderer.md §4.4b, Wren ec7d8df figure card). The parser rejects unsupported syntax by rendering the raw block with a note, never a broken diagram.

## Crate layout

`crates/bbt-doc`: `markdown/` (parse → block tree → layout → draw list), `mermaid/` (lexer, parsers per diagram, `layout/layered.rs`, `render.rs`), `paths.rs` (semantic history detection). Consumed by `bboxy450-term` (document tab) and `bboxybook` (document and diagram galleries: every lab documents.html state and a Mermaid fixture set rendered through the parity loop).

## Build order (Executor-sized)

1. `bbt-doc` markdown block tree + layout on the text pipeline; bboxybook gallery renders `.bboxy/vision.md` and the lab's documents states. Verify: headless render, no overflow, headings on the type scale.
2. Path detection + ⌘-click in the app; OSC 8; wikilinks via the index.
3. Mermaid lexer + flowchart parser + layered layout; fixture set of 20 flowcharts from the Mermaid docs rendered headless and inspected by a harness.
4. Sequence and state diagrams.
5. Code blocks via tree-sitter, images, tables.

## Gaps

- **Gap:** Mermaid grammar coverage target; start from the subset used in this repo's `.bboxy/` and the Mermaid docs' examples.
- **Gap:** large documents: virtualised layout past ~5k lines; measure first.

Links: [[vision]] [[renderer]] [[gui-components]] [[work-items]] [[ux-prototype]] [[plan]]

## Figures

Adapted from mdxcn (MIT, https://github.com/shadcn-labs/mdxcn): a figure sits in a
dashed frame with `+` corners and its title as `[ TITLE ]` centred on the top edge, in
the accent; the frame ink is the muted foreground at half alpha.

- A GFM alert (`> [!NOTE]`, `[!TIP]`, `[!IMPORTANT]`, `[!WARNING]`, `[!CAUTION]`) is a
  callout: the kind as its title and one glyph — `i` muted, `+` and `!` accent, `×` in
  the errored state colour — beside the prose. A plain `>` quote keeps its left rule.
- A fenced code block is framed with its language, uppercased, as the title; an
  unlabelled fence has no title. Wide code still pans in its own viewport, keyed by the
  block. mdxcn's pasteable ASCII figures render as written inside a plain fence.
- Witness: `crates/bbt-doc/tests/layout.rs`
  `callouts_and_code_blocks_are_framed_with_their_title_on_the_top_edge`.
