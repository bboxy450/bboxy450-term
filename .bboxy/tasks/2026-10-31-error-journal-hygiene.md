---
assignee: seat:01M6ABQRB8SJNZHP6NRHQD3BC8
authorship:
  kind: seat
  process_generation: '1'
  public_key: dNWbVeHRHyt/TnV6QHFyDXtWIj+UElZjvaWtEgWhcFg=
  signer: seat:01M6ABH71TVWGZ621NTD701PSH
created: 2026-11-01T01:31:38.218Z
created_by: seat:01M6ABH71TVWGZ621NTD701PSH
delegated_by: seat:01M6ABH71TVWGZ621NTD701PSH
id: 01M6ABQK94JATCJFWMSWQQSN54
number: 509
provenance:
  digest: sha256:5a46864384ee89cbe2c7c4e3369ca7698f5083c9b7d009495af8d00d65dc4671
  frontmatter_digest: sha256:f31f5f22a301c1cd4ff00daf78d6ab69b469ae523cb87d32fe4a140c49e0935a
  log: .bboxy/log/2026-11-01.jsonl#51
  sig: rPcl74iYAgMIqBezlonto5vXfc9TvjgLKzam75yVeXV7vKbxoeU3/MS1fgMPMGMWTqnkjb6THsdVobQPvN8uCA==
  signer: seat:01M6ABH71TVWGZ621NTD701PSH
role: executor
status: claimed
swarm: 01M649102FBS60QWNPZS5VAGF9
title: 'Error journal: keep test fixtures out of the operator journal; do not journal server/discover probes'
type: task
updated: 2026-11-01T01:31:51.054Z
updated_by: seat:01M6ABH71TVWGZ621NTD701PSH
verify: make test-boundaries
---
# Error journal hygiene from `ctl debug errors` triage

ROLE: EXECUTOR. Worktree `~/mydev/bboxy450-term-journal`, branch `journal-hygiene` (from origin/main 42320d93). Only this worktree.

## Evidence (Oracle, 2026-10-31, `bboxy450-term ctl debug errors --since 0`, 188 rows)
- Fixture rows in the operator's real journal: `source:provider op:request "LinkDown: Cloud { provider: \"nebula\", host: \"worker-east\" }"` and `worker-west` (pid 34930, at_ms 1793292533697), `LinkDown: Ssh { host: \"build-box\" }` (pid 88923, 1793333121608). These hostnames exist only in `crates/bbt-proto/src/hosts.rs`, `crates/bbt-mux/tests/provider*.rs`, `crates/bboxy450-term/tests/hosts.rs`. Same run wrote `source:ctl op:app/status "[redacted sensitive diagnostic line]"` (pids 34892/34903/34931, ms apart) and later pid 20911; plus `ctl command "usage: app verbs: …"` (pid 77247). An earlier fix 6d8e49e8 isolated malformed ctl diagnostics but tests still reach the operator journal.
- `source:mcp op:server/discover "Method not found"` from a live Claude seat (at_ms 1793496490746): a client capability probe journaled as an error.
- Stale, no action: 113 `hook input bound` codex-notify rows all before #80 fix fe183086; 66 MCP refusals are correct refusals of agent misuse.

## Required outcome
1. Every test that spawns ctl/app/provider code resolves its error journal inside its private test home; none can write the operator's journal. Add a guard test that runs the fixture provider + `ctl app status` failure path and asserts the operator-journal path is untouched (broken control: remove isolation → fails).
2. `server/discover` (and other unknown optional JSON-RPC methods that are pure probes) answers per MCP/JSON-RPC but is not written as an error row; real tool errors still journal. Test both.
3. Find the cause of the `app/status` redacted rows; if it is the same leak, item 1 covers it; if it is a real product failure, fix it or report the exact cause.

## Verify
`make test-boundaries`, the new tests, strict clippy on touched crates, `make test` on committed source. Land with `scripts/land.sh` if green.

## Communication (operator order: no token burn)
No progress DMs, no channel posts, no acknowledgements. One `task_report` at the end: files+mechanism, commands+exit codes, evidence per item, gaps, commit. Message the Oracle only for a blocker that stops all work, in ≤3 lines.
