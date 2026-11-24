---
allowed_paths:
- crates/bbt-mux
- crates/bboxy450-term
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
status: open
swarm: 01M7P04CW1T6KY8CMSS6DQM12Q
title: Closing a Codex seat leaves its MCP servers running inside the checkout
type: task
updated: 2026-11-24T04:03:50.467Z
updated_by: seat:01M85BW3PMN3QBFPPMWBS866RA
verify: make test-boundaries
provenance:
  digest: sha256:5153bb6cf3132a8436a9522fee1f70e0a381f3f8a44a60f55805fdfb8e69eb1e
  frontmatter_digest: sha256:49d1bede1c3fcfc901bbf3bc3c5927d77c6213ed0cb7636ada8d52bffe4aff39
  signer: seat:01M85BW3PMN3QBFPPMWBS866RA
  sig: aHeDX9EidSrHQFZEQeD2MYrb9Nhil34fQSm2OS88Fl9uj6923w1504NCAmefY9TlkausDN9+aWm3ThNQHzJSDw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1549
  event:
    id: 01M85VJTVWFVFEVPWAZR7PK32B
    sequence: 1549
    hash: 9404b2527c3cf53dec3bd3e71164bf5e86c620e7293a00bf885814e76b15f143
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