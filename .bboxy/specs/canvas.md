---
authorship:
  kind: seat
  process_generation: '1'
  public_key: zaZM/RXH8b9WIWoi0q31BjOJ4guZHxdNg4Ux4XiqNMI=
  signer: seat:01M7TBJ9FCVMEJ19T5G3N2EFJA
created_by: seat:01M79HGAW321GYNK8468FYN4K9
decided: 2026-11-09
id: 01M71S12MYSVQ81FW9VSKV02KZ
intent: Any seat can open an endless canvas that it draws on live, stroke by stroke; every canvas is a numbered record with full history that survives restarts, and the operator can move, edit, draw and point on it.
links:
- rel: sibling
  to: '[[d49-swarm-canvas]]'
- rel: builds-on
  to: '[[d255-visualization-is-ours]]'
- rel: amends
  to: '[[d13-document-types]]'
- rel: builds-on
  to: '[[gui-components]]'
- rel: constrained-by
  to: '[[d365-mandatory-formal-verification]]'
number: 1794274420276168
operator:
- it should tie to the bigger bboxy450 sutie and use our GPU GUI library to render things, it should have infinite zoom in and out and ability to tag creations, but not to be overwhelmed, you know?
- yes canvases can have their own format of how we store them on disk in canvases, love that!
- we are in another harness working on beyound markdown so you can store not just md, but other formats, but md is ok for now
- '2026-11-12: canvas should be more freeflowing and harnesses should be able to do things like they would be able to do with three.js maybe / more effects more styles more sexiness! it should be a killer feature, expand the api and sdk!'
status: decided
title: canvas — an endless, kept, shared drawing surface a harness draws on live, rendered by the suite's GPU kit
type: spec
updated: 2026-11-19T23:51:45.988Z
updated_by: seat:01M7TBJ9FCVMEJ19T5G3N2EFJA
provenance:
  digest: sha256:264d000300ac2431f2092ad1032d8e36073f419831919819567edf697321439e
  frontmatter_digest: sha256:199ff7c75b7de9a7fe77b0c53a5994ca4d20a2bdf5f4e1c6fec930d22a0a0cc1
  signer: seat:01M7TBJ9FCVMEJ19T5G3N2EFJA
  sig: R/vi207rhK39Om/2AlfxyvxrUw3/Z9JsvKeV0fiZIm6fXkX9QHeeLHKB4nU5BfOzjtmhXkwpKimW4dbUVH08DA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/83106
  event:
    id: 01M7V3JCMCXYKM1QVBGQCXJP60
    sequence: 83106
    hash: 87525b34a49860f180bdbeefda6d27ba58aa4bfef4bdd822974d50469fc64557
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
- **The log is the canvas.** The picture is the replay of placement operations, with one explicit exception: an unpinned stored artifact image resolves latest content outside the log. Pinned images and reproducible capture follow the stored-artifact amendment below; no second mutable copy of image content.
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


## Draft amendment: stored artifact images

Companion to [[specs/python-charts]], under Architect/operator review; no implementation is implied by this KB edit.

An image placement is `Image { bounds, artifact: "ID[:N]" }`, backed by the generic stored artifact system. The canvas log owns the placement and reference; the artifact event stream owns revisioned image bytes. Existing add/change/remove, selection, move/resize, clipping and pan/zoom apply. There is no chart-specific element or copied mutable chart source. SVG painting uses the shared bboxy450-svg renderer specified in [[specs/svg-renderer]]. Removing a placement never deletes its artifact.

**The single live-replay exception:** `artifact:ID` without a revision resolves the artifact's latest authorized revision when viewed or replayed. Its pixels therefore depend on content outside the placement log and can change after the log was written. This explicitly qualifies the deterministic-replay promise for unpinned artifact images.

`artifact:ID:N` resolves exactly revision N and its retained bytes. A reproducible checkpoint, history scrub or export first resolves every live reference to an exact revision and captures those pins, required SVG bytes and renderer/font identity; it never labels latest-content replay as an exact historical picture. A previously captured checkpoint preserves its pins. Missing/inaccessible payloads remain visible placeholders, not live-source refetches. Blob bytes are machine-local and do not accompany a KB push.

Canvas and harness embeds share the new authenticated artifact branch in images::load and bboxy450-svg preparation/painting. The existing local PNG decoder remains unchanged. The shared image representation retains SVG vector drawing data and paints at the current camera transform; any raster work covers a bounded visible viewport, rather than stretching the first bitmap. Host limits are named once in [[specs/python-charts]]; SVG parser/path/glyph bounds live in [[specs/svg-renderer]]. Reuse prepared geometry/coverage by content, font and effective transform while keeping the resolved revision in the view label. On A(rev1) → B(rev2) → A(rev3), reused pixels must still identify rev3. Delayed lookup/decode cannot revert the current reference or label. Changes use existing event delivery; reopening re-resolves live references. SVG creation happens in Python before save, never in the canvas GUI. Export layout and canvas painting use the one production font manifest in the Shared chart font section of [[specs/svg-renderer]]; a missing or different registered identity produces a reasoned placeholder/refusal, never a canvas-specific font fallback.
