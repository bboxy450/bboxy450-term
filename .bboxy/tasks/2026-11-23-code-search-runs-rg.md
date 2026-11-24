---
allowed_paths:
- crates/bboxy450-code
- crates/bboxy450-harness
- scripts
assignee: seat:01M86G92ZCH7KJ28YMGPGTRNYR
authorship:
  kind: seat
  process_generation: '1'
  public_key: bGMqsRvbQXvEDVnOwRt47b6crPtdbIu8ioBX1dU/YNs=
  signer: seat:01M85BW3PMN3QBFPPMWBS866RA
closed:
  by: seat:01M85BW3PMN3QBFPPMWBS866RA
  op_id: 01M86Q3GH2GJ3WN2PS3ASSG7GZ
created: 2026-11-23T23:41:14.817Z
created_by: seat:01M85BW3PMN3QBFPPMWBS866RA
delegated_by: seat:01M85BW3PMN3QBFPPMWBS866RA
id: 01M85CHZHNAD2H6MCGMRT0Q6VR
number: 1794292861627826
report:
  commit: a76e188d74b3dac549b35ab5fe3c5f6d4d99ed9e
  op_id: 01M86Q1120T2QN8GNJJK6Q05YA
  report: '[[reports/2026-11-24-code-search-runs-rg.den]]'
  reported_by: seat:01M86G92ZCH7KJ28YMGPGTRNYR
  summary: Landed compact code search using ripgrep 15.2.0 in signed commit a76e188d7. All 14 query tests passed under make land, the fixture matched rg -n, and forcing a split in overlapping context failed the fixture as expected. Removed the Rust RegexSet matcher and per-hit context arrays. Process-boundary checks and formal parity for 1,003 models passed. The worktree is clean. The local report is readable as a draft. Its KB checkpoint was refused because reports are ignored by that repository.
  verify_revision: 0
role: executor
status: done
swarm: 01M7P04CW1T6KY8CMSS6DQM12Q
title: 'bboxy_code.search: run ripgrep underneath and return compact rg-style hits'
type: task
updated: 2026-11-24T12:04:50.213Z
updated_by: seat:01M85BW3PMN3QBFPPMWBS866RA
verify: cargo nextest run -p bboxy450-code
verify_result:
  command: cargo nextest run -p bboxy450-code
  evidence:
  - criterion: Ripgrep output parity and merged context
    reference: crates/bboxy450-code/tests/queries.rs:166
  - criterion: Broken control fails on an extra context block
    reference: target/code-search-broken-control.log
  - criterion: Bounded deterministic result and compact MCP text
    reference: crates/bboxy450-code/tests/queries.rs:497 and :853
  - criterion: Landed signed commit and lean verification
    reference: target/code-search-land.log
  - criterion: Formal evidence for landed SHA
    reference: 35a8bd7e0f4cb68ad347b9fc7ddc98cac761f193e101a19549de3431af9fcc82
  exit_code: 0
provenance:
  digest: sha256:0276bd240af3c300116e53ada42dc7cb463e7cc9df10ea508e2b235efb6ed348
  frontmatter_digest: sha256:9df29d9e9f605a759af44e6460a75471e8fcff35d3537d0c9d1ec9802f6986aa
  signer: seat:01M85BW3PMN3QBFPPMWBS866RA
  sig: a4PjL10OVHR0Tm9LIDg6KBbLrzD9cS4MG/Gk4oC5sxb4rAP3HUQIPzHFiDLyr0/9ZwSByNKlrWzA/MFLQqh4BA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1723
  event:
    id: 01M86Q3J2213JV0XAZKJ779NAS
    sequence: 1723
    hash: 747af47a5348076a34baa965a4f13c047afe40a53246803ebef2e012c53f3dc6
---
Our stand: subtract before you add; report what you removed.

## Why (operator, 2026-11-23)
"even you right now are using rg, not boxy code search … its ok to just run rg in our code search for now and then we can improve it."
`make tool-usage DAYS=1`: shell 1129 code reads (rg 355, sed 254) vs bboxy_code 1. The Oracle itself reached for `rg` in Bash because
`bboxy_code.search` results are bulky: each hit repeats its full before/after arrays, and adjacent hits in one function repeat the same 20 lines (observed: 3 hits in `fn authorize` returned the same block three times).

## Do
1. Make `search` shell out to `rg` (pinned/bundled path, `--json`), keeping its existing parameters (`patterns`, `before`, `context`, `ignore_case`, `all`) and the enclosing-item label. Delete the bespoke matcher it replaces.
2. Merge overlapping context windows in one file into one block; default output is compact text `path:line: text` with context lines once, like `rg -n -C`.
3. Keep bounded output and the spill-to-file behaviour.
4. Tests on a fixture tree: overlapping hits produce one block; results equal `rg -n` for the same pattern; broken control: disable merging → duplicate-block test fails.
5. `make land`; report rg version and what you removed.

## Report from seat:01M86G92ZCH7KJ28YMGPGTRNYR

Landed compact code search using ripgrep 15.2.0 in signed commit a76e188d7. All 14 query tests passed under make land, the fixture matched rg -n, and forcing a split in overlapping context failed the fixture as expected. Removed the Rust RegexSet matcher and per-hit context arrays. Process-boundary checks and formal parity for 1,003 models passed. The worktree is clean. The local report is readable as a draft. Its KB checkpoint was refused because reports are ignored by that repository.

```json
{
  "op_id": "01M86Q1120T2QN8GNJJK6Q05YA",
  "commit": "a76e188d74b3dac549b35ab5fe3c5f6d4d99ed9e",
  "report": "[[reports/2026-11-24-code-search-runs-rg.den]]",
  "verify_result": {
    "command": "cargo nextest run -p bboxy450-code",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Ripgrep output parity and merged context",
        "reference": "crates/bboxy450-code/tests/queries.rs:166"
      },
      {
        "criterion": "Broken control fails on an extra context block",
        "reference": "target/code-search-broken-control.log"
      },
      {
        "criterion": "Bounded deterministic result and compact MCP text",
        "reference": "crates/bboxy450-code/tests/queries.rs:497 and :853"
      },
      {
        "criterion": "Landed signed commit and lean verification",
        "reference": "target/code-search-land.log"
      },
      {
        "criterion": "Formal evidence for landed SHA",
        "reference": "35a8bd7e0f4cb68ad347b9fc7ddc98cac761f193e101a19549de3431af9fcc82"
      }
    ]
  }
}
```
