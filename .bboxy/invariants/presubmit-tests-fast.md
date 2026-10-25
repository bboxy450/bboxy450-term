---
id: 01M5SZ3DBWC2P0EG4GJQVAVEEY
type: invariant
number: 326
title: "presubmit tests are fast: no test binary in the presubmit tier runs longer than 60 s and no test waits on the wall clock; long-duration behaviour is tested through an injected clock or a shortened test duration, or in the night tier"
status: pending
slow: true
gate: pending-presubmit-log
check: sh scripts/fast-tests.sh
cost: a minute of real sleep in one test is a minute in every presubmit on every machine forever; six such binaries cost over eight of the thirty-seven cargo hours measured on 2026-10-25, and no hardware purchase buys them back
links:
  - { to: "[[landing]]", rel: spec }
  - { to: "[[d321-out-of-integration-hell]]", rel: decision }
  - { to: "[[d325-ci-capacity]]", rel: builds-on }
  - { to: "[[seats-967-frame]]", rel: builds-on }
created: "2026-10-25T16:43:02Z"
updated: "2026-10-25T16:43:02Z"
---
# Presubmit tests are fast

**Property.** In the presubmit tier ([[landing]] §3) no test binary runs longer
than **60 seconds** and no test waits on the wall clock: a behaviour whose
contract is a duration (a 60-second undo, a backlog timeout, a restart grace,
a custody hold expiry) is tested through an **injected clock** or a **shortened
duration** the test build selects, the way the UI's animations read one App
clock the night can freeze and advance. A test that must run the real
duration is tagged for the night tier and never runs in presubmit. A wait in
a presubmit test is a wait on a condition with a short bound, never a sleep of
a second or more.

**Check.** `sh scripts/fast-tests.sh` — exit 0 held. It reads the per-binary
durations the presubmit writes to `presubmit.log` in the landing state
directory ([[landing]] §0a) for the newest presubmit and fails naming any
binary over 60 s that is not tagged for the night tier, and greps the
presubmit-tier tests for `sleep(Duration::from_secs(` with an argument of one
or more. **Pending** until `presubmit.sh` writes per-binary durations; the
flags keep it out of the default run until then, and the landing commit
removes them.

**Cost of a violation.** A minute of real sleep in one test is a minute in
every presubmit on every machine forever; six such binaries cost over eight of
the thirty-seven cargo hours measured on 2026-10-25, and no hardware purchase
buys them back.

**Evidence.** The Oracle's parse of 175 gate logs, 2026-10-25 (36.9 h of cargo
time): test run 36 %, test compile 25 %, build 21 %, post-test smoke,
invariants and parity builds 12 %, clippy 6 %. The slowest binaries summed:
`socket_backlog.rs` 2.1 h (one test, 52 to 72 s in every gate),
`lifecycle.rs` 1.9 h (three tests, about 61 s), `custody` 1.2 h,
`identity_local` 1.1 h, `logic` 0.8 h, `provenance` 0.75 h. Those are
real-clock waits. Report at the board's `time.html`; parser
`tools/gate_time.py` in the board repository.

Owner: Corvin Duskmantle <ARCHITECT>.
