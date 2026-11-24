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
created: 2026-11-23T23:41:14.817Z
created_by: seat:01M85BW3PMN3QBFPPMWBS866RA
delegated_by: seat:01M85BW3PMN3QBFPPMWBS866RA
id: 01M85CHZHNAD2H6MCGMRT0Q6VR
number: 1794292861627826
role: executor
status: claimed
swarm: 01M7P04CW1T6KY8CMSS6DQM12Q
title: 'bboxy_code.search: run ripgrep underneath and return compact rg-style hits'
type: task
updated: 2026-11-24T10:05:33.806Z
updated_by: seat:01M85BW3PMN3QBFPPMWBS866RA
verify: cargo nextest run -p bboxy450-code
provenance:
  digest: sha256:495a8f21840f55185403e7908715574e5dc9876fe2cb5ce4d6f4b6c99cac60b1
  frontmatter_digest: sha256:c8b0856bc0323a3d2448105eca37768836f23e33d9c095aff147b2e3a5175d97
  signer: seat:01M85BW3PMN3QBFPPMWBS866RA
  sig: ruTBil4Mv+XM/zowH2tvUKa5MpSm9iGG7VmZU1UpyCOFjJxTa2kLnO4Xu7EqNzJIeRWz3yaawV8nqtWUTZXRBg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1681
  event:
    id: 01M86G95DKFS90QXXN3SDM34DA
    sequence: 1681
    hash: 0f7e1afc50c109275c9889e82d0116eee9a64fc06ba4365ee6a09937073feb2e
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