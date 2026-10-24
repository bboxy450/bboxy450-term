---
id: "01M5MR66GGNFMSPY3RJHZXH2TD"
type: "decision"
number: 33
title: "Vendor the operator's reactor! macro as bbt-yarn"
status: "decided"
decided: "2026-10-23"
operator:
  - "if yes just create this as bboxy450-crate, name it somehow sexy"
  - "document this design into our docs first; maybe useful now or later"
links:
  - { to: "[[2026-10-22-architecture-round-1#33]]", rel: "supersedes-as-index" }
  - { to: "[[yarn]]", rel: "decides" }
created: "2026-10-23T16:06:02Z"
updated: "2026-10-24T00:10:38Z"
---
# Decision 33: Vendor the operator's reactor! macro as bbt-yarn

**Vendor the operator's `reactor!` macro as `bbt-yarn` (operator,
2026-10-23: "if yes just create this as bboxy450-crate, name it somehow
sexy").** Yes, because the two scale bugs of 2026-10-22 were arbitration
policy hidden in hand-written loops and this macro makes that policy a
compiler-checked declaration; starvation waivers are decision records in
code. Spec [[yarn]]: vendored unchanged from `Gonzih/kittens-old`
293e134 (the operator's own code; provenance in the README; license
**MIT OR Apache-2.0**, set by the operator as author on 2026-10-23), first in the
layering order, adopted one loop at a time starting with the app IO task
where operator input must never starve behind deltas, with a no-raw-
select source scan per adopted file. Rejected: depending on the
archived crate (a retired identity's archive is not a supply chain);
adopting everywhere at once (logic's Worker loop is mid-churn and sync);
renaming the grammar (the KTR codes and agent guide are the value).
Names rejected: litter, ears, pounce, purr. Timing (operator, same
day): "document this design into our docs first; maybe useful now or
later": the design is in [[yarn]] §0; Y1 runs when a loop needs it or
an Executor is free, not ahead of the cockpit tracks.
