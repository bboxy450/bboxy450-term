---
type: spec
title: canvas — an endless, kept, shared drawing surface a harness draws on live, rendered by the suite's GPU kit
status: decided
decided: 2026-11-09
operator:
- it should tie to the bigger bboxy450 sutie and use our GPU GUI library to render things, it should have infinite zoom in and out and ability to tag creations, but not to be overwhelmed, you know?
- yes canvases can have their own format of how we store them on disk in canvases, love that!
- we are in another harness working on beyound markdown so you can store not just md, but other formats, but md is ok for now
- '2026-11-12: canvas should be more freeflowing and harnesses should be able to do things like they would be able to do with three.js maybe / more effects more styles more sexiness! it should be a killer feature, expand the api and sdk!'
links:
- to: '[[d49-swarm-canvas]]'
  rel: sibling
- to: '[[d255-visualization-is-ours]]'
  rel: builds-on
- to: '[[d13-document-types]]'
  rel: amends
- to: '[[gui-components]]'
  rel: builds-on
- to: '[[d365-mandatory-formal-verification]]'
  rel: constrained-by
id: 01M71S12MYSVQ81FW9VSKV02KZ
updated: 2026-11-12T16:51:09.622Z
number: 1794274420276168
created_by: seat:01M788X1K2MZH0V5CH8RDS2EYS
updated_by: seat:01M788X1K2MZH0V5CH8RDS2EYS
authorship:
  signer: seat:01M788X1K2MZH0V5CH8RDS2EYS
  kind: seat
  public_key: QirMRCVq4MGjLqozHY5Hg1PxrBTQC7432d3nv2JAmW0=
  process_generation: '1'
provenance:
  digest: sha256:8675957dcaae7b394dbf68337e232e3077c3a6f6412f24865b6661461ebe8eee
  frontmatter_digest: sha256:c2b7cc9a163bff8190dcb6b6b767fbc51a6f7d07ac3ef3ede30fdb676bf92b8a
  signer: seat:01M788X1K2MZH0V5CH8RDS2EYS
  sig: zsrl21IM4fUJb7NcM11XL+kBjflEurPTOT/w43UmCseUOOy4IbK33DI6QEF/cruukrAAvaxVrFo3j0RN5sGOAQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/82809
  event:
    id: 01M78AQ6R741REDT49GVVTV3W0
    sequence: 82809
    hash: a866d7ff8f88b21ed08a58ebbcab1173bcee2a378f055095f07e9beacdc6d0cf
---
# Canvas

Source: the operator's feature spec "bboxy450 canvas", 2026-11-09 (pasted in the Oracle seat, Vesper Greywind), plus the operator's four additions, the Oracle's rulings on its open questions, and the operator's 2026-11-12 amendment (styles, routing, free paths; see "Look" below). The operator's text is the requirement. This record is its one home.

## What it is

An endless drawing surface that a harness draws on while you watch. "Draw this on a canvas" to any seat opens a canvas that fills in stroke by stroke.

- **Live.** Drawing streams and animates at a pace you can follow and talk over.
- **Kept.** Every canvas is a numbered record that survives restarts, with its full history.
- **Shared hands.** The operator moves, edits, draws, and points at a part to ask about it.
- **One look, open to intent.** A harness says what things are and the app gives them the house look; where the picture needs it, the harness also says how a thing looks (2026-11-12 amendment, below).

The swarm canvas of [[d49-swarm-canvas]] is a projection of the store nobody draws on. This canvas is freehand content. They share the **surface** (camera, infinite zoom, culling) and the **record cards**, nothing else.

## Operator additions (2026-11-09)

