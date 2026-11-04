---
author: "seat:01M6CTK0S45RPQ1ZZK2P6NABXJ"
authorship: {"kind":"seat","process_generation":"1","public_key":"6zlI3Xtz61NbNKB19xGmAaUggcsK+9h71GpWZr81k1I=","signer":"seat:01M6CTK0S45RPQ1ZZK2P6NABXJ"}
commits: ["3d9316d4ae846f71b4e77a7c2dbd79f6ff372c0c"]
created: "2026-11-02T03:23:01.704Z"
created_by: "seat:01M6CTK0S45RPQ1ZZK2P6NABXJ"
id: "01M6D4G6PV3K38H4P0X7GQM1P1"
number: 5156
provenance: {"digest":"sha256:6175a25d436ad29764ff4bd5321b74f1391446856dfc82a1ee4eebc0f2e2c287","frontmatter_digest":"sha256:85e82c300c0bea9a1f3ce1ec8db67f992967bd7843e29578e857609d57fe23ed","log":".bboxy/log/2026-11-02.jsonl#34156","sig":"0wwrvzng0WkFu0Ceh6iFC4r7nEPpztexz+g0lDbLaTKIffXb53JFz+J9eBrztjPONWEWluXaY30vMa26LDTZDQ==","signer":"seat:01M6CTK0S45RPQ1ZZK2P6NABXJ"}
supersedes: "[[reports/2026-11-02-term-capability-scan-race]]"
task: "[[tasks/2026-11-02-term-capability-scan-race]]"
title: "Capability scan verification and disk-guard abort"
type: "report"
updated: "2026-11-02T03:23:01.704Z"
updated_by: "seat:01M6CTK0S45RPQ1ZZK2P6NABXJ"
verify: {"command":"make test","exit_code":2}
---

# Capability scan verification and disk-guard abort

Supersedes [[reports/2026-11-02-term-capability-scan-race]]. The scanner implementation and its 200-run source hashes are unchanged. This correction records the full-CI result obtained after the first report was committed.

