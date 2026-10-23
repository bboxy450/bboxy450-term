---
id: 01M5MT43P4APAE4G7AKFKR5GRR
type: invariant
title: glow depth never moves layout
status: active
check: cargo test -p bbt-ui --locked --lib glow_never_moves_layout -- --exact 2>&1 | grep -q 'ok. 1 passed; 0 failed'
cost: a glow that touches text makes the terminal unreadable at the edge; a glow that moves layout breaks parity for every pane case
links:
  - { to: "[[rim-glow]]", rel: constrains }
  - { to: "[[gui-components]]", rel: constrains }
created: 2026-10-23T16:39:50Z
updated: 2026-10-23T21:53:46Z
---
# Glow depth never moves layout

**Property.** At a fixed rim size, the solved layout of every fixture scene
(pane rects, dividers, sidebar, strips) is byte-identical with glow off, glow
on, and every depth in `glow.depth_min..=glow.depth_max` ([[rim-glow]] §2, §3).
Depth is paint, never geometry. Rim size is the one glow token that is layout
by design (gutter = R, inset = R/2, operator 2026-10-23), and
[[rim-size-is-layout]] bounds exactly what it may move; this invariant was
"glow settings never move layout" until that pick narrowed it to depth.

**Check.** `cargo test -p bbt-ui --locked --lib glow_never_moves_layout -- --exact 2>&1 | grep -q 'ok. 1 passed; 0 failed'`
— exit 0 held; same named-test rule as above. G1 creates it.

**Cost of a violation.** A slider that moves panes breaks strict geometry parity
for every pane case at once and makes the glow setting a layout setting.

**Evidence.** G1 (f403bd9): `crates/bbt-ui/src/modules/pane.rs:714`, in the default invariant set.

Owner: Corvin Duskmantle <ARCHITECT>.
