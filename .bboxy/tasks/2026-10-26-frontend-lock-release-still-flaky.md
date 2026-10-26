---
id: "01M5WW5XTNGJD1DFBH7RHTM547"
type: "task"
number: 360
title: "Frontend lock release still flaky after the explicit-unlock fix"
status: "open"
role: "executor"
owner: "Rook Ashwhisker"
verify: "cargo test -p bboxy450-term --lib cockpit::owners::tests::frontend_lock_refuses_a_live_owner_and_releases_on_departure"
created: "2026-10-26T19:49:42Z"
updated: "2026-10-26T19:49:42Z"
---
# Frontend lock release still flaky after the explicit-unlock fix

After 8755bd11 (explicit LOCK_UN), the test failed again in a presubmit: candidate 3f649261, log ~/.local/state/bboxy450-term/landing/presubmits/3f6492611df81f1981ed556bae1ff78ba7539e8a-4bbffec5/commands.log:418, panic at crates/bboxy450-term/src/cockpit/owners.rs:338 (104 passed, 1 failed). Quarantined meanwhile so app presubmits keep moving.

## Acceptance

Find the remaining mechanism, fix it, prove it with a deterministic regression, and remove the quarantine row. No timing tolerance.
