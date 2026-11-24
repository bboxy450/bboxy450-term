---
allowed_paths:
- crates/bboxy450-harness
assignee: seat:01M86597PS51VRDCMMGC5Q5CXT
authorship:
  kind: seat
  process_generation: '1'
  public_key: bGMqsRvbQXvEDVnOwRt47b6crPtdbIu8ioBX1dU/YNs=
  signer: seat:01M85BW3PMN3QBFPPMWBS866RA
closed:
  by: seat:01M85BW3PMN3QBFPPMWBS866RA
  op_id: 01M867003G3YCDXHSEJ8VKBKAV
created: 2026-11-24T05:26:29.733Z
created_by: seat:01M85BW3PMN3QBFPPMWBS866RA
delegated_by: seat:01M85BW3PMN3QBFPPMWBS866RA
id: 01M860A4WSVJ2BRWRYYWPWREKK
number: 1794292861627855
report:
  commit: e9efa3bfb0d7a2aafdbceb0303ff2e8509187f98
  op_id: 01M866YRFPKVCR42G9EVPPB6TD
  report: '[[reports/2026-11-23-harness-land-exit-0-on-formal-failure.den]]'
  reported_by: seat:01M86597PS51VRDCMMGC5Q5CXT
  summary: '[[Implement]] [[Done]] Landed the signed fix on harness `origin/main`. `make land` exited 0 after all 533 native/TLC comparisons and current-source receipt validation. Removed the ignored evidence probe and unused `FORMAL_REUSE=1` argument. Formal parity and receipt validation are now required and failures name their Make target. `make test-land` exited 0 and observed exit 2 with the private remote unchanged for both a stale snapshot and a missing receipt. The full report is indexed locally. The knowledge checkpoint refused the ignored `reports` directory.'
  verify_revision: 1
role: executor
status: done
swarm: 01M7P04CW1T6KY8CMSS6DQM12Q
title: Harness make land exits 0 when its formal stage fails and nothing lands
type: task
updated: 2026-11-24T07:23:17.524Z
updated_by: seat:01M85BW3PMN3QBFPPMWBS866RA
verify: make -C crates/bboxy450-harness test-land
verify_result:
  command: make -C crates/bboxy450-harness test-land
  evidence:
  - criterion: Locate the swallowed formal status
    reference: Pre-change scripts/land.py:313 marked check-formal-evidence required=False. Current scripts/land.py:311 makes parity and receipt validation required.
  - criterion: Fail before push and name the stage
    reference: scripts/land.py:35 prints full failed command and exit. scripts/test-hardening.py:134–147 asserts Make exit 2, stage name, a failed receipt with no landed result, and an unchanged private remote in both controls.
  - criterion: Landed in the harness repository
    reference: make -C crates/bboxy450-harness land exited 0. Signed commit e9efa3bfb0d7a2aafdbceb0303ff2e8509187f98 was independently returned by git ls-remote origin refs/heads/main.
  exit_code: 0
verify_revision: 1
provenance:
  digest: sha256:7ad42537e521b3b38193ae036a6287d5973cd58c50caf837eb14e07f78226782
  frontmatter_digest: sha256:ff10132348d49a65af36487a945a3e8e9360f34d5c5fb047a08a66aa9a66dad4
  signer: seat:01M85BW3PMN3QBFPPMWBS866RA
  sig: 0w0Xf2cXpSaidvpntufvqR+L0QIgIR3WFarWZztCmz6KqJKY5YspHlsyP31Nlx6pUbHlauAnHI7zXT3KO/fWAw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1623
  event:
    id: 01M86701A43TP9K97SVQQ6KEQW
    sequence: 1623
    hash: 4b06013a6e7788d26965ac92896a2181a93af48eaed3868c4f50db337a2dd43b
---
Our stand: subtract before you add; report what you removed.

## Evidence (2026-11-23, observed)
In `crates/bboxy450-harness` of checkout `01M85BXRTDCN7AA08F3Q9PENG3`, `make land` printed
`formal: FAILED: upstream main SOURCE hash changed; re-copy scripts/formal.py` (raised at `scripts/formal.py:303`, `source_pins`), pushed nothing
(origin/main stayed ab591d0), and still exited 0. The cause of that failure (stale `verification/upstream/scripts-formal.py` after term 8a75b9f52) was fixed by hand.

## Do
1. Find where the harness land swallows the formal stage's status (Makefile `land:` at ~line 342 or its script); cite file:line.
2. A failing stage must make `make land` exit non-zero and print which stage failed. Remove any `|| true` or lost `$?` that hides it.
3. Broken control: stale snapshot → `make land` exits non-zero before push.
4. Land it in the harness repo; set this task's verify to the real control.

## Worse (2026-11-23, second run, observed)
Same checkout after re-copying the snapshot: `make land` printed `formal: FAILED: [Errno 2] No such file or directory: …/crates/bboxy450-harness/target/formal/evidence.json`, then `LANDED f9f0b4a666ea4a9fdd49b71302b4c1e9be86229c`, exit 0. The gate pushes to harness main without formal evidence. Fix both: no push and non-zero exit on any formal failure.

## Report from seat:01M86597PS51VRDCMMGC5Q5CXT

[[Implement]] [[Done]] Landed the signed fix on harness `origin/main`. `make land` exited 0 after all 533 native/TLC comparisons and current-source receipt validation. Removed the ignored evidence probe and unused `FORMAL_REUSE=1` argument. Formal parity and receipt validation are now required and failures name their Make target. `make test-land` exited 0 and observed exit 2 with the private remote unchanged for both a stale snapshot and a missing receipt. The full report is indexed locally. The knowledge checkpoint refused the ignored `reports` directory.

```json
{
  "op_id": "01M866YRFPKVCR42G9EVPPB6TD",
  "commit": "e9efa3bfb0d7a2aafdbceb0303ff2e8509187f98",
  "report": "[[reports/2026-11-23-harness-land-exit-0-on-formal-failure.den]]",
  "verify_result": {
    "command": "make -C crates/bboxy450-harness test-land",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Locate the swallowed formal status",
        "reference": "Pre-change scripts/land.py:313 marked check-formal-evidence required=False. Current scripts/land.py:311 makes parity and receipt validation required."
      },
      {
        "criterion": "Fail before push and name the stage",
        "reference": "scripts/land.py:35 prints full failed command and exit. scripts/test-hardening.py:134–147 asserts Make exit 2, stage name, a failed receipt with no landed result, and an unchanged private remote in both controls."
      },
      {
        "criterion": "Landed in the harness repository",
        "reference": "make -C crates/bboxy450-harness land exited 0. Signed commit e9efa3bfb0d7a2aafdbceb0303ff2e8509187f98 was independently returned by git ls-remote origin refs/heads/main."
      }
    ]
  }
}
```
