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
closed:
  by: seat:01M85BW3PMN3QBFPPMWBS866RA
  op_id: 01M86NEHQAYQX7G8CY5FS3NCEZ
created: 2026-11-24T04:03:50.467Z
created_by: seat:01M85BW3PMN3QBFPPMWBS866RA
delegated_by: seat:01M85BW3PMN3QBFPPMWBS866RA
id: 01M85VJT6DHSQN7F1ZG1T1239W
number: 1794292861627847
report:
  commit: 3c3cc4851f0ffeffc1d292506d05e63aa4c101ba
  op_id: 01M86KF9B8M6TFXEV1WM0ZVC33
  report: '[[reports/2026-11-24-codex-close-checkout-handles.den]]'
  reported_by: seat:01M86BFF3067ZDDF0HSF73WVHP
  summary: '[[Implemented]] Landed 3c3cc4851f0ffeffc1d292506d05e63aa4c101ba. Real Codex 0.169.0 close left zero checkout holders within 15 seconds, with the shared daemon at / and a peer still live after undo. The bare-TUI-stop control retained its MCP holder for the full window. Removed checkout cwd inheritance and duplicate close scheduling. `make test-boundaries`, `make test-licenses`, and `make land` each exited 0 on the final runs. Native/TLC parity matched all 1003 models. The full report is indexed locally. Its KB checkpoint was rejected because reports are Git-ignored.'
  verify_revision: 0
role: executor
status: done
swarm: 01M7P04CW1T6KY8CMSS6DQM12Q
title: Closing a Codex seat leaves its MCP servers running inside the checkout
type: task
updated: 2026-11-24T11:35:54.672Z
updated_by: seat:01M85BW3PMN3QBFPPMWBS866RA
verify: make test-boundaries
verify_result:
  command: make test-boundaries
  evidence:
  - criterion: Cold daemon cwd and exact final-close archive
    reference: crates/bboxy450-harness-wake-up/src/launch.rs:920 and crates/bbt-mux/src/logic/codex_close.rs:33
  - criterion: No checkout holders after close; omitted archive leaks
    reference: target/codex-close-boundaries-final.log:333 and target/codex-close-landed-witness-final.log:5
  - criterion: Landed signed commit and formal parity
    reference: target/codex-close-land-final.log:1087 and target/codex-close-land-final.log:1878
  exit_code: 0
provenance:
  digest: sha256:ee4de0fd74bb71f03b79489b031bd62f23a3095e94a0eb51fc27d9e333b006b5
  frontmatter_digest: sha256:3d462aac9fd73ddfa10cd1aa6bbb4b9f414147e251fbdce72e58d78e8040f30c
  signer: seat:01M85BW3PMN3QBFPPMWBS866RA
  sig: JbRNbFahkV9Zee9z1ZfFNRCJgSTxJdDEvFxqgonC2/YleIXMY5+Q4rd2rMSmvBE3dt0MFBwLWfQ1xOGibzyQDQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1704
  event:
    id: 01M86NEK5HY207AHD4P04S887K
    sequence: 1704
    hash: 2bc6d899055d9666e0e848f7fdfb990ba0414aefd9917ffa3f6d7e906f216fb8
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

## Report from seat:01M86BFF3067ZDDF0HSF73WVHP

[[Implemented]] Landed 3c3cc4851f0ffeffc1d292506d05e63aa4c101ba. Real Codex 0.169.0 close left zero checkout holders within 15 seconds, with the shared daemon at / and a peer still live after undo. The bare-TUI-stop control retained its MCP holder for the full window. Removed checkout cwd inheritance and duplicate close scheduling. `make test-boundaries`, `make test-licenses`, and `make land` each exited 0 on the final runs. Native/TLC parity matched all 1003 models. The full report is indexed locally. Its KB checkpoint was rejected because reports are Git-ignored.

```json
{
  "op_id": "01M86KF9B8M6TFXEV1WM0ZVC33",
  "commit": "3c3cc4851f0ffeffc1d292506d05e63aa4c101ba",
  "report": "[[reports/2026-11-24-codex-close-checkout-handles.den]]",
  "verify_result": {
    "command": "make test-boundaries",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Cold daemon cwd and exact final-close archive",
        "reference": "crates/bboxy450-harness-wake-up/src/launch.rs:920 and crates/bbt-mux/src/logic/codex_close.rs:33"
      },
      {
        "criterion": "No checkout holders after close; omitted archive leaks",
        "reference": "target/codex-close-boundaries-final.log:333 and target/codex-close-landed-witness-final.log:5"
      },
      {
        "criterion": "Landed signed commit and formal parity",
        "reference": "target/codex-close-land-final.log:1087 and target/codex-close-land-final.log:1878"
      }
    ]
  }
}
```