Assignment: [[tasks/2026-11-02-term-capability-scan-race]] (#5119), seat:01M6CTK0S45RPQ1ZZK2P6NABXJ. Worktree: /Users/feral/mydev/bboxy450-term-capscan; branch capability-scan-race; base eb968968.

## Files and mechanisms

- `scripts/test-capability-scan.py:25` starts an owned Python child with private HOME, piped stdin/stdout and bounded cleanup. `child:45` waits up to ten seconds for the child's post-exec ready marker. The child then blocks on stdin throughout the scan. Both real-child tests use this handshake.
- `scripts/test-capability-scan.py:54` allows the macOS Python launcher's two observed additions (`__CF_USER_TEXT_ENCODING`, `__PYVENV_LAUNCHER__`) alongside Darwin auxiliary entries. Waiting for the final interpreter exposed these entries; the test still rejects arbitrary runner variables and checks its exact HOME/marker.
- `scripts/e2e/capability_scan.py:28` retries only syscall EIO at 10 ms intervals for a one-second budget. The original raw parser remains in `_process_snapshot_once`. Persistent EIO and all other errors still propagate to the existing labelled-process exit-proof/failure path; no process is silently skipped and no successful empty snapshot is fabricated.
- `scripts/test-capability-scan.py:76` is the deterministic broken control: an owned child gates a deliberately unreadable exec-argument page using mprotect. Scanning without the readiness wait produces a real KERN_PROCARGS2 errno 5. Restoring the page and waiting for ready permits the same PID to scan successfully. The retry clock is expired by injection for this control; the syscall and errno are real. This reproduces the unavailable-map failure, rather than claiming deterministic scheduling of a native exec transition.
- `scripts/test-capability-scan.py:110` checks retry-to-observation and expiry/non-EIO refusal. `:174` requires a live labelled second bridge's snapshot failure to remain fatal. Exit tests require ESRCH or an actual successful PID census excluding the PID, retain roles and count exits. The legacy-value test also asserts that its failure contains no fixture capability bytes.
- The task copy is in `.bboxy/tasks/2026-11-02-term-capability-scan-race.md`; compact command/exit/source/log hashes are in `.bboxy/runs/2026-11-02-term-capability-scan-race.json`. Existing source attribution headers and the root policy already cover these project-owned scripts.

## Completed verification

- `python3 -I scripts/test-capability-scan.py` — 200 consecutive subprocess runs, every exit 0; nine tests each (1800 test executions). Raw output: `target/capability-scan-200.log`, SHA256 `1c7a5a3b8ab22174e3ea4d2bffe7d664a2450bb4bdbcb64e1abd101db65410d8`.
- `make test-licenses` — exit 0 after `make init-submodules` exited 0; 32 workspace crates and source notice checks passed. Initial license attempt exited 2 because this fresh worktree had no initialized pinned harness submodule; no source change or waiver was needed. Logs: `target/capability-scan-licenses.log` and `target/capability-scan-licenses-ready.log`.
- `git diff --check` — exit 0.

## State contract and coverage limits

The fixture child owns startup and readiness; the parent owns scanning and cleanup. Initial state is starting with stdin open; the ready write follows final exec; scanning follows marker receipt; cleanup terminates/reaps only this exact child. The controlled instance has one PID, one unavailable/readable page and two markers. Safety: an unavailable live process never becomes a successful/omitted observation, and capability bytes never enter diagnostics. Progress assumes the child is scheduled; absent readiness times out, persistent EIO fails after its bounded retry budget. The native negative control removes the readiness ordering and observes EIO. Synthetic checks cover transient/repeated failure, non-EIO refusal, exit and failed exit-recheck.

Runtime admission, generation, ownership and cleanup semantics are unchanged. Existing SeatCapabilityEnvironment/BrokenSeatCapabilityEnvironment and SeatCapabilityCleanup models remain the runtime contracts; Darwin snapshot observability and the scanner's retry timing are implementation-only test-instrument coverage. No new crate, UI geometry or native product process lifecycle is changed.



## First committed-source gate

`make test` on signed commit `3d9316d4ae846f71b4e77a7c2dbd79f6ff372c0c` exited **2**. Workspace build, strict all-target Clippy, workspace tests/doctests, the scanner test, native harness, native/TLC verification and private headless smoke passed. The actual leak-reintroduction control returned red_exit=101 and green_exit=0; its restoration hash is recorded in `target/capability-file/environment-control.json`. The formal receipt records 205 models, 32 crates, 84 contracts and dirty=false; evidence SHA256 is `bc8d549968956e506751f55daab770931106bf7d5057de917c077f62fb85e369`.

The only final invariant failure was `checkpoint_bounded_recovery`, at `crates/bbt-mux/tests/recovery.rs:329`: its guard observed 10,010,890,240 free bytes, below the required 10 GiB (10,737,418,240 bytes). It aborted before running the recovery scenario. The other 26 active invariant checks passed; the standard 11 slow checks were skipped. Raw log: `target/ci-local-disk-refusal.log`, SHA256 `0723ce0e22f4f5d82521db6b55cd296cd11f24fa5c4c25d0007900d5f6f8de65`. The compact command/exit/hash record is `.bboxy/runs/2026-11-02-term-capability-scan-race-ci.json`.

The original CI log was preserved before a rerun. Available disk space had recovered to 33.7 GiB at 2026-11-02T03:22:22Z, so no build artifacts were removed. The recovery test and its disk guard were left unchanged. Disk availability remains a machine-level precondition of that existing test.

## Final committed-source gate

This append-only correction is committed before the next exact-source `make test`. Its result is pending here; the final single task_report will supply the actual exit, signed SHA and PR. `target/ci-local.log` will retain the new source/result lines. The two scanner files must continue matching the 200-run hashes in the committed run summary. There has been no merge, land.sh, install or release. Separate UI parity and product lifecycle targets remain outside this scanner-only change.