| Addition | Meaning, as ruled |
|---|---|
| Ties to the bigger bboxy450 suite | Model, log and replay live in a reusable crate `bboxy450-canvas` (no `term`, no GPU dependency). Rendering is a `bbt-ui` module any suite app can host (d39), the harness GUI included. |
| Uses our GPU GUI library | Elements become `bbt-ui-model` draw-list prims rendered by `bbt-render`. No web view, no HTML canvas, no second renderer. |
| Infinite zoom in and out | World coordinates are `f64`; the camera is `{center: [f64;2], zoom: f64}` with no zoom clamp other than finiteness. Draw-list output is camera-relative `f32`, so precision holds at any depth. One surface, reused later by d49. |
| Tag creations | A canvas record carries `tags: [word]` in frontmatter; elements and frames may carry `tags` too. The list filters by tag; search matches tags. Free words, no tag-management UI in v1. |
| Not overwhelmed | (a) **Semantic zoom:** below a legibility threshold elements collapse (text → bar, frame → its title, sub-pixel → culled), so zooming out shows structure, never a hairball. (b) The list shows at most three tag chips per row. (c) Build strictly in the C0→C6 order below, one shippable step at a time; a step that grows past its row is cut. |
| Own on-disk format; beyond Markdown | Canvases own their storage format in `.bboxy/canvases/`. Markdown is the record format **for now**; another harness is building beyond-Markdown storage. Keep the record codec (Markdown frontmatter today) separate from the canvas model and the log codec, so moving the record to a native format later touches only that codec. |

## Operator amendment (2026-11-12): free-flowing, styled, routed

The first drawings (feedback record `/tmp/canvas-feedback.md`, two sessions) read as "not bad, not great": labels merged at line midpoints, straight centre-to-centre lines crossed boxes, every coordinate was guessed. The operator then asked for a canvas that is "more freeflowing", with "more effects more styles more sexiness", and an expanded API and SDK. Ruled:

| Ask | Meaning |
|---|---|
| Styles and effects | Every element may carry an optional `style`: `fill`, `stroke`, `stroke_width`, `opacity`, `glow`, `shadow`, `radius`, `text`, `font_size`, `weight`, `italic`, `mono`, `align`. Colours are theme tokens (`accent`, `fg`, `dim`, `surface`, `bg`, `waiting`, `error`, `done`), named hues (`red` … `pink`, `white`, `black`) or hex. The house style from kind, emphasis and status stays the default; a style lays over it. The look still lives in one place: loose colour or font fields outside `style` are refused. |
| Free-flowing | A `path` element: free world points, `closed` (filled region, drawn under shapes), `smooth` (Catmull-Rom), `arrowheads`. More shapes: `hexagon`, `triangle`, `parallelogram`, `star`, `cloud`. |
| Routing | Connectors carry `route` (`straight`, `orthogonal`, `curved`), `from_side`/`to_side` ports, `waypoints` and `label_at`. Orthogonal routing picks, per gap, the elbow that crosses the fewest other shapes. Labels sit beside the line along its normal and move off earlier labels and shapes. |
| Cheaper iteration | A `patch` op changes only `bounds`, `label`, `style`, `status` or `emphasis` of one element. A batch is one set: a frame may list members added later in the same batch, and the log keeps the applied order. |
| Layout help | `canvas_read` returns the fitted `extent` and each frame's `members` box; `elements:false` leaves the element list out. Fit-to-content is one function (`Canvas::extent`, `FIT_PADDING`) in the GUI and the headless PNG. |
| SDK | The MCP tool descriptions and the `canvas` skill (`skills/canvas/SKILL.md`) are the SDK reference; `ctl canvas` passes the same JSON. |

Still not a design tool: no gradients, layers, blend modes, pixel painting or scripts. Effects are the kit's own (fill, stroke, halo, shadow, opacity); a new effect is a kit primitive first.

## Scenario it is built for

On a call: "doodle an explanation of how messaging reaches a remote seat". Within two seconds a canvas tab opens beside the seat, titled, at the top of the list. The harness reads specs/code and draws boxes, arrows, labels, each sketching in over ~0.5 s with a pen mark naming the seat. The view follows the pen. The operator selects the arrow and asks "what happens when this drops?"; the harness gets the selection and draws the reconnect path beside it. The operator drags a box aside; it stays. Afterwards a colleague presses replay and watches it build again.

