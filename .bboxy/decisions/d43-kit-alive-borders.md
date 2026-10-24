---
id: "01M5NE355R0ED92M7GNKQMGDGR"
type: "decision"
number: 43
title: "Alive borders are a kit capability"
status: "decided"
decided: "2026-10-23"
operator:
  - "each block in our UI GPUGUI system should have ability to define this type of dynamic border and border swell and such! thats important, our blocks should be sexy out of the box!"
links:
  - { to: "[[2026-10-22-architecture-round-1#43]]", rel: "supersedes-as-index" }
  - { to: "[[rim-glow]]", rel: "decides" }
  - { to: "[[gui-components]]", rel: "decides" }
created: "2026-10-23T22:28:51Z"
updated: "2026-10-24T00:10:38Z"
---
# Decision 43: Alive borders are a kit capability

**Alive borders are a kit capability (operator, 2026-10-23, verbatim:
"each block in our UI GPUGUI system should have ability to define this
type of dynamic border and border swell and such! thats important, our
blocks should be sexy out of the box!").** Rim and swell become style
properties of any `bbt-ui` block (`Style.border { rim, swell }`,
replacing the pane-only `style.overrides.glow` that `Pane` sets at
`modules/pane.rs:398`), one rim pass over the block rect with per-block alive uniforms through the
R12-2 arenas; shader in `bbt-render`, style types in `bbt-ui-model`,
defaults in `bbt-ui` (panes, strips, seat rows, tabs, cards), nothing in
the app ([[d39-suite-is-the-product|#39]]). Only the pane gutter and stage inset follow rim size;
other blocks draw the outer half into their existing spacing, clipped,
so `rim-size-is-layout` stands ([[rim-glow]] §8, [[gui-components]]
§3.5). Lands as G5 after G3. Rejected: pane-only rim code copied per
block; letting row spacing follow the slider.
