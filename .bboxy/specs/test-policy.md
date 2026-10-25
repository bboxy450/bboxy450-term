---
id: 01M5SZK7DG7CTKSGVFM4BD2RTC
type: spec
number: 327
title: "test policy: what a test is for, the mechanical criteria for pointless, dumb and too-direct tests, what is deleted, rewritten, moved to the night or kept, and how a sweep proves coverage did not drop"
status: decided
links:
  - { to: "[[landing]]", rel: builds-on }
  - { to: "[[presubmit-tests-fast]]", rel: invariant }
  - { to: "[[d321-out-of-integration-hell]]", rel: builds-on }
  - { to: "[[e2e-acceptance]]", rel: builds-on }
  - { to: "[[d316-pass-means-the-whole-promise]]", rel: builds-on }
  - { to: "[[mantras]]", rel: mantra }
created: "2026-10-25T16:51:40Z"
updated: "2026-10-25T16:51:40Z"
---
# Test policy

Operator, 2026-10-25, verbatim: "we probably have lots of tests that are pointless or dumb approach or too direct of approarch", and "timeout is problematic coz we might run tests in parallel then everything goes to shit". The numbers on `main` that day: 884 `#[test]` functions (bbt-mux 238, bbt-ui 160, bboxy450-term 131, bboxybook 72, bbt-yarn 46, bbt-ui-model 43, bbt-supervisor 41); 35 integration-test files call `sleep`; 13.4 hours of test run and 9.3 of test compile over 175 gates.

**Lens.** A test is a check of one contract an invariant, a spec line or a decision promised, written so that it fails for exactly one reason and passes under any load. **Assumes** tests run in parallel by default and on a machine shared with other presubmits. **Breaks when** a test's pass depends on time, order or the machine.

## 1. What a test is for

One contract, one behaviour, one failure mode. The tests that are the law's own checks, the `check:` lines of the invariants, the night scripts of [[e2e-acceptance]], the wire goldens, and the tests a spec's "Acceptance, the spec signs off" section names, are **kept** whatever else the sweep does; everything else justifies itself against the criteria below or goes.

## 2. The criteria, mechanical

An Executor applies these to one crate's tests, one test at a time, and writes the letter and number beside each in the handoff.

**A. Pointless, deleted.** A1 **duplicate coverage**: the same function under test and an assertion set that is a subset of another test's on the same path; keep the one with the stronger assertions. A2 **a test of the fixture**: every assertion is about values the test itself constructed, and the code under test is never called, or is called and its result unused. A3 **asserting prose or constants**: a string literal, a variant name, a default value or a struct layout compared with itself, or a snapshot of a constant. A4 **cannot fail**: no assertion, `assert!(true)`, or `is_ok()` on a path that cannot return an error.

**B. Dumb approach, rewritten.** B1 **real time**: `sleep`, waiting on `Instant::now`, a `timeout` as the assertion; rewrite on an injected clock (the App clock, `tokio::time::pause`, a `Clock` seam), or shorten the duration in the test build, so the behaviour is tested and the seconds are not. B2 **real processes, sockets or PTYs where a seam exists**: in-memory transport, the fake supervisor, the recording backend; rewrite on the seam, and keep **one** real-boundary test per boundary in the night tier so the seam is itself proven honest. B3 **polling and retry loops**: rewrite as a wait on a condition the code signals. B4 **load-dependent**: anything that passes alone and fails under three presubmits; it is a bug, a seam, or the night tier, never a retry.

**C. Too direct, rewritten to the contract.** C1 **asserting internal state**: a private field, a struct the caller never sees, log text; assert the reply, the record, the screen or the event the contract names. C2 **interaction testing**: asserting the call order or arguments of internals or mocks where an output is observable. C3 **pinned to error prose**: assert the error kind, not the sentence. C4 **goldens of internal structures**; the wire goldens of [[d320-dispatch-by-noun]] and the parity catalogue are contracts and stay.

**D. Kept, and tagged so the sweep can see it.** Every invariant's check test, every night script, every wire golden, one test per spec acceptance line, and every regression test a decision cites by commit. The sweep marks each with one comment line above the test, `// keeps: invariant <name>`, `// keeps: spec <file> §<n>` or `// keeps: decision #<n>`; a test without a `keeps:` line after the sweep is one the crate's owner has judged, not one that slipped through.

## 3. Dispositions

**Delete** A1 to A4. **Rewrite** B1 to B3 and C1 to C4 on the seam or the contract. **Move to the night** B4 and the one real-boundary test per boundary of B2, tagged for the night tier so the presubmit never runs them. **Keep** D. The sweep's handoff lists, for every deleted test, the test that still covers its behaviour, and for every moved test, its night script; the acceptance of the sweep is that **no invariant check and no spec acceptance line lost its test**, checked by the `keeps:` tags against the invariants' README and the specs' acceptance sections, and that the crate's test-run time fell.

## 4. Timeouts and parallelism

The operator's constraint is the rule: **no test asserts a wall-clock bound**, and no test's pass depends on how many others run beside it. A test-internal timeout is a **hang guard only**, at least ten times the expected duration and never under 30 seconds, and its firing is a failure to investigate, never a verdict on speed. Tests are independent: unique temporary directories, no fixed ports or paths, no shared process-wide state, no ordering. Timing is measured in exactly two places, both under controlled load: the frame gate of [[seats-967-frame]] and the night's `fast-tests` step. **[[presubmit-tests-fast]] is amended accordingly**: its hard gate in presubmit is the deterministic grep for sleeps and real-clock waits; its 60-second per-binary bound is measured in the night tier with the binaries run alone, and a presubmit over it is reported, not failed.

## 5. The sweep, per crate

One Executor per crate, in the Oracle's order (bbt-mux first, 238 tests and the slowest binaries). (1) Inventory with `cargo test -p <crate> -- --list`. (2) Tag D. (3) Classify the rest with §2, one letter and number each. (4) Dispose per §3. (5) Run the crate's tests and the invariants. (6) Hand off: the per-test table, the coverage map of §3, the before and after test count and test-run seconds, the number of `sleep` calls removed. Rook's #326 cuts on the six slowest binaries run first and are the sweep's first evidence.

## 6. Metrics

Baseline on `main`, 2026-10-25: 884 tests, 35 files with `sleep`, 13.4 h test run and 9.3 h test compile over 175 gates. The board shows these per crate after each sweep; the target the Oracle sets is the test-run time of each swept crate halved with no `keeps:` lost.

Owner: Corvin Duskmantle <ARCHITECT>.
