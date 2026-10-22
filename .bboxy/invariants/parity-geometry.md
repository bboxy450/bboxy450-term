---
id: 01M5JRD321TD30WH1MB71TSGVD
type: invariant
title: web and native geometry agree for every parity case
status: active
check: sh scripts/parity.sh
cost: a geometry miss is a layout bug shipped as a pixel difference; the library drifts from the lab it is matched against
links:
  - { to: "[[renderer]]", rel: constrains }
  - { to: "[[runbook-web-to-native]]", rel: spec }
  - { to: "[[gui-components]]", rel: constrains }
created: 2026-10-22T21:31:18Z
updated: 2026-10-22T21:31:18Z
---
# Web and native geometry agree

**Property.** For every parity case, the native layout geometry matches the web
lab's within the case policy ([[renderer]] §4.4a; `crates/bboxybook/src/parity.rs`
thresholds). Geometry is deterministic, so it is a hard gate. The pixel review
residue stays judgement: a harness reads the images and advises; it never blocks
(P1, decision #16: hard gates and judgement are separate instruments).

**Check.** `sh scripts/parity.sh` — runs `bboxybook compare --strict-geometry`
(`scripts/parity.sh:14`) — exit 0 held, non-zero on any geometry failure or a
failed web capture. Slow and needs Chrome: run under `sh scripts/ci-local.sh
--parity` when lab, design, UI or renderer changed; a scoped run sets
`BBOXYBOOK_GALLERY`.

**Cost of a violation.** A geometry miss is a layout bug that ships as a pixel
difference nobody can explain; the library drifts from the lab it is matched
against, and every later layer inherits the drift.

**Evidence.** P1 landed on main at 8e66b3a (Velvet, accepted by the Oracle);
the Architect has not run the full parity suite.

Owner: Corvin Duskmantle <ARCHITECT>.
