---
id: 01M5JFWDVFVS0KY552YYZKWRYM
type: invariant
title: crate layering — every crate depends only on crates below it
status: active
links:
  - { to: "[[open-source]]", rel: constrains }
  - { to: "[[2026-10-22-architecture-round-1]]", rel: decision }
  - { to: "scripts/layering.sh", rel: check }
created: 2026-10-22T19:02:24Z
updated: 2026-10-22T19:02:24Z
---
# Crate layering

**Property.** Every workspace crate depends only on crates earlier in the order
`bbt-rt bbt-core bboxy-firehose bbt-ui-model bbt-render bbt-ui bboxybook bbt-mux bboxy450-term`.
Any prefix of that order builds and publishes without the rest. A new crate gets
its place in the order in the commit that adds it.

**Check.** `sh scripts/layering.sh` — exit 0 held, exit 1 an upward edge or a
crate missing from the order. Runs in CI on every push.

**Cost of a violation.** An upward edge welds a library to the product. Every
bboxy450 crate goes public eventually (operator, 2026-10-22); untangling it then
costs the velocity we are protecting now, at a worse rate.

**Advisory, not a gate.** Domain words (seat, swarm, harness) inside library
crates. bbt-ui-model, bbt-ui and bboxybook hold domain types and L7 modules by
design until O2 ([[open-source]] §1). Not grepped: comments and test strings make
a word gate noisy, and a noisy gate gets ignored.

Owner: Corvin Duskmantle <ARCHITECT>. Decided 2026-10-22, [[2026-10-22-architecture-round-1]] D8–D9.
