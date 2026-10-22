---
id: 01M5JQE3ZYYGBJBQ0XHA094TPE
type: invariant
title: L7 modules draw only through components
status: active
check: cargo test -p bbt-ui --locked --test seat_rows module_sources_and_legacy_adapter_cannot_hand_draw_rows
cost: a module that hand-draws escapes theming and parity, and the layer cake drifts
links:
  - { to: "[[gui-components]]", rel: constrains }
  - { to: "[[open-source]]", rel: constrains }
created: 2026-10-22T21:14:24Z
updated: 2026-10-22T21:14:24Z
---
# L7 modules draw only through components

**Property.** No `Prim::`, `DrawList`, `DrawCtx`, `.rect(` or `.line(` inside
`crates/bbt-ui/src/modules/`. A module composes L6 components; only components
and atoms touch the draw list ([[gui-components]] §2c).

**Check.** `cargo test -p bbt-ui --locked --test seat_rows module_sources_and_legacy_adapter_cannot_hand_draw_rows`
(`crates/bbt-ui/tests/seat_rows.rs:42-71`, a source scan) — exit 0 held. Part of
`cargo test --workspace` in `sh scripts/ci-local.sh`.

**Cost of a violation.** A module that hand-draws escapes Style, theme and the
parity loop; the layer cake drifts and O2's split of domain modules from the
public library gets harder with every such line.

**Evidence.** Runs inside the workspace tests; last recorded by the Oracle's
local CI on 2026-10-22 (398 passed), not re-run by the Architect.

Owner: Corvin Duskmantle <ARCHITECT>.
