---
id: 01M5QNKAYKD6J9NEEFGFWJ0PSQ
type: invariant
number: 283
title: "the e2e night runs unattended: no operator input, no login-Keychain access, no window, no build or signature"
status: active
slow: true
gate: night
check: sh scripts/e2e-unattended.sh
cost: a test that needs a human present is a meeting; the operator worked fourteen hours for a night that should have run itself
links:
  - { to: "[[d282-app-tests-itself-unattended]]", rel: decision }
  - { to: "[[e2e-acceptance]]", rel: constrains }
  - { to: "[[id1-seat-chain]]", rel: constrains }
  - { to: "[[ctl-opt-in]]", rel: sibling }
created: "2026-10-24T19:18:29Z"
updated: "2026-10-25T12:18:12Z"
---
# The e2e night runs unattended

**Property.** `scripts/e2e-night.sh`, run with no operator present, creates a
throwaway test account for the run (its own root in file custody under the
run's private directory, never the operator's account or identity), starts the
identity-signed bundle the day produced in headless mode under that account, spawns the worker testers, delegates every e2e record, answers known
harness prompts through recipes, collects every `verify_result`, updates the
checklist, and exits `0` when the checklist is green or every non-green row
names its open task; during the run no window is created, no login-Keychain
item is read (the custody backend in use is `file` and the journal shows no
`keychain` backend call), no prompt waits on a human, no operator act of any kind is needed, every
verdict comes from the runner's assertions and never from an agent's report, and
nothing is built or
signed ([[d282-app-tests-itself-unattended]]). Every record the night
writes carries `issued_by: test-account:<run>`.

**Check.** `sh scripts/e2e-unattended.sh` — exit 0 held. The script builds
nothing and, for every `###` group of [[e2e-acceptance]] §1, runs
`bboxy450-term e2e night <group>` with stdin closed (`</dev/null`), no window
and a time bound per group, then asserts three things and only these:
(1) **it ends by verdict**: every group exits `0` or `1`, never `2` (refused)
and never killed at the bound, because a run that is still alive at the bound
waited on someone; (2) **every promise is written**: each group's summary has
`unwritten: []` and one `e2e` record per §1 promise exists in the run's records
repository; (3) **no operator identity**: every record's `run_by` begins
`test-account:`, its signer is the night's own device, and every app the runner
started reported `custody_backend: file` and `windows: 0` (the runner prints
one such line per app). Held is all three, whatever the verdicts: a red
promise is the checklist's business ([[d316-pass-means-the-whole-promise]]),
not this invariant's. Run in the slow tier of local CI in a gate slot.

**Why the first check was not a check (Seraph's exact read on 7d4d599e,
2026-10-25).** `BBOXY_UNATTENDED=1 sh scripts/e2e-night.sh` exited 0 in 103 s,
but it ran only the terminal group (the script's default argument), the
variable was read by nothing in the tree, and its exit was the night's amber
verdict rather than proof that nobody was needed; over every group it would
exit 1 by design wherever a promise has no script yet. A verdict is not the
property. **Active (U-3, Seraph, landed 6dfbf569, 2026-10-25).** `scripts/e2e-unattended.sh`
held on its gate (05ec2d2f) and on main: twelve groups, 55 promises written,
every record by a test account and the night's device, every app on file
custody with zero windows, every group ended by verdict. The check stays in
the slow tier (`slow: true`, `gate: night`): it runs the whole night, so it
runs under `scripts/ci-local.sh --slow` and in the night itself, never in the
default gate. With this, the four pieces of
[[d282-app-tests-itself-unattended]] are all on main.

**Cost of a violation.** A test that needs a human present is a meeting; the
operator worked fourteen hours for a night that should have run itself.

**Evidence.** None yet (pending U-3). The night of 2026-10-23 to 24 is the
counter-evidence: Keychain holds, trust prompts and a locked display each needed
the operator.

Owner: Corvin Duskmantle <ARCHITECT>.
