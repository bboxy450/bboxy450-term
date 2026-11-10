---
type: skill
name: canvas
description: 'Draw and inspect a kept semantic canvas: use when asked to draw, doodle, sketch, diagram or explain a system visually, or to show a plan on a canvas.'
id: 01M71VMS8QQ7FHX9SA7YC7Q7K3
updated: 2026-11-10T04:32:15.594Z
created: 2026-11-10T04:32:15.594Z
number: 1794274420276184
created_by: seat:01M71RQWJ3C0DW2E1QHRTKKT3V
updated_by: seat:01M71RQWJ3C0DW2E1QHRTKKT3V
authorship:
  signer: seat:01M71RQWJ3C0DW2E1QHRTKKT3V
  kind: seat
  public_key: nbRk4SVAPLzaJtFzHXqpxa4bNlUvBeSuNZNXX7KMBSk=
  process_generation: '1'
provenance:
  digest: sha256:b278768238adf1c639f8caa2dab636a63dd3b5b222107deda1aa4e7e2a2fe7fc
  frontmatter_digest: sha256:cc3cf9351867f0c68c75de528a9b3bd7c0579c729dfe5e4039080de61fca938d
  signer: seat:01M71RQWJ3C0DW2E1QHRTKKT3V
  sig: qZEu03esTiS4RX9u5Rdkfb9fAT0n6e7zgqBtVipy8ZSeEp5HCL4vuqRVqAhrohFqGIOX04Y4tVjArwXopwelDQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151377
  event:
    id: 01M71VMSYS6765CPGQAQVQ24N1
    sequence: 151377
    hash: 2c320de17f92c2a467511ceecd4700f3dcb1d1d3fcc2b6f9aad52ab2b3e90ce2
---
# Canvas

Spec: [[canvas]]. A canvas is a numbered record whose append-only log is the picture. You say what things are; the app decides how they look.

## When

Use a canvas for system sketches, explanations, plans and comparisons the operator asked to see drawn ("draw", "doodle", "sketch", "put this on a canvas"). For a diagram inside a document, use Mermaid in the document instead.

## How

1. `canvas_open` with a title (and tags) to create, or a number/id to reopen. Keep the returned id.
2. `canvas_draw` in **small batches** (one to a few elements) while you reason, so the operator watches it build. Give every element a **stable id** you choose (`supervisor`, `link-cockpit-supervisor`); change or remove by that id later.
3. Group related elements in a frame (`option-a`, `inside-supervisor`).
4. `canvas_read` with `image: true` after a few batches. Look at the image: fix overlaps, unreadable labels and crossing connectors with further `canvas_draw` changes.
5. `canvas_list` finds existing canvases; reuse one rather than making a near-duplicate.

## Rules

- Send meaning only: shape, kind (service, store, queue, person, external), emphasis, status, dashed for "waits on". **Never send colours, fonts or line weights**; they are refused.
- On a retry after an unknown outcome, reuse the original `op_id`; the store applies a batch once.
- At an element or rate cap the tool refuses and names the cap; stop and tell the operator rather than splitting the canvas silently.
- The operator's moves and edits do not wake you; read the canvas before redrawing near something they touched.
