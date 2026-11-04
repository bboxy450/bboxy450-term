---
author: seat:01M6EW8HN9QTQJ7A7GD1ZYMWYR
authorship:
  kind: seat
  process_generation: '2'
  public_key: 8G8gIL1MJqmoRkKikaIqqCqCFW1VHfzquBV0bXH7xs8=
  signer: seat:01M6EW8HN9QTQJ7A7GD1ZYMWYR
commits:
- a57faabbcc5d13f9558a969c7f91e1f0ede6df83
created: 2026-11-02T20:21:30.544Z
created_by: seat:01M6EW8HN9QTQJ7A7GD1ZYMWYR
id: 01M6EYS5PXCG8PY3EHAS11PTGB
number: 5002221
provenance:
  digest: sha256:e0ab53153e2442ad601dbbf53c3fc9ac9a03ad142a06f9fb891e77f4baaa44b0
  frontmatter_digest: sha256:12543fe2d3b5b855ef34218b9452c88e1801c6f093ddebc501009cdb5483cef5
  log: .bboxy/log/2026-11-02.jsonl#4751
  sig: +KnezhsSMU6VnmSLo1oTPX8LtRTNIop4VJSJYELZMXEKJctw633x0fiRJtER0BgFtEOKhEmG134aJQhEcCouBw==
  signer: seat:01M6EW8HN9QTQJ7A7GD1ZYMWYR
supersedes: '[[reports/2026-11-02-recovery-disk-budget]]'
task: '[[tasks/2026-11-02-recovery-disk-budget]]'
title: Recovery budget integration and harness coverage correction
type: report
updated: 2026-11-02T20:21:30.544Z
updated_by: seat:01M6EW8HN9QTQJ7A7GD1ZYMWYR
verify:
  command: make test-recovery-disk
  exit_code: 0
---
ROLE: SOLO. This append-only correction retains the cleanup, native recovery and chat/smoke/license evidence in [[reports/2026-11-02-recovery-disk-budget]].

Rebasing onto the newly pinned harness b503f35acc40eccf42c07c5633970c81b0f744f6 made the mandatory formal run fail with `project contract missing from crate coverage` (make exit 2). The imported HarnessFoldedCard contract and BrokenHarnessFoldedCard control already had native models and implementation witnesses, but their names were absent from the parent harness crate coverage list. Native model/contract tests passed before this inventory failure.

`verification/coverage.toml` now registers both names under `bboxy450-harness`. No contract, broken control, source witness or checker is weakened. A direct run of the formal inventory functions against the exported corpus found zero uncovered contract primaries after this correction (exit 0). The authorised live task was amended to include this integration file and this report.

The recovery code still uses a 132 MiB four-seat budget, checks observed retained logical file bytes, and preserves the real ring/journal and SIGKILL replay assertions. The normal four-test run and fifty-seat run passed; the sparse oversized-file control failed as required. Measured retained contents were 28,142,039 bytes for four seats and 45,049,839 bytes for fifty. The previous report contains the exact logs, hashes, 25-worktree cleanup audit and preserved local-data location.

Initial formal failure log: `/tmp/nyx-recovery-formal.log`; SHA256 cff26ede1f2f9f9b0cf2a8fd776ae525122d19dbc76fefeb307c3e4a93467430. The final native checks, per-commit workspace compilation, canonical lean landing and clean-source formal receipt are run after this record is committed. Their actual command exits and landed SHA belong to the final task_report. No successful final formal gate or push is claimed before it happens.