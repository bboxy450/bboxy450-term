---
id: "01M5NE355R7A5K7B3N13DJQTNF"
type: "decision"
number: 42
title: "The focus swell replaces the locate ripple"
status: "decided"
decided: "2026-10-23"
operator:
  - "can we change focus circle effert to just using silk border glow, and pulsing it to say 40pxs and then back, you know? without changing spacing between blocks, as an indication of me focusing it"
  - "40px swell is fucking perfect!"
  - "THIS IS SO SEXY I FUKCING LOVE ALL OF IT GO MAKE THIS HAPPEN!"
links:
  - { to: "[[2026-10-22-architecture-round-1#42]]", rel: "supersedes-as-index" }
  - { to: "[[rim-glow]]", rel: "decides" }
created: "2026-10-23T22:28:51Z"
updated: "2026-10-24T00:10:38Z"
---
# Decision 42: The focus swell replaces the locate ripple

**The focus swell replaces the locate ripple (operator, 2026-10-23,
verbatim via Wren: "can we change focus circle effert to just using
silk border glow, and pulsing it to say 40pxs and then back, you know?
without changing spacing between blocks, as an indication of me
focusing it", "40px swell is fucking perfect!").** On focus the
block's silk swells 40 px past its rim and settles in 0.9 s on a sine
envelope, painted over gutter and neighbours, never layout; zero at
rest so the rim invariants are untouched; the lab reference sets the
alpha curve and records the peak weight over a neighbour's text cell
([[rim-glow]] §7; Wren's reference df6d58e: at most 0.48 over the
nearest cell). Operator on that reference, verbatim: "THIS IS SO SEXY I
FUKCING LOVE ALL OF IT GO MAKE THIS HAPPEN!"; top priority after the
in-flight gates (Oracle, same evening). Rejected: keeping the ripple beside the swell (two
focus signals); a swell that pushes neighbours (layout following focus).
