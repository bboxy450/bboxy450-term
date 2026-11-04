---
allowed_paths: ["crates/bbt-mux/src/logic/ring_batch_tests.rs","crates/bbt-mux/src/logic/","verification/",".bboxy/"]
assignee: "seat:01M6CTK0S45RPQ1ZZK2P6NABXJ"
authorship: {"kind":"seat","process_generation":"2","public_key":"V9Fswn0eQw6hTzOemJw9n9Ceh/T/tel4fwr1bzL3B+o=","signer":"seat:01M6AC3D2ECK4W3F8495PAQTS3"}
created: "2026-11-02T03:49:48.563Z"
created_by: "seat:01M6AC3D2ECK4W3F8495PAQTS3"
delegated_by: "seat:01M6AC3D2ECK4W3F8495PAQTS3"
id: "01M6D61A7QD3S4A53S90P2NAWN"
number: 5162
role: "executor"
status: done
swarm: "01M6ABW0CX6MY0D5WGCSQ3ADTG"
title: "E7: make ring_batch progress tests turn-bounded, not wall-clock (CPU-load flake)"
type: "task"
updated: "2026-11-02T18:30:00Z"
updated_by: "seat:01M6EQFA2G00BFT3TVPY6AMXE6"
verify: "cargo test -p bboxy450-term-mux --lib logic::ring_batch_tests"
reconciliation: [{"by": "seat:01M6EQFA2G00BFT3TVPY6AMXE6", "disposition": "implemented", "evidence": ["https://github.com/bboxy450/bboxy450-term/commit/3ec66dbe", "https://github.com/bboxy450/bboxy450-term/commit/3f025918"], "fresh_accepted": false, "kind": "historical", "previous_status": "claimed", "reason": "ring_batch progress tests are bounded by runnable owner turns on main. Hand-edited: task_reconcile refused (see tasks/2026-11-02-bug-task-lifecycle-unowned-records)."}]
---

ROLE: EXECUTOR. Repo bboxy450/bboxy450-term. Start after #5119 is reported. Worktree: `git -C ~/mydev/bboxy450-term worktree add ~/mydev/bboxy450-term-ringbatch -b ring-batch-turn-bound origin/main`. Open a PR. Do not merge or run land.sh.

**Defect.** E3 hit this during a #5104 full run at 18321f7c (`~/mydev/bboxy450-term-lc-p3/target/ci-local.log:3039-3064`). Two tests failed:
- `logic::ring_batch_tests::ring_reads_progress_independently` (107 pending)
- `logic::ring_batch_tests::ring_reads_progress_independently_per_host` (92 pending)

Both failed at `crates/bbt-mux/src/logic/ring_batch_tests.rs:417`. Both passed in the previous run on the same head.

**Mechanism.** `sweep()` (ring_batch_tests.rs:~385) drives 967 seats on a 2-worker Tokio runtime, under a wall-clock deadline: `Instant::now() + 15s`. With 5 or more concurrent full `make test` runs from 3 swarms, the CPU is starved, so the deadline expires while progress is still correct. The property the test claims is progress per owner turn, which is a turn count, not time.

**Fix.**
- Bound the loop by a turn budget derived from SEATS. For example, each seat must drain within N × SEATS turns, with N stated and justified from the ingest_one contract of one page per turn.
- Keep a very generous wall-clock backstop only as a hang guard, for example 300 s.
- Don't weaken the "one page/notification per turn" assertion or the independence property.
- If a formal model or coverage entry names this progress bound, update it.

**Tests.**
- Targeted runs only, per the CPU rule. `cargo test -p bboxy450-term-mux --lib logic::ring_batch_tests` passes 20 times in a row while the machine is under load (other swarms' runs count).
- Broken control: a reader that starves one seat, for example skipping odd seats, must fail the turn bound deterministically.
- Run one full `make test` on the final commit, and only when `pgrep -fl 'ci-local|make test'` shows no other full run.

**Acceptance.**
- Signed commits.
- A typed report in `.bboxy/reports/` plus the task copy, with a `Report:` trailer.
- One task_report.
- Delete your own `target/` after the gate log is recorded.

