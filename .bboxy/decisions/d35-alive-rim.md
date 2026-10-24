---
id: "01M5MTA4B0ZSQF2P1ZRW74YFAT"
type: "decision"
number: 35
title: "The pane glow is the alive rim inside the border"
status: "amended"
decided: "2026-10-23"
operator:
  - "I ACTUALLY LOVE RIM EFFECT"
  - "8px alive rim is the thing we want"
  - "go with it"
  - "yeah what we have as is great, just want 2extra pixels of silk glowness, just that"
  - "i know every pixel matters, but thats visual indicator, ad i want it to be slightly more pronounced"
  - "2px can come 1px on the inside, 1px on the outside so border feels more alive"
  - "rim size configurable from 2 to 8 px, half of size on each side!"
  - "do half, half, maybe? for balance"
  - "half / half / gap grows with the rim, outer halves meet this is the way to go!"
  - "solidify and commit and make sure it gets implemented in real native ui"
  - "make sure it gets implemented in real native ui"
links:
  - { to: "[[2026-10-22-architecture-round-1#35]]", rel: "supersedes-as-index" }
  - { to: "[[rim-glow]]", rel: "decides" }
created: "2026-10-23T16:43:08Z"
updated: "2026-10-24T00:10:38Z"
---
# Decision 35: The pane glow is the alive rim inside the border

**The pane glow is the alive rim inside the border (operator,
2026-10-23, verbatim: "I ACTUALLY LOVE RIM EFFECT", "8px alive rim is
the thing we want", "go with it").** Spec [[rim-glow]]: a fixed 2 px rim
plus a bloom to depth L (the Settings value, 2–8, default 8), two
counter-moving waves snapped to the perimeter, seat colour hue-rotating
along the perimeter with a white mix on crests, pulse by activity, the
lab's activity table, focus ×1.3; the outside 8 px silk lane retires,
the gutter becomes a fixed 2 px, the Settings row keeps 2–8 px and now
means rim depth, drawn inside the pane padding, never over text; two
invariants with named tests; the `materials.wgsl` lane pass keeps the
silk machinery and changes geometry to the inside perimeter strip, no
blur; R12-3 retargeted to the rim pass; R4c glow numbers re-measured.
First written for the plain 2 px variant; superseded the same evening
by the operator's choice of `alive-rim` in Wren's lab. Cell edge rule:
the band is clipped to the pane padding and the invariant measures the
whole band; padding does not grow; rejected a core-only invariant
(permits bloom over text) and growing the padding (layout following a
glow setting, against the operator's tighter panes). Rejected: keeping
the lane as an option (two glow models in parity); letting depth move
the gutter; drawing over the cell inset. **Amended 2026-10-23, evening**
(operator via Wren, verbatim: "yeah what we have as is great, just want
2extra pixels of silk glowness, just that" / "i know every pixel
matters, but thats visual indicator, ad i want it to be slightly more
pronounced"): `glow.rim` 2 → 4 as a token the lab and native shaders
read as a uniform; the bloom runs from the rim to L; the band stays
`[0, min(depth, padding))`, so both invariants hold unchanged and no
geometry moves; a depth below the rim width clips the rim. Rejected:
growing depth past the 8 px padding (the cell edge rule); a literal 4
in any shader; raising `depth_min` (the operator may want a thin rim).
**Amended again within the hour** (operator, 2026-10-23, verbatim and in order: "2px can come 1px on the inside, 1px on the outside so border feels more alive", "rim size configurable from 2 to 8 px, half of size on each side!", "do half, half, maybe? for balance", "half / half / gap grows with the rim, outer halves meet this is the way to go!", "solidify and commit and make sure it gets implemented in real native ui"), from
Wren's demo of three shapes: the rim is centred on the border, R/2 out
and R/2 in, `layout.gutter = R` so neighbouring outer halves meet, the
stage inset is R/2, the Settings row is "Rim size" 2–8 default 4
(unstated; Wren's and the Architect's default, the "2 extra pixels"
over the old 2), depth fixed at 8. Rim size is a layout setting by
design: `glow-never-moves-layout` narrows to depth and the new
`rim-size-is-layout` bounds what the slider may move (gutter and inset
only); the fixed 2 px gutter yields. Rejected by the operator's eye: (B)
half/half in a fixed 2 px gutter with overlapping halves; (C) 1 px out
and the rest in. Native lands it as G3 ("make sure it gets implemented
in real native ui").
