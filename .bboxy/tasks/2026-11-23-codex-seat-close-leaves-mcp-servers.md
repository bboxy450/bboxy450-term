---
allowed_paths:
- crates/bbt-mux
- crates/bboxy450-term
assignee: seat:01M86BFF3067ZDDF0HSF73WVHP
authorship:
  kind: seat
  process_generation: '1'
  public_key: bGMqsRvbQXvEDVnOwRt47b6crPtdbIu8ioBX1dU/YNs=
  signer: seat:01M85BW3PMN3QBFPPMWBS866RA
created: 2026-11-24T04:03:50.467Z
created_by: seat:01M85BW3PMN3QBFPPMWBS866RA
delegated_by: seat:01M85BW3PMN3QBFPPMWBS866RA
id: 01M85VJT6DHSQN7F1ZG1T1239W
number: 1794292861627847
role: executor
status: claimed
swarm: 01M7P04CW1T6KY8CMSS6DQM12Q
title: Closing a Codex seat leaves its MCP servers running inside the checkout
type: task
updated: 2026-11-24T08:41:39.255Z
updated_by: seat:01M85BW3PMN3QBFPPMWBS866RA
verify: make test-boundaries
provenance:
  digest: sha256:5153bb6cf3132a8436a9522fee1f70e0a381f3f8a44a60f55805fdfb8e69eb1e
  frontmatter_digest: sha256:bf3960030efdfb010160957c40633f46c45653f181c85401fe47f5c58ad0d540
  signer: seat:01M85BW3PMN3QBFPPMWBS866RA
  sig: N/+tJM0ZiCrhQoeYwu4YSNhsM2vU6pTIf/QwthgVPuDiflJPMa/WvuCeTjlSumHpT/HWHRGy1nPYXdHEdBw+Ag==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1652
  event:
    id: 01M86BFGVCX4R94YNX28R1RHVK
    sequence: 1652
    hash: d89f4b5cae3f44d8672a10663099fc98a0a476e8de25b92c870b2f6fa94970cf
---
Our stand: subtract before you add; report what you removed.

## Evidence (2026-11-23, observed)
After `seat_close` of Codex executor `01M85NB8C9RRE0976A6G6XYDY5`, workspace `01M85NB1GVD9K2HPXW6QW1BKSC` release failed `workspace_in_use`.
`lsof +D` showed 6 processes, all children of the shared `codex app-server --managed-daemon` (pid 16991, up 3h26m), started 1h48m earlier for that seat:
`bboxy450-term mcp`, `bboxy450-term code`, `paperhands mcp`, `rea mcp`, 2× `node_repl`, cwd = the checkout. The Oracle killed those exact PIDs; release then succeeded.
Every Codex seat close leaks its MCP servers until the app-server daemon exits.

## Do
1. Find where seat close ends a Codex seat (thread close / process stop); cite file:line. Make close end the seat's app-server thread so its MCP children exit (Codex protocol call, or reap the exact children bound to that thread).
2. Process witness: spawn a Codex seat in a temp checkout, close it, assert no process has its cwd under the checkout within N s; broken control without the fix leaves them.
3. `make land`.