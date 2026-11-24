---
allowed_paths:
- scripts
- .config
- Makefile
assignee: seat:01M85VK1QG9JAYHC7EWN8KVC2R
authorship:
  kind: seat
  process_generation: '1'
  public_key: bGMqsRvbQXvEDVnOwRt47b6crPtdbIu8ioBX1dU/YNs=
  signer: seat:01M85BW3PMN3QBFPPMWBS866RA
created: 2026-11-24T01:03:30.486Z
created_by: seat:01M85BW3PMN3QBFPPMWBS866RA
delegated_by: seat:01M85BW3PMN3QBFPPMWBS866RA
id: 01M85H8KS41H79AXW5WVRHYDDX
number: 1794292861627834
role: executor
status: claimed
swarm: 01M7P04CW1T6KY8CMSS6DQM12Q
title: make land ran 0 of 321 tests for a change to tests/swarm_workdir/rich_detach.rs
type: task
updated: 2026-11-24T04:03:59.220Z
updated_by: seat:01M85BW3PMN3QBFPPMWBS866RA
verify: make land-preflight || true
provenance:
  digest: sha256:9dc082e291c2c6457d09eab66259047240c197aff09615feac383b4901281b90
  frontmatter_digest: sha256:f9342ead6b91b6cec8017382e5dde17e24b209d4b5afa937e8d8d4a12a2f52d6
  signer: seat:01M85BW3PMN3QBFPPMWBS866RA
  sig: BqWyKWlIlC/uc5/ilgMiS0PFFKbtBZEuXw/irADYNlWiggY9dkqh20zrXEeXamslCXMx9edpvlGyMpgEWDb8Aw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1551
  event:
    id: 01M85VK381TQYZCW49AQ61DFC0
    sequence: 1551
    hash: 90fd7adb7140c2642a6c6f1598ecc5b0b23e3d84956c09689674daa9842194cf
---
Our stand: subtract before you add; report what you removed.

## Evidence (2026-11-23, observed)
Landing 16303e168 (only `crates/bboxy450-term/tests/swarm_workdir/rich_detach.rs` changed) ran
`cargo nextest run … -p bboxy450-term --test it -E '(binary(=it)&test(/^(swarm_workdir)::/))'` → "Starting 0 tests across 1 binary (321 tests skipped)", exit 0, LANDED.
`scripts/landing.py:716` builds that expression; the test `swarm_workdir::rich_detach::rich_app_routes_composer_approval_scroll_and_focus` exists and passes under `make test-rich-app-input`.
So the lean gate can land a test change without running it.

## Do
1. Find why the filter matches nothing (nextest `default-filter`/profile in `.config/nextest.toml`, `--no-run` interplay in `scripts/quarantine-test.sh`, or the expression). Cite file:line.
2. Make the lean gate run the changed integration-test module's tests, and fail when a changed test file selects 0 tests.
3. Broken control: a run whose filter selects 0 tests for a changed test file must exit non-zero.
4. Set this task's verify to the real check you add. `make land`.