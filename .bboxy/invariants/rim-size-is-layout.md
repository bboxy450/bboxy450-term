---
id: 01M5NC2YESME91PR6YHH2T70KF
type: invariant
title: rim size moves only the gutter and the stage inset
status: pending
check: cargo test -p bbt-ui --locked --lib rim_size_moves_only_gutter_and_inset -- --exact 2>&1 | grep -q 'ok. 1 passed; 0 failed'
cost: a rim slider that moves anything but the gutter and the inset breaks strict geometry parity for every pane case and turns a glow setting into an unbounded layout setting
links:
  - { to: "[[rim-glow]]", rel: constrains }
  - { to: "[[glow-never-moves-layout]]", rel: bounds }
created: 2026-10-23T21:53:46Z
updated: 2026-10-23T21:53:46Z
---
# Rim size moves only the gutter and the stage inset

**Property.** For every fixture scene and every rim size R in
`glow.rim_min..=glow.rim_max`, the solved layout differs from the layout at the
default R = 4 only in `layout.gutter = R` between panes and the stage inset
R/2 at the window edge; pane content padding, strips, the sidebar and every
cell grid are unchanged, and the change arrives through relayout, never at
paint ([[rim-glow]] §2, §3; operator 2026-10-23, "gap grows with the rim, outer
halves meet").

**Check.** `cargo test -p bbt-ui --locked --lib rim_size_moves_only_gutter_and_inset -- --exact 2>&1 | grep -q 'ok. 1 passed; 0 failed'`
— exit 0 held. The grep demands that exactly this named test ran and passed.
G3 creates the test; its commit removes `slow` and `gate` from this file so it
joins the default set.

**Cost of a violation.** A rim slider that moves anything but the gutter and the
inset breaks strict geometry parity for every pane case and turns a glow setting
into an unbounded layout setting.

**Evidence.** None yet; the test does not exist (pending G3).

Owner: Corvin Duskmantle <ARCHITECT>.
