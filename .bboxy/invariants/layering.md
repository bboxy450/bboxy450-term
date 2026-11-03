---
id: 01M5JFWDVFVS0KY552YYZKWRYM
number: 132
type: invariant
title: crate layering — every crate depends only on crates below it
status: active
check: sh scripts/layering.sh
cost: an upward edge welds a library to the product; untangling it later costs the velocity we protect now
links:
  - { to: "[[open-source]]", rel: constrains }
  - { to: "[[2026-10-22-architecture-round-1]]", rel: decision }
  - { to: "scripts/layering.sh", rel: check }
created: 2026-10-22T19:02:24Z
updated: "2026-10-24T19:15:36Z"
---
# Crate layering

**Property.** Every workspace crate depends only on crates earlier in the order
`bboxy450-keychain-macos bboxy450-keychain-linux bboxy450-keychain-windows bboxy450-keychain-file bboxy450-keychain bboxy450-desktop-notifications-macos bboxy450-desktop-notifications-windows bboxy450-desktop-notifications-linux bboxy450-desktop-notifications bboxy450-usage bboxy450-tlc bboxy450-yarn-macros bboxy450-yarn bboxy450-rt bboxy450-term-core bboxy450-harness-wake-up bboxy450-mermaid bboxy450-term-firehose bboxy450-ui-core bboxy450-render bboxy450-ui bboxy450-doc bboxy450-term-desktop-core bboxy450-term-desktop-macos bboxy450-term-desktop bboxy450-harness bboxy450-book bboxy450-harness-host bboxy450-plugin bboxy450-term-plugin-sdk bboxy450-term-plugin-fixture bboxy450-term-data_source-gitkb bboxy450-term-proto bboxy450-term-mux bboxy450-term` (the ORDER line of `scripts/layering.sh` is the truth; this prose follows it).
Any prefix of that order builds and publishes without the rest. A new crate gets
its place in the order in the commit that adds it.

**Check.** `sh scripts/layering.sh` — exit 0 held, exit 1 an upward edge or a
crate missing from the order. First step of `sh scripts/ci-local.sh`, the local
CI that runs before every handoff and merge (operator, 2026-10-22: no cloud
CI during heavy development; GitHub runs on manual dispatch only).

**Cost of a violation.** An upward edge welds a library to the product. Every
bboxy450 crate goes public eventually (operator, 2026-10-22); untangling it then
costs the velocity we are protecting now, at a worse rate.

**Advisory, not a gate.** Domain words (seat, swarm, harness) inside library
crates. bbt-ui-model, bbt-ui and bboxybook hold domain types and L7 modules by
design until O2 ([[open-source]] §1). Not grepped: comments and test strings make
a word gate noisy, and a noisy gate gets ignored.

Owner: Corvin Duskmantle <ARCHITECT>. Decided 2026-10-22, [[2026-10-22-architecture-round-1]] D8–D9.
