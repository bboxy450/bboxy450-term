---
assignee: seat:01M6D4J7CTD3BX29WGVYXZ3TC9
authorship:
  kind: seat
  process_generation: '1'
  public_key: J+Eete7RzpCHzcNSDpuEpx+ye6zu+UTGX+qDDQkPrw4=
  signer: seat:01M6CQ90BPKR0WZ70BDJD6R7MV
created: 2026-11-02T03:24:04.063Z
created_by: seat:01M6CQ90BPKR0WZ70BDJD6R7MV
delegated_by: seat:01M6CQ90BPKR0WZ70BDJD6R7MV
id: 01M6D4J5ZEKSQWEHDSYXZN6N4X
number: 3145
provenance:
  digest: sha256:0d0babb804f97862b80544ad8fdc058c4e4489dbe5d3237815a5749f17ebbba7
  frontmatter_digest: sha256:ede07da195f561416dc60f00430c3d109d3bf8c5c07129ab7d57a165fd430a22
  log: .bboxy/log/2026-11-02.jsonl#13700
  sig: 2+n96I+ZorT56XVl1JLaZknvS2ikPfb8KVy17NXgwPgeUD9/2zETR2O47sIstLLqC3tN1KY+bGXZiszHCI8VAg==
  signer: seat:01M6CQ90BPKR0WZ70BDJD6R7MV
role: executor
status: claimed
swarm: 01M649102FBS60QWNPZS5VAGF9
title: Chat renders message seconds as milliseconds; every message shows ~the same 1970 time
type: task
updated: 2026-11-02T03:24:08.483Z
updated_by: seat:01M6CQ90BPKR0WZ70BDJD6R7MV
verify: make test
---
# Chat timestamps: seconds rendered as milliseconds

ROLE: EXECUTOR. Work only in a new worktree `~/mydev/bboxy450-term-chat-ts` on branch `chat-ts`, created from `origin/main`. Work on your own and record your decisions in the report.

## Bug (operator, 2026-11-01): chat messages show almost the same timestamp

Mechanism (Oracle, verified against source):
- `crates/bbt-mux/src/messaging/service.rs:1146` sets `MessageRow.ts = message.view().timestamp`. The envelope timestamp is in **seconds** (`timestamp: now / 1000`, `service.rs:3227/3331/3410`; `bbt-proto/src/messaging.rs:269`). `MessageRow.ts` (`bbt-proto/src/messaging.rs:698`) has no unit in its name or docs.
- `crates/bboxy450-term/src/messaging.rs:238-239` passes `row.ts` to `crate::local_time::hm`, which takes **milliseconds** (`local_time.rs:28`).
- Effect: sent 15:35, 16:20 and 18:46 all render as 10:12-10:13, because they are read as times on 1970-01-21. A whole day of messages fits in about 86 seconds of display time.
- Second effect: `sidebar_chat.rs:31` compares `data.ts` (seconds) with `notice.ts_ms` (milliseconds), and `:369` orders entries by `data.ts`. Notices and messages sort wrongly against each other.

## Required outcome

1. One unit end to end. Preferred: `MessageRow` carries an explicitly named `ts_ms` or documented milliseconds, converted once at the row boundary from the signed envelope seconds. Do not change the signed envelope's `timestamp` semantics: it is part of the signature bytes. Keep wire compatibility for `MessageRow` consumers (ctl/MCP JSON) or version it deliberately, and record which.
2. The UI renders the message's own timestamp (when it was sent or admitted), never the render or receive time. Check every chat surface: the sidebar chat, the conversation view, the composer history (`messaging/history.rs`) and ctl/MCP `message_list` output. Check that the hm formatting has the date context it needs for messages from earlier days, if the design shows dates.
3. Fix the notice/message ordering comparison in `sidebar_chat.rs:31` and `:369`.
4. Tests: a unit test with two messages an hour apart that render different `hm` values (it fails on main); an ordering test with a notice between two messages; a real-path test that sends a message through the messaging service and checks the rendered time against its envelope time.
5. Formal (d365): only if ordering or arbitration semantics change. Otherwise record in the report why no model changed.

## CPU budget (operator)

The machine is CPU-constrained. While iterating, run only targeted tests (`cargo test -p bboxy450-term <filter>`, `-p bbt-mux`, `-p bbt-proto`). Run full `make test` once, on the final commit. Before starting it, check `pgrep -fl "ci-local|make test"`, and wait if another full run is active.

## Allowed paths

`crates/bboxy450-term/**` (messaging, chat, local_time), `crates/bbt-mux/src/messaging/**`, `crates/bbt-proto/src/messaging.rs`, tests for those, `verification/**`, `.bboxy/reports/**`, and this exact task record copied byte-for-byte to `.bboxy/tasks/2026-11-01-chat-timestamps-seconds-as-ms.md` (the report gate needs it in-tree).

## Verification

`make test` on the exact committed source, exit 0. Strict clippy on touched crates.

## Done

Signed commits on `chat-ts`, a report in `.bboxy/reports/` with a `Report:` trailer, and `task_report`. Do not merge or push.
