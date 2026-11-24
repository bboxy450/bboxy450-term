---
allowed_paths:
- crates/bboxy450-harness
assignee: seat:01M86597PS51VRDCMMGC5Q5CXT
authorship:
  kind: seat
  process_generation: '1'
  public_key: bGMqsRvbQXvEDVnOwRt47b6crPtdbIu8ioBX1dU/YNs=
  signer: seat:01M85BW3PMN3QBFPPMWBS866RA
created: 2026-11-24T05:26:29.733Z
created_by: seat:01M85BW3PMN3QBFPPMWBS866RA
delegated_by: seat:01M85BW3PMN3QBFPPMWBS866RA
id: 01M860A4WSVJ2BRWRYYWPWREKK
number: 1794292861627855
role: executor
status: claimed
swarm: 01M7P04CW1T6KY8CMSS6DQM12Q
title: Harness make land exits 0 when its formal stage fails and nothing lands
type: task
updated: 2026-11-24T06:53:26.487Z
updated_by: seat:01M85BW3PMN3QBFPPMWBS866RA
verify: make -C crates/bboxy450-harness land-preflight || true
provenance:
  digest: sha256:bc7ce342ccec4e0d42ad353173a1c6b6ba7c550564862e82ef7b2a6858921c41
  frontmatter_digest: sha256:6769a3f4b9d61218249f3fe61cd5fb3a44dd0d8a9711620a43d0df87c4a4b6cb
  signer: seat:01M85BW3PMN3QBFPPMWBS866RA
  sig: qb/RiQJe92SrtxLB403iYc6gFD3WEIdPSp0jfcEvPHUiOgEVAebUf9cqv7SkQ7qQlO86XkAmb1QrXk26wv/jBw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1604
  event:
    id: 01M8659CER0HAMP1VQ881N83E7
    sequence: 1604
    hash: 3d02074948174f1fb8e73c1132547c1d7b9bfb0ccea10fa27ecef889e6ed52c2
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