---
id: 01M5JRD321TD30WH1MB71TSGVD
type: invariant
title: web and native geometry agree for every parity case
status: active
slow: true
gate: parity
check: sh scripts/parity.sh
cost: a geometry miss is a layout bug shipped as a pixel difference; the library drifts from the lab it is matched against
links:
  - { to: "[[renderer]]", rel: constrains }
  - { to: "[[runbook-web-to-native]]", rel: spec }
  - { to: "[[gui-components]]", rel: constrains }
created: 2026-10-22T21:31:18Z
updated: 2026-10-23T23:17:54Z
---
# Web and native geometry agree

**Property.** For every case in the canonical catalogue (1704 current screens at
P3, dd47f4e, 2026-10-23), the native layout geometry matches the web lab's within
the case policy ([[renderer]] §4.4a, including the 2026-10-23 rule that mark-only
atom cases are vectors; `crates/bboxybook/src/parity.rs` thresholds):
zero mismatches **and zero unproven cases** (a case whose geometry could not be
measured fails, `parity.rs:242, 669-670`). Any visual change gates on it. Geometry is deterministic, so it is a hard gate. The pixel review
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

**Evidence.** P1 at 8e66b3a made geometry failures fail; P3 at dd47f4e widened the
gate to the canonical catalogue: 1704 cases, 1600 measured parts, 0 unproven,
0 mismatches (Velvet, accepted by the Oracle, 2026-10-23). Invocation:
`sh scripts/ci-local.sh --parity`. The Architect has not run the full suite.

Owner: Corvin Duskmantle <ARCHITECT>.
