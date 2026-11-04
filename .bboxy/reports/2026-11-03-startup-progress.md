---
id: 4K8RB6KXVXWS38W3ME53HZGD1P
number: 5002234
type: report
title: Honest startup progress, centered badges and three minute watchdogs
author: "seat:01M6HCFJ8QFC193R49JW3G0VQX"
task: "[[tasks/2026-10-21-w3-2d-7-modules]]"
supersedes: "[[reports/2026-11-03-startup-checklist]]"
commits: ["ed0fa22ca6889fbe9b507aef5f1c220e90183f01"]
created: "2026-11-03T22:20:01Z"
verify:
  command: "make test-startup-modal"
  exit_code: 0
---
# Honest startup progress, centered badges and three minute watchdogs

The operator observed 0 of 0 records followed by a 30-second timeout and requested
three minutes, accurate progress, centered badges, a main push/build and no full CI.
The source and build remain in the main checkout; no worktree was created.

`store/events.rs:127` reports only the newly checked suffix after opening a saved
checkpoint. Its 0/0 is not the size of retained history. The UI now says the
checkpoint is current, identifies the last checked stream and reports only the
new records actually verified. It does not display a misleading 0-of-0 counter
or an aggregate history percentage.

`bbt-mux/src/logic.rs` now relays actual worker bootstrap operations through the
existing bounded readiness channel: registry, workspace records, messaging,
saved sessions, task state, deliveries and identity. Sessions/tasks carry actual
completed/total counts, throttled every 32 items and at completion. The app
shows this restoration on the fourth checklist step instead of leaving the
last checkpoint observation on screen throughout runtime initialization.

Both the app watchdog and worker readiness inactivity wait are 180 seconds;
real progress refreshes the waits. The CLI render-readiness allowance is 210
seconds so it does not abandon the requested startup window prematurely.
The badge uses a full-width centered text item in a vertically centered row;
the web reference uses the same geometry. Zero/unknown restoration totals show
the current operation without a numeric fraction or bar.

`make test-startup-modal` passed two native layout tests and two app state/
progress tests (440 unrelated app tests filtered). The layout test checks both
badge axes, narrow/normal widths, all stages and long diagnostics; no 0-of-0 text
is accepted. New workspaces/session-restoration preview states extend the
Boxybook catalogue to 96 variants. Their visual comparison and the production
build are run next; this record does not preclaim those results. No full CI or
full formal run is claimed under the operator's explicit exception.

Before these corrections, the release at ed0fa22c was built from main and
installed successfully after the operator approved Apple Distribution identity
B7B799761F58D5315C2D19162A44550E1677D325 for the same UQB3368A84 team. Bundle
identifier and designated Team requirement were verified unchanged. Its startup
subsequently failed with the observed 30-second watchdog, so that old successful
CLI start reply is not treated as proof of sustained backend readiness.
