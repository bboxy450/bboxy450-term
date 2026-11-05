---
assignee: seat:01M6ENBMM5BKB01BEA87BTAEXB
authorship:
  signer: seat:01M6H97SAX3GN1BYB4YG76P3P0
  kind: seat
  public_key: B3hFSjBIE1Nnj7mw9Eg9pqyRpYHF/GQaKXPyV1KAod0=
  process_generation: '1'
created: 2026-11-02T17:36:48.497Z
created_by: seat:01M6CQ90BPKR0WZ70BDJD6R7MV
delegated_by: seat:01M6CQ90BPKR0WZ70BDJD6R7MV
id: 01M6ENBK7E0D9JMVASZXZDMZYF
number: 3850
reconciliation:
- kind: historical
  fresh_accepted: false
  previous_status: claimed
  disposition: implemented
  reason: Historical implementation is already on signed origin/main at 3e79499f0159ab7ff3362edc3d46d2fcd8235d77; the former executor seat is closed and its claimed task was left stale. This records the implemented disposition, not a new acceptance or an unrun unit-suite pass.
  evidence:
  - git verify-commit 3e79499f0159ab7ff3362edc3d46d2fcd8235d77 — exit 0; approved Boxy key 9B07E849480C68328CA20684844ECC0587DE7AB0
  - git merge-base --is-ancestor 3e79499f0159ab7ff3362edc3d46d2fcd8235d77 origin/main — exit 0
  - origin/main:crates/bbt-mux/src/mcp/seats.rs:382 — seat_close parses exact target/generation; :513 enforces caller swarm
  - origin/main:crates/bbt-mux/tests/mcp.rs:4800 — close control, worker/cross-swarm/self/no-window regressions present
  - Current runtime seat_close operations 01M6HCJZJYEW1EEKP8069WMKX0, 01M6HCJZQFX1XDJ59NRJ13XHDE and 01M6HCJZTRZNWZZPA215SXQHAZ — applied to three live sibling Claude seats; later seat_list shows closed_retained
  - Current runtime operation 01M6HD2733WYQV0PQ9EN2CRAED — rejected target outside caller swarm
  by: seat:01M6H97SAX3GN1BYB4YG76P3P0
  op_id: 01M6HEERZ1A9FN9Q8PQNPZBWJ7
role: executor
status: done
swarm: 01M649102FBS60QWNPZS5VAGF9
title: 'Oracle MCP seat_close: remove a seat in the caller''s swarm'
type: task
updated: 2026-11-03T19:33:57.300Z
updated_by: seat:01M6H97SAX3GN1BYB4YG76P3P0
verify: cargo test -p bbt-mux --test mcp
provenance:
  digest: sha256:db8ce848b5eca2f988385a8cc9c42fbd52b0f118c31607eba0ef43d0942d2006
  frontmatter_digest: sha256:89e10bb7d166da89e3f8e1d58a3de5013eee029228722e5e8c0b962a4be7818d
  signer: seat:01M6H97SAX3GN1BYB4YG76P3P0
  sig: 4tTz8OZOmdgx1Kgq/+qY8TlVqU7hNtH9CwT+3CffQzT2xQw5902N3AaF/oWyX4JPOdDDwzHOV6S2h4jerLz8Dw==
  log: .bboxy/log/2026-11-03.part-86213.jsonl#94696
---
# Oracle MCP seat_close

ROLE: EXECUTOR. Worktree `~/mydev/bboxy450-term-seat-close`, branch `seat-close`, from `origin/main`. Work autonomously; report once on completion.

**Operator: do it fast, cut all corners.** No full `make test`, no formal model, no report document. Targeted tests only. Signed commit, then `task_report`.

## Problem

The Oracle can spawn, split, focus, mute and interrupt seats over MCP (`crates/bbt-mux/src/mcp/seats.rs:28-36`, catalogue `:64`) but cannot close one. Killing a seat's processes leaves the seat in the window. The only close path is the operator's shell `ctl seat close` (`crates/bboxy450-term/src/ctl.rs`, `seat_socket.rs`), which needs the GUI ctl endpoint.

## Outcome

1. New Oracle-only MCP tool `seat_close {seat_id, expected_process_generation}`: closes a seat in the caller's swarm, same effect as `ctl seat close` (the existing close path and its undo window; reuse it, do not write a second close). Refuse the caller's own seat and seats outside the caller's swarm. Must work whether or not the seat's process is still alive (the target case is a dead/killed harness).
2. Add `seat_close` to the `ctl` MCP tool's command list.
3. Add it to the expected tool lists in `crates/bbt-mux/tests/mcp.rs` (`:942`, `:2492`) and one test: Oracle closes a sibling seat; worker is refused; other-swarm seat is refused.

## Allowed paths

`crates/bbt-mux/**`, `crates/bboxy450-term/src/**` only if the close path needs wiring there, the bboxy450 skill doc.

## Verification

`cargo test -p bbt-mux --test mcp` exit 0, plus `cargo check --workspace --all-targets`. Record verify_result.command exactly `cargo test -p bbt-mux --test mcp`. Do not merge or push.
