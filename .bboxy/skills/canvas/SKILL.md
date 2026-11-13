---
type: skill
name: canvas
description: 'Draw and inspect a kept canvas with styles, routed connectors, free paths and layout help: use when asked to draw, doodle, sketch, diagram, explain a system visually or show a plan on a canvas.'
id: 01M71VMS8QQ7FHX9SA7YC7Q7K3
updated: 2026-11-12T16:49:57.193Z
number: 1794274420276184
created_by: seat:01M788X1K2MZH0V5CH8RDS2EYS
updated_by: seat:01M788X1K2MZH0V5CH8RDS2EYS
authorship:
  signer: seat:01M788X1K2MZH0V5CH8RDS2EYS
  kind: seat
  public_key: QirMRCVq4MGjLqozHY5Hg1PxrBTQC7432d3nv2JAmW0=
  process_generation: '1'
provenance:
  digest: sha256:66974101a595f65afa2b8eee7007cfc7e797cebccd0a7d97f8290cbe88e67f6e
  frontmatter_digest: sha256:11b5456e156c2f78419dd8e2370c9db2c5308acfd57ed9c066090f77a831cb7f
  signer: seat:01M788X1K2MZH0V5CH8RDS2EYS
  sig: bAxGrjfGSWxCvazV65FqdGgKdTIf8o6mRwOcTV0ZN+RgoUlnE3knRaFU0J54yCN9a3jCGJ1xe0nb04OxtHlIDQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/82807
  event:
    id: 01M78AN00HG6DW0HX9KEYP3RNN
    sequence: 82807
    hash: 32d60880915991795c66c77099f74662503dcfac4e3dbf7431c40faead2f3adb
---
# Canvas

Spec: [[canvas]]. A canvas is a numbered record whose append-only log is the picture. You say what things are and, when it matters, how they look; the app renders with its GPU kit in both themes.

## When

Use a canvas for system sketches, explanations, plans, comparisons and anything the operator asked to see drawn ("draw", "doodle", "sketch", "put this on a canvas"). For a diagram inside a document, use Mermaid in the document instead.

## The loop

1. `canvas_open` with a title (and tags) to create, or a number/id to reopen. Keep the returned id.
2. `canvas_draw` in **small batches** while you reason, so the operator watches it build. Give every element a **stable id** you choose (`supervisor`, `link-cockpit-supervisor`); `patch`, `change` or `remove` by that id later. A batch is one set: list a frame before the members it groups if you like.
3. `canvas_read` with `elements: false` for the fitted `extent` and each frame's `members` box, so you place the next element without guessing. `image: true` renders the same fit the GUI shows. Look at it; fix overlaps and unreadable labels with `patch`.
4. `canvas_list` finds existing canvases; reuse one rather than making a near-duplicate. `canvas_close` retires one; `canvas_restore` brings it back.

## Elements

| type | fields | notes |
|---|---|---|
| `shape` | `bounds {origin,size}`, `shape`, `label` | `box`, `rounded`, `ellipse`, `diamond`, `cylinder`, `actor`, `hexagon`, `triangle`, `parallelogram`, `star`, `cloud`. Labels wrap to the box; `\n` breaks lines. |
| `text` | `bounds`, `text`, `role` | `heading`, `paragraph`, `note`. Paragraphs and notes wrap to their bounds. A styled text draws as a card. |
| `frame` | `bounds`, `name`, `members` | A named region; zoom target; `canvas_read` reports its members' box. |
| `connector` | `from`, `to`, `label`, `arrowheads [bool,bool]`, `dashed`, `route`, `from_side`, `to_side`, `waypoints`, `label_at` | `route`: `straight` (default), `orthogonal` (elbows that avoid other shapes), `curved` (one smooth S leaving and entering along the ports). Sides: `top`, `right`, `bottom`, `left`; omit for automatic. `waypoints` are world points the route passes through. `label_at` is 0 at `from`, 1 at `to`, default 0.5. Labels sit beside the line and move off other labels and shapes. |
| `path` | `points` (2 to 4096), `closed`, `smooth`, `arrowheads` | A free polyline or polygon: hand marks, outlines, regions. A closed path is filled and draws under shapes; an open one draws on top. Its bounds are its points' box, so connectors can attach to it. |

Every element also carries `tags`, optional `kind` (`service`, `store`, `queue`, `person`, `external`), `emphasis`, `status` (`normal`, `waiting`, `error`, `done`) and an optional `style`.

## Style

Meaning sets the house look: kind and emphasis pick the accent, status picks waiting, error or done colours, `dashed` reads as "waits on". Add a `style` only when the picture needs it. Every field is optional:

| field | value |
|---|---|
| `fill`, `stroke`, `text`, `glow` | a colour: theme token `accent`, `fg`, `dim`, `surface`, `bg`, `waiting`, `error`, `done`; named hue `red`, `orange`, `yellow`, `green`, `teal`, `blue`, `indigo`, `violet`, `pink`, `white`, `black`; or `#rgb`, `#rrggbb`, `#rrggbbaa` |
| `stroke_width` | 0 to 64, logical pixels at zoom one; scales with the drawing |
| `opacity` | 0 to 1 for the whole element |
| `glow` | a soft halo in that colour: use it for the thing you are talking about |
| `shadow` | `true` lifts a box off the surface |
| `radius` | corner radius for box-like shapes |
| `font_size`, `weight` (100 to 900), `italic`, `mono`, `align` (`left`, `center`, `right`) | text inside the element |

Theme tokens follow light and dark; literal hues do not, so prefer tokens for anything that must read in both.

## Ops

| op | fields | does |
|---|---|---|
| `add` | `element` | adds; a duplicate id is refused |
| `change` | `element` | replaces the whole element, keeping its identity |
| `patch` | `id` plus any of `bounds`, `label`, `style`, `status`, `emphasis` | changes only those fields. `label` is a shape's label, a text's body, a frame's name or a connector's label. `bounds` on a path moves and scales its points. An empty `style` clears the style |
| `remove` | `id` | removes; a referenced element is refused until its connectors and frames let go |
| `tag` | `id`, `tag` | adds one tag |

## Rules

- On a retry after an unknown outcome, reuse the original `op_id`; the store applies a batch once.
- At an element, point or rate cap the tool refuses and names the cap; stop and tell the operator rather than splitting the canvas silently.
- The operator's moves and edits do not wake you; read the canvas before redrawing near something they touched.
- Loose `color`, `font` or `weight` fields outside `style` are refused; the look lives in one place.

## A good diagram in one pass

Lay shapes on a grid of about 160 by 110 world units; use frames for columns; route connectors `orthogonal` between columns and `curved` for feedback; put the label at `label_at: 0.3` when two connectors share a gap; glow the element the explanation is about; read `extent` and the frames' `members` boxes before adding the next column.
