---
allowed_paths:
- crates/bboxy450-harness-wake-up
- crates/bboxy450-term
- crates/bbt-mux
- verification
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
updated: 2026-11-24T08:48:05.650Z
updated_by: seat:01M85BW3PMN3QBFPPMWBS866RA
verify: make test-boundaries
provenance:
  digest: sha256:0ebdf3ef35aa5b8f9aaa141bd00e62966d915b47cdd26a89ba45a22b4c7871e5
  frontmatter_digest: sha256:5d0a18a1184ab0cb4cf4a0fde32a7f06d1e21a2b8782f8b9146c00a01670f104
  signer: seat:01M85BW3PMN3QBFPPMWBS866RA
  sig: YWcbllGWcoC4J9dnf4FRWq10/mWsVayc1Gy/qMOFkJTdDMEvlTc8tEesj7lWLQlDGuTWVzJoeX6AHDXsqH5BCw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1660
  event:
    id: 01M86BVA8F271C8PBYTXD86TXK
    sequence: 1660
    hash: 15a1b1ec9d964e01b9a4c438debc5ff4d048b11552320f57de9faed1334d88fd
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

## Oracle ruling (2026-11-24) — supersedes the open decision
Do NOT add supervisor-owned Codex runtimes per checkout (more processes, new concept). Instead:
1. Start the shared Codex app-server daemon with cwd `/` (`launch.rs:912`), exactly as `scripts/cache-env.sh` now does for kache (2ebb9f7db). Seat cwd stays per thread.
2. On Codex seat close, send `thread/archive` for that seat's thread before ending the relay (`logic.rs:13071` → `supervisor.rs:2805,2854`), so its MCP children exit.
3. Process witness: after close, no process has cwd or an open file under the checkout (lsof +D empty within N s); broken control without archive leaves the MCP child.
Existing daemon-held checkouts: the Oracle handles them; not your scope. Keep interfaces/app-harness consistent (the daemon stays shared).