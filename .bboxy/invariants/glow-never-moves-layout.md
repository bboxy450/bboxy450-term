---
id: 01M5MT43P4APAE4G7AKFKR5GRR
type: invariant
title: glow settings never move layout
status: active
slow: true
gate: pending-g1
check: cargo test -p bbt-ui --locked --lib glow_never_moves_layout -- --exact 2>&1 | grep -q 'ok. 1 passed; 0 failed'
cost: a glow that touches text makes the terminal unreadable at the edge; a glow that moves layout breaks parity for every pane case
links:
  - { to: "[[rim-glow]]", rel: constrains }
  - { to: "[[gui-components]]", rel: constrains }
created: 2026-10-23T16:39:50Z
updated: 2026-10-23T16:39:50Z
---
# Glow settings never move layout

**Property.** The solved layout of every fixture scene (pane rects, dividers,
sidebar, strips) is byte-identical with glow off, glow on, and every rim width
in `glow.rim_min..=glow.rim_max` ([[rim-glow]] §2, §3). Glow is paint, never
geometry; the gutter is a fixed token.

**Check.** `cargo test -p bbt-ui --locked --lib glow_never_moves_layout -- --exact 2>&1 | grep -q 'ok. 1 passed; 0 failed'`
— exit 0 held; same named-test rule as above. G1 creates it.

**Cost of a violation.** A slider that moves panes breaks strict geometry parity
for every pane case at once and makes the glow setting a layout setting.

**Evidence.** None yet (pending G1).

Owner: Corvin Duskmantle <ARCHITECT>.
