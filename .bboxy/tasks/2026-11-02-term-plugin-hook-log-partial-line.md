---
allowed_paths:
- crates/bbt-mux/tests/plugin_host.rs
- crates/bbt-mux/tests/
- .bboxy/
assignee: seat:01M6ADTTF9QWWFZFVZJHJ3DD8E
authorship:
  kind: seat
  process_generation: '2'
  public_key: V9Fswn0eQw6hTzOemJw9n9Ceh/T/tel4fwr1bzL3B+o=
  signer: seat:01M6AC3D2ECK4W3F8495PAQTS3
created: 2026-11-02T03:54:32.029Z
created_by: seat:01M6AC3D2ECK4W3F8495PAQTS3
delegated_by: seat:01M6AC3D2ECK4W3F8495PAQTS3
id: 01M6D69Z3N4FFA8Q886BFRFN3G
number: 5165
role: executor
status: done
swarm: 01M6ABW0CX6MY0D5WGCSQ3ADTG
title: 'E1c: plugin_host test parses a partially written hook-log line (flaky EOF)'
type: task
updated: "2026-11-02T18:30:00Z"
updated_by: "seat:01M6EQFA2G00BFT3TVPY6AMXE6"
verify: cargo test -p bboxy450-term-mux --test plugin_host
reconciliation: [{"by": "seat:01M6EQFA2G00BFT3TVPY6AMXE6", "disposition": "implemented", "evidence": ["https://github.com/bboxy450/bboxy450-term/commit/6ec9be18"], "fresh_accepted": false, "kind": "historical", "previous_status": "claimed", "reason": "hook-log framing fixture made deterministic on main. Hand-edited: task_reconcile refused (see tasks/2026-11-02-bug-task-lifecycle-unowned-records)."}]
---
ROLE: EXECUTOR. Repo bboxy450/bboxy450-term. Start after #5117 is reported. Worktree: `git -C ~/mydev/bboxy450-term worktree add ~/mydev/bboxy450-term-hooklog -b plugin-hook-log-partial origin/main`. Open a PR. Do not merge or run land.sh.

**Defect.** E3 hit this during a #5104 full run at 18321f7c (log `~/mydev/bboxy450-term-lc-p3/target/ci-local-p3-183-rings-hooks.log`). The failing test is `hooks_receive_harness_and_bus_events_with_payload_only_when_granted`. It panicked at `crates/bbt-mux/tests/plugin_host.rs:183` with "EOF while parsing a string, column 30". The same test passed in the previous run.

**Mechanism.**
- `hooks_received()` (plugin_host.rs:~178-185) reads the fixture plugin's hook log while the plugin may still be appending.
- It `unwrap()`s `serde_json::from_str` on every line, including a final line that has no newline yet.
- `wait_for_hooks` polls it every 20 ms, so a poll that lands mid-write panics.
- It's more likely under CPU load.

**Fix.**
- Parse only newline-terminated lines and ignore a trailing partial line, which the next poll picks up.
- A complete line that fails to parse must still fail loudly.
- Check the fixture plugin's writer too. If it writes a record in more than one write call, make each record a single write.

**Tests.**
- Targeted only, per the CPU rule.
- `cargo test -p bboxy450-term-mux --test plugin_host` passes 20 times in a row.
- Broken control: a writer that emits half a record, pauses, then emits the rest must pass with the fix and panic without it. Test it with a unit test of the reader helper.
- Run one full `make test` on the final commit, and only when `pgrep -fl 'ci-local|make test'` shows no other full run.

**Acceptance.**
- Signed commits.
- A typed report in `.bboxy/reports/` plus the task copy, with a `Report:` trailer.
- One task_report.
- Delete your own `target/` after the gate log is recorded.