---
id: 01M5WG2WS8HHK493AF4YYDA8TK
type: interface
number: 352
title: "ICD: lab ↔ native renderer"
status: baselined
links:
  - { to: "[[interfaces]]", rel: index }
created: "2026-10-26T16:18:20Z"
updated: "2026-11-03T21:25:00Z"
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

## Startup checklist

`lab/web/startup.html` previews the app's public startup observations. Its four
ordered steps are supervisor connection, runtime start, signed-history verification
and session restoration/cockpit attachment. The current phase marks exactly one active row; prior
stages are complete and later stages remain pending. Supervisor adoption stays
on step one. Reconnection starts at step one; a runtime refresh starts at step
two. A sticky failure marks the current row failed and retains the exact error,
available history counts and log paths. Retry clears that progress through the
existing startup state machine. Phase time is labelled as time on this step.
No aggregate percentage or estimated duration is invented. Verification reports
the newly checked suffix of the last stream; zero means its checkpoint is current,
not that retained history is empty. Runtime restoration reports its actual operation
and session/task completed counts. A bar appears only for a known restoration
total; unknown totals display the operation. Both startup inactivity guards allow
180 seconds and reset on real progress; the CLI readiness allowance is 210 seconds.

Both sides receive `bbt_ui::modules::startup::Startup` display data. The web
constructor builds its own UI tree; Boxybook exports the public data on that
tree, then calls the production native composition instead of replaying web
child positions. Neutral native action-name wrappers are omitted from geometry
diagnostics. `make parity-startup` runs the existing strict geometry/pixel policy
for all twelve states, four densities and both themes. `make test-startup-modal`
checks live phase projection, sticky failure/retry, the full checklist and
wrapped diagnostics at 320 and 560 logical pixels, with centered number/check
badges. Process ownership and input routing are unchanged.