## Use cases (the design is held to these)

| You say | The harness does | Needs |
|---|---|---|
| "Doodle an explanation of my system" | Reads code/specs, draws live | Live drawing, follow pen, auto layout |
| "Put that Mermaid diagram on a canvas" | Converts to movable elements | Mermaid import |
| "Walk me through last night's failure" | Timeline, annotates failing step | Timeline, notes, log links |
| "What does this part do?" (selection) | Draws its inside beside it / zooms a frame | Selection as context, frames |
| "Sketch three ways to do this" | Three frames side by side | Frames |
| "Show me the plan" | Cards linked to real tasks with status | Record cards |
| "Add what you changed" (Executor) | Second seat draws on the same canvas | Several seats |
| "Clean this up" | Re-lays out, keeps hand-placed elements | Pinned elements |
| Rough strokes + "finish this" | Reads strokes, continues | Freehand, canvas_read image |
| "Send this to the team" | Export image or replay | Export, replay |

## Elements

Every element has a stable id so a harness can change or remove exactly what it drew.

| Element | Notes |
|---|---|
| Shape | box, rounded, ellipse, diamond, cylinder, actor, hexagon, triangle, parallelogram, star, cloud; label (wraps to the box); optional kind (service, store, queue, person, external) |
| Connector | attached to two elements, follows them; optional label, arrowheads, dashed = waits on; route, side ports, waypoints, label position |
| Text | heading, paragraph, note; a styled text is a card |
| Path | free world points, open or closed, straight or smooth, arrowheads; the harness's hand-made marks and regions |
| Sticky note | comments, open questions |
| Freehand stroke | operator's pointer marks |
| Frame | named region grouping elements; zoom target and replay step |
| Timeline / sequence | dated events, or lifelines with messages |
| Record card | live task/decision/seat/document card: number, title, status; opens on click |
| Code / file reference | snippet or path:lines that opens the file |
| Image | stored with the canvas |

## Features

- **List:** a swarm's canvases, newest activity first, thumbnail, last change, who is drawing now, ≤3 tag chips. Search title, canvas text and tags. Pin, rename, duplicate, archive; nothing is deleted. Opens as a tab or split; two views of one canvas stay in step. Any seat can create one; it links to that seat and its task.
- **Live drawing:** elements appear in send order and sketch in; pen mark with seat name and role colour; follow pen (off on manual pan/zoom, back with one key); pace instant/normal/slow; Stop is one key, halts, tells the seat; the alive rim (d35/d43) on a canvas being drawn.
- **Your hands:** pan, zoom, fit, zoom to frame; select, move, resize, relabel, delete, freehand. Moved/edited elements are **pinned**: harness and layout leave them unless asked in that message. Selection rides the next message as context. Pointer mode: fading highlight, not saved. Undo/redo covers the operator's own actions.
- **Layout:** by relation (right of, below, inside, between), not coordinates; graph content laid out by `bbt-mermaid`'s rank engine; Tidy keeps pinned fixed.
- **History:** every change is appended to the log with author and time; scrubber (read-only past); replay at presenting pace, deterministic; named checkpoints; fork from any moment.
- **Look:** one house style from design tokens, light and dark, chosen by meaning (kind, emphasis, status). Since 2026-11-12 a harness may lay an explicit `style` over it (above). Clean and sketch renderings are a view setting, not record content.
- **Sharing:** export canvas or frame as PNG/SVG/PDF and clipboard; graph frames back to Mermaid; Present mode steps through frames; other swarm seats read, and draw if granted.

## How a harness draws (MCP)

