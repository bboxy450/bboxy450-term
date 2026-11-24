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
closed:
  by: seat:01M85BW3PMN3QBFPPMWBS866RA
  op_id: 01M8604982R67CDQ0F9WA0DE7D
created: 2026-11-24T01:03:30.486Z
created_by: seat:01M85BW3PMN3QBFPPMWBS866RA
delegated_by: seat:01M85BW3PMN3QBFPPMWBS866RA
id: 01M85H8KS41H79AXW5WVRHYDDX
number: 1794292861627834
report:
  commit: 0d88d632e5eede56ada55941786e99909d2dd821
  op_id: 01M85ZB0CMWYW7QDPMAHX762E7
  report: '[[reports/2026-11-23-lean-land-ran-zero-tests.den]]'
  reported_by: seat:01M85VK1QG9JAYHC7EWN8KVC2R
  summary: '[[Implement]] Landed signed `0d88d632e5` on `main` with a clean worktree. Separate Nextest selections make lean landing reject a changed module with zero matches, and `rich_detach.rs` changes run the existing native composer and approval test. Removed the combined integration filter and the unconditional empty-run pass for explicit integration tests. The amended `make test-lean-ignored` exited 0 on the landed SHA with 31 fixtures and the native test passed. The broken control rejected the empty module and kept the private origin unchanged. `make land`, `make test-licenses`, `make test-formal FORMAL_REUSE=1`, and `make check-formal-evidence` each exited 0.'
  verify_revision: 1
role: executor
status: done
swarm: 01M7P04CW1T6KY8CMSS6DQM12Q
title: make land ran 0 of 321 tests for a change to tests/swarm_workdir/rich_detach.rs
type: task
updated: 2026-11-24T05:23:18.684Z
updated_by: seat:01M85BW3PMN3QBFPPMWBS866RA
verify: make test-lean-ignored
verify_result:
  command: make test-lean-ignored
  evidence:
  - criterion: Cause of the zero-test landing
    reference: 16303e168, scripts/landing.py:754-762 and crates/bboxy450-term/tests/swarm_workdir/rich_detach.rs:225
  - criterion: Run the changed integration selection and the native rich App witness
    reference: scripts/test-landing.py:803 and target/lean-ignored-amended-verify.log:47-61
  - criterion: Reject a changed module whose filter has zero matches
    reference: scripts/test-landing.py:813 and target/lean-ignored-amended-verify.log:19-29
  - criterion: Signed landing
    reference: target/lean-land-verify.log:42 and origin/main at 0d88d632e5eede56ada55941786e99909d2dd821
  exit_code: 0
verify_revision: 1
provenance:
  digest: sha256:6a4eccb413b69aac420c5a9829d98a0ca2642e559074d618fcfd341a8e06785e
  frontmatter_digest: sha256:5bf6802ab006f3516a6b244f971a966d9e479c4533f3dbffb1a689cbcec62c91
  signer: seat:01M85BW3PMN3QBFPPMWBS866RA
  sig: 4aYdBuDDrZjbQ6tShqId5EGl+n/FZqhXac7OeuifYatctlabqjq4qXwpW2xxAo1afBTZfoVKWYuHvuIIgdHgDA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1573
  event:
    id: 01M8604B9GB04S6XNFQ9MSBTBH
    sequence: 1573
    hash: 3ae459dca6ded01fba8be52904df6cf13bc4c2379ed079654ed8093a55218201
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

## Report from seat:01M85VK1QG9JAYHC7EWN8KVC2R

[[Implement]] Landed signed `0d88d632e5` on `main` with a clean worktree. Separate Nextest selections make lean landing reject a changed module with zero matches, and `rich_detach.rs` changes run the existing native composer and approval test. Removed the combined integration filter and the unconditional empty-run pass for explicit integration tests. The amended `make test-lean-ignored` exited 0 on the landed SHA with 31 fixtures and the native test passed. The broken control rejected the empty module and kept the private origin unchanged. `make land`, `make test-licenses`, `make test-formal FORMAL_REUSE=1`, and `make check-formal-evidence` each exited 0.

```json
{
  "op_id": "01M85ZB0CMWYW7QDPMAHX762E7",
  "commit": "0d88d632e5eede56ada55941786e99909d2dd821",
  "report": "[[reports/2026-11-23-lean-land-ran-zero-tests.den]]",
  "verify_result": {
    "command": "make test-lean-ignored",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Cause of the zero-test landing",
        "reference": "16303e168, scripts/landing.py:754-762 and crates/bboxy450-term/tests/swarm_workdir/rich_detach.rs:225"
      },
      {
        "criterion": "Run the changed integration selection and the native rich App witness",
        "reference": "scripts/test-landing.py:803 and target/lean-ignored-amended-verify.log:47-61"
      },
      {
        "criterion": "Reject a changed module whose filter has zero matches",
        "reference": "scripts/test-landing.py:813 and target/lean-ignored-amended-verify.log:19-29"
      },
      {
        "criterion": "Signed landing",
        "reference": "target/lean-land-verify.log:42 and origin/main at 0d88d632e5eede56ada55941786e99909d2dd821"
      }
    ]
  }
}
```
