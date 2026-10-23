---
id: 01M5MT43P4RP34NK00SFTBNBEH
type: invariant
title: the glow rim never covers a text cell
status: active
check: cargo test -p bbt-ui --locked --lib rim_never_covers_text -- --exact 2>&1 | grep -q 'ok. 1 passed; 0 failed'
cost: a glow that touches text makes the terminal unreadable at the edge; a glow that moves layout breaks parity for every pane case
links:
  - { to: "[[rim-glow]]", rel: constrains }
  - { to: "[[gui-components]]", rel: constrains }
created: 2026-10-23T16:39:50Z
updated: 2026-10-23T16:39:50Z
---
# The glow rim never covers a text cell

**Property.** For every pane case in the fixture catalogue and every depth in
`glow.depth_min..=glow.depth_max`, the glow band (rim plus bloom, the half-open strip
`[0, min(depth, padding))` just inside the pane border, clipped to the content
padding) does not intersect the pane's text cell grid rectangle: no cell pixel
receives glow at any depth ([[rim-glow]] §1, §3). The whole band is measured,
never the 2 px core alone.

**Check.** `cargo test -p bbt-ui --locked --lib rim_never_covers_text -- --exact 2>&1 | grep -q 'ok. 1 passed; 0 failed'`
— exit 0 held. The grep demands that exactly this named test ran and passed
(a cargo filter matching nothing exits 0). G1 creates the test; its commit
removes `slow` and `gate` from this file so it joins the default set.

**Cost of a violation.** A glow that touches text makes the terminal unreadable
at the edge, and a reader cannot tell a crest from a glyph.

**Evidence.** None yet; the test does not exist (pending G1).

Owner: Corvin Duskmantle <ARCHITECT>.