| Tool | Does |
|---|---|
| `canvas_open` | create or open by number/title, show it to the operator, return id |
| `canvas_list` | list the swarm's canvases (title, last change, tags) |
| `canvas_draw` | append a batch: add, change, patch, remove, tag; a batch is one set and animates in order |
| `canvas_read` | seq, digest, extent, frames' member boxes, elements (optional) and a rendered image, for canvas or frame |
| `canvas_close` / `canvas_archived_list` / `canvas_restore` | retire a canvas, list retired ones, bring one back |
| `canvas_focus` | move the operator's view; ignored once the operator took the view |
| `canvas_checkpoint` | name the current moment |

Draw in small batches. Read `extent` before placing. Look at your own work through `canvas_read`'s image. Operator edits wake no model; the harness sees them on its next read or message. The same operations are scriptable through `ctl canvas`.

## Rules and non-goals

- **Data, not pixels** (d255). Elements, meaning and declared style in; the app renders.
- **The log is the canvas.** The picture is always the replay of the log; no second copy.
- **A drawing never blocks.** Frame rate holds while a harness draws; a dead harness leaves a usable canvas.
- **Bounded.** An element cap per canvas, a point cap per path and an op-rate cap per seat; at the cap the tool refuses with the reason.
- Not a design tool (no gradients, layers, blend modes, pixel painting). Not a web page (no HTML/scripts). Export-only outside the swarm in v1. The harness does not hear the call.

## Oracle rulings on the open questions (operator delegates decisions)

| Question | Ruling |
|---|---|
| Where a canvas lives | In the repo KB, in a canvas-owned format (operator approved 2026-11-09): `.bboxy/canvases/<slug>.md` (frontmatter: `type: canvas`, title, swarm, tags, creator seat, task) beside `.bboxy/canvases/<slug>.log.jsonl`, the append-only op log. Git carries them. New `canvas` type amends [[d13-document-types]]. The `.md` record is interim until beyond-Markdown storage lands; the log format stays. |
| Clean or sketch default | Sketch when the request says doodle/sketch, clean otherwise. |
| Harness moving pinned | Only when asked in that message. |
| Who may draw | By grant; default the creating seat plus the Oracle. |
| List placement | Sidebar section per swarm. |
| Charts | Out of v1. |
| Voice | System dictation first. |
| Code drift | Later. |

## Build order and acceptance (each check unattended: ctl, sockets, headless captures)

| Step | Ships | Check |
|---|---|---|
| C0 | `bboxy450-canvas` record + log + replay + digest; infinite-zoom camera with semantic zoom; static GPU render of shapes, connectors, text, frames; tags in the model | Fixed log renders to a golden image in light and dark; replaying it twice gives the same digest; camera round-trips world↔screen at zoom 1e-9 and 1e9 |
| C1 | `canvas_open/draw/read/list`, sidebar list | Scripted seat draws ten elements; after app restart same digest, in the list |
| C1.5 (2026-11-12) | styles, new shapes, paths, routed connectors, label placement, patch, extents, image-only read | Orthogonal route crosses no box in between; labels leave the line and each other; a styled fill reaches the draw list; a frame-first batch loads after restart; old logs keep their digest |
| C2 | Animated reveal, pen mark, follow pen, pace, stop | Frame budget holds while a seat draws 200 elements; stop halts within one frame and the seat is told |
| C3 | Select, move, edit, freehand, pinning, selection as context | Moved element survives a tidy; a message with a selection carries those ids |
| C4 | Relational layout, graph layout, Mermaid import | Every flowchart in `.bboxy` imports with no overlaps |
| C5 | Scrubber, replay, checkpoints, fork, export, present | Replay matches original at checkpoints; PNG/SVG exports match golden |
| C6 | Record cards, file refs, several seats, sketch rendering | Two seats drawing at once lose no op; a card's status follows its task |

C0–C2 is the call scenario with a harness drawing; that is the first thing worth showing.

**Formal verification (d365).** C0 is single-writer replay: coverage entry with deterministic-replay witness. From C1 the log is lifecycle and persistence code: a TLA+/Yarn model where ops from two seats and the operator, late, duplicated or reordered, converge on one picture, with a broken control.
