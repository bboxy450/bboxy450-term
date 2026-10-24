---
id: 01M5QNKAYKD6J9NEEFGFWJ0PSQ
type: invariant
number: 283
title: "the e2e night runs unattended: no operator input, no login-Keychain access, no window, no build or signature"
status: pending
slow: true
gate: pending-u3
check: BBOXY_UNATTENDED=1 sh scripts/e2e-night.sh
cost: a test that needs a human present is a meeting; the operator worked fourteen hours for a night that should have run itself
links:
  - { to: "[[d282-app-tests-itself-unattended]]", rel: decision }
  - { to: "[[e2e-acceptance]]", rel: constrains }
  - { to: "[[id1-seat-chain]]", rel: constrains }
  - { to: "[[ctl-opt-in]]", rel: sibling }
created: "2026-10-24T19:18:29Z"
updated: "2026-10-24T19:24:37Z"
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

**Check.** `BBOXY_UNATTENDED=1 sh scripts/e2e-night.sh` — exit 0 held, exit 1
with the red rows named; run in the slow tier of local CI (`sh scripts/ci-local.sh
--slow`) in a gate slot, with stdin closed and the display locked or absent. The
runner does not exist yet; U-2 and U-3 create it and the commit removes `slow`
and `gate` from this file.

**Cost of a violation.** A test that needs a human present is a meeting; the
operator worked fourteen hours for a night that should have run itself.

**Evidence.** None yet (pending U-3). The night of 2026-10-23 to 24 is the
counter-evidence: Keychain holds, trust prompts and a locked display each needed
the operator.

Owner: Corvin Duskmantle <ARCHITECT>.
