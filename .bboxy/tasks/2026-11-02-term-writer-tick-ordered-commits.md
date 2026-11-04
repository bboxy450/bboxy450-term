---
allowed_paths:
- crates/bbt-supervisor/src/ring/writer.rs
- crates/bbt-supervisor/src/ring/
- verification/
- .bboxy/
assignee: seat:01M6ADTTF9QWWFZFVZJHJ3DD8E
authorship:
  kind: seat
  process_generation: '2'
  public_key: V9Fswn0eQw6hTzOemJw9n9Ceh/T/tel4fwr1bzL3B+o=
  signer: seat:01M6AC3D2ECK4W3F8495PAQTS3
created: 2026-11-02T04:05:01.900Z
created_by: seat:01M6AC3D2ECK4W3F8495PAQTS3
delegated_by: seat:01M6AC3D2ECK4W3F8495PAQTS3
id: 01M6D6X67F67ZEA5DAE5H51EMN
number: 5169
role: executor
status: done
swarm: 01M6ABW0CX6MY0D5WGCSQ3ADTG
title: 'E1c: writer tick test counts an extra ordered commit under CPU load'
type: task
updated: "2026-11-02T18:30:00Z"
updated_by: "seat:01M6EQFA2G00BFT3TVPY6AMXE6"
verify: cargo test -p bboxy450-term-supervisor ring::writer
reconciliation: [{"by": "seat:01M6EQFA2G00BFT3TVPY6AMXE6", "disposition": "implemented", "evidence": ["https://github.com/bboxy450/bboxy450-term/commit/6ec9be18"], "fresh_accepted": false, "kind": "historical", "previous_status": "claimed", "reason": "writer tick fixture made deterministic on main. Hand-edited: task_reconcile refused (see tasks/2026-11-02-bug-task-lifecycle-unowned-records)."}]
---
ROLE: EXECUTOR. Repo bboxy450/bboxy450-term. **Same branch as #5117 (`resource-ledger-race`, worktree `~/mydev/bboxy450-term-ledger-race`).** Do #5165 on this branch too: three fixes, one PR, and one final full `make test` covering all three, per the CPU rule. Do not merge or run land.sh.

**Defect.** E1c hit this during the second #5117 full run, in `crates/bbt-supervisor/src/ring/writer.rs:1167`, test `one_writer_tick_orders_two_trickling_seats_without_full_flushes`. It got `ordered_commits=2` where 1 was expected. Three isolated runs pass. Log: `~/mydev/bboxy450-term-ledger-race/target/resource-ledger-race/make-test-second-ci.log`.

**Likely mechanism, which you must confirm from the source.** The test calls `tick(&writer)` once and asserts exactly one ordered commit per seat. Under CPU load, the CaptureWriter's own timer or wake path probably runs a second tick, or splits the two appends across ticks, before the test's tick lands. Find which one it is: read the writer loop's tick triggers and cite file:line.

**Fix.**
- Make the test deterministic about the tick it measures. For example, stop the writer's own timer from firing during the test, or drive ticks only through the test hook, or wait until both appends are queued before the tick.
- Keep the property the test claims: both trickling seats get ordered by one tick with no full (durable) flush.
- Don't loosen it to `>= 1` unless the source proves that extra ordered commits are allowed by the spec (`mux-daemon.md`, Ring records). If you do, cite the spec line.
- If the product writer itself can double-tick in a way the model forbids, report that with the evidence instead of patching the test.

**Tests.**
- Targeted: the writer tests pass 50 times in a row, both under load and alone.
- Broken control: a forced extra tick between append and measurement must fail the assertion you keep.
- Then run one full `make test` for the whole branch (#5117 + #5165 + this), only when `pgrep -fl 'ci-local|make test'` shows no other full run.

**Acceptance.**
- Signed commits.
- One typed report covering the three tasks, with a `Report:` trailer.
- One task_report per task, all citing the same gate log.
- Delete your own `target/` after the gate log is recorded.