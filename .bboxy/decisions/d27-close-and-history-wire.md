---
id: "01M5KXVQH062KWHJ3E7B1761EC"
type: "decision"
number: 27
title: "Two C1c-2 wire rulings"
status: "decided"
decided: "2026-10-23"
operator: []
links:
  - { to: "[[2026-10-22-architecture-round-1#27]]", rel: "supersedes-as-index" }
created: "2026-10-23T08:25:56Z"
updated: "2026-10-24T00:10:38Z"
---
# Decision 27: Two C1c-2 wire rulings

**Two C1c-2 wire rulings (2026-10-23, Oracle).** (a) `GuiAction::UndoClose`
carries `layout_revision` for concurrent-layout conflict detection; the
close revision comes from the durable operation id. (b) `GuiReply::History`
is capture-sequence based: bounded `(sequence, bytes)` records, explicit
`Gap` entries in order, `next: Option<sequence>`; not a row pager. Rows
come from GUI-side replay through a `bbt-core` `Term`, later. Rejected:
a row pager on the wire (logic would keep a second parsed history per
client, against the viewport-grid-plus-ring memory model); undo without
a layout revision (a race the operator would see as a seat landing in
the wrong split).
