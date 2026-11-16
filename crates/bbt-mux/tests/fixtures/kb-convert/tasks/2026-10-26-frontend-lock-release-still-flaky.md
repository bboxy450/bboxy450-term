---
authorship:
  device_id: 01M677XWZPW1XGTQYYGA50W74M
  issued_by: test-account:01M677XWYWXAQQM5DEQP83TQYY
  kind: device
  principal: operator:01M677XWZPW1XGTQYYGA50W74M
  public_key: cHjhL+5pI9VNQE+Uhb4HJBnYoPqSIH7aCYU+b9MElWs=
  signer: device:01M677XWZPW1XGTQYYGA50W74M
created: 2026-10-26T19:49:42Z
created_by: device:01M677XWZPW1XGTQYYGA50W74M
id: 01M5WW5XTNGJD1DFBH7RHTM547
issued_by: test-account:01M677XWYWXAQQM5DEQP83TQYY
number: 360
owner: Rook Ashwhisker
provenance:
  digest: sha256:911a872bbdc81f54d8164debe30bbdaab8fc8f16f3598005c446a22dd91b69bb
  frontmatter_digest: sha256:baf35f68ab503e6ed1c020c795f750f751743bde1ad07e17b680b94e4d5f02bb
  log: .bboxy/log/2026-10-30.jsonl#743
  sig: nk1UDxaz+DenPkAipB4tkUeFAWqq00mI4a+fe2vgPICCmLFawF435TExG6wubE1LUFmzycAKgaLMna704T4cBw==
  signer: device:01M677XWZPW1XGTQYYGA50W74M
reconciliation:
- by: operator:01M677XWZPW1XGTQYYGA50W74M
  disposition: implemented
  evidence:
  - https://github.com/bboxy450/bboxy450-term/commit/f33e9fe61a86050677c64ba2b746af1fb530b670
  fresh_accepted: false
  kind: historical
  op_id: 01M677YDNR3FSDBZRCA30KT2ZJ
  previous_status: open
  reason: Operator-authorized historical msgrec reconciliation, 2026-10-30; implementation already landed on main; no new acceptance claimed.
role: executor
status: done
title: Frontend lock release still flaky after the explicit-unlock fix
type: task
updated: 2026-10-30T20:27:44.316Z
updated_by: device:01M677XWZPW1XGTQYYGA50W74M
verify: cargo test -p bboxy450-term --lib cockpit::owners::tests::frontend_lock_refuses_a_live_owner_and_releases_on_departure
---
# Frontend lock release still flaky after the explicit-unlock fix

After 8755bd11 (explicit LOCK_UN), the test failed again in a presubmit: candidate 3f649261, log ~/.local/state/bboxy450-term/landing/presubmits/3f6492611df81f1981ed556bae1ff78ba7539e8a-4bbffec5/commands.log:418, panic at crates/bboxy450-term/src/cockpit/owners.rs:338 (104 passed, 1 failed). Quarantined meanwhile so app presubmits keep moving.

## Acceptance

Find the remaining mechanism, fix it, prove it with a deterministic regression, and remove the quarantine row. No timing tolerance.
