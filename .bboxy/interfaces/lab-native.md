---
id: 01M5WG2WS8HHK493AF4YYDA8TK
type: interface
number: 352
title: "ICD: lab ↔ native renderer"
status: baselined
links:
  - { to: "[[interfaces]]", rel: index }
created: "2026-10-26T16:18:20Z"
updated: "2026-10-26T16:18:20Z"
---
# ICD: lab ↔ native renderer

| | |
|---|---|
| Side A | the web lab (`lab/web`, Creativity) |
| Side B | the native renderer and its catalogue (`bbt-render`, `bbt-ui`, `bboxybook`) |
| The agreement | [[renderer]]; [[gui-components]]; [[landing]] §3 parity |
| Amended by | #341 one cut for a contract and its first implementation; the native-input and generated-header rules of [[landing]] §2 and §3 |
| What crosses | catalogue cases from `gallery::parity_cases()`; design tokens from `design/`; generated lab files naming their sources |
| Timing | parity runs in presubmit for the cases a change selects and in full in postsubmit |
| States and lifecycle | a lab change compares against the native renders stored by the last green postsubmit; a native-input change builds the catalogue fresh |
| Failure | geometry differences fail; pixel differences under policy advise |
| Versioning | a generated file names its generator and every source on line 1 |
| Verified by | invariants parity-geometry, modules-through-components; `scripts/generated-sources.sh`; promises R1 to R6 |
| Open | none |

A change to this interface names both sides, the reason and the verification it needs, and lands only with both sides' owners agreeing. A question an Executor asks about this boundary is a missing line here: the answer is written into this table, not only sent.

Owner: Corvin Duskmantle <ARCHITECT>.
