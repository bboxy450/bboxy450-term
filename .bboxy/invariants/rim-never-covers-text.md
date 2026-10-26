---
id: 01M5MT43P4RP34NK00SFTBNBEH
number: 153
type: invariant
title: the glow rim never covers a text cell
status: active
check: cargo test -p bboxy450-ui --locked --lib rim_never_covers_text -- --exact 2>&1 | grep -q 'ok. 1 passed; 0 failed'
cost: a glow that touches text makes the terminal unreadable at the edge; a glow that moves layout breaks parity for every pane case
links:
  - { to: "[[rim-glow]]", rel: constrains }
  - { to: "[[gui-components]]", rel: constrains }
created: 2026-10-23T16:39:50Z
updated: 2026-10-23T21:53:46Z
---
# The glow rim never covers a text cell

**Property.** For every pane case in the fixture catalogue and every rim size in
`glow.rim_min..=glow.rim_max`, the glow band (the rim's outer half `[−R/2, 0)`
outside the border, its inner half and the bloom, the half-open strip
`[0, min(8, padding))` inside, clipped to the content padding) does not
intersect any pane's text cell grid rectangle: the outer half lies in the gutter
or the stage inset, which hold no content, and no cell pixel receives glow
([[rim-glow]] §1, §3). The whole band is measured,
never the rim core alone (`glow.rim`, 4 px since 2026-10-23; the width is a token, not a fixed 2).

**Check.** `cargo test -p bboxy450-ui --locked --lib rim_never_covers_text -- --exact 2>&1 | grep -q 'ok. 1 passed; 0 failed'`
— exit 0 held. The grep demands that exactly this named test ran and passed
(a cargo filter matching nothing exits 0). G1 creates the test; its commit
removes `slow` and `gate` from this file so it joins the default set.

**Cost of a violation.** A glow that touches text makes the terminal unreadable
at the edge, and a reader cannot tell a crest from a glyph.

**Evidence.** G1 (f403bd9): `crates/bbt-ui/src/modules/pane.rs:656`, inside band only; G3 extends it to the outer half.

Owner: Corvin Duskmantle <ARCHITECT>.
