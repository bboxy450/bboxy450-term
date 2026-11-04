---
author: "seat:01M6CTK0S45RPQ1ZZK2P6NABXJ"
authorship: {"kind":"seat","process_generation":"1","public_key":"6zlI3Xtz61NbNKB19xGmAaUggcsK+9h71GpWZr81k1I=","signer":"seat:01M6CTK0S45RPQ1ZZK2P6NABXJ"}
commits: []
created: "2026-11-02T00:37:34.768Z"
created_by: "seat:01M6CTK0S45RPQ1ZZK2P6NABXJ"
id: "01M6CV1997VSNM9PV7C7EW3G60"
number: 5123
provenance: {"digest":"sha256:488ca38677b708227b4452719f43d8019f1762cbfc4f8aad800e56b8d8e1643e","frontmatter_digest":"sha256:bef78d39df711b7bcee57b7303a58e645393f8d565738945c39656cead8e3bbf","log":".bboxy/log/2026-11-02.jsonl#33837","sig":"6Ut7umNhuvQTezgYxdG4WQov+iWgQ5m785xp6ZAs+OK5dZNqVCor7CCG8NimQITID/uJJMY2wL5Oyb4moqmwCA==","signer":"seat:01M6CTK0S45RPQ1ZZK2P6NABXJ"}
task: "[[tasks/2026-11-02-term-capability-scan-race]]"
title: "Capability scan readiness and bounded EIO retries"
type: "report"
updated: "2026-11-02T00:37:34.768Z"
updated_by: "seat:01M6CTK0S45RPQ1ZZK2P6NABXJ"
verify: {"command":"python3 -I scripts/test-capability-scan.py (200 consecutive runs)","exit_code":0}
---

# Capability scan readiness and bounded EIO retries

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

## Final committed-source gate

This append-only report records the focused verification completed before its containing commit. After committing it with a Report trailer, `make test` must run on that exact commit; the final single task_report will supply the actual exit, signed SHA and PR. This report does not claim that future gate has already passed. Its raw output will be retained at `target/ci-local.log`, with `ci-local: source/result <SHA> normal` lines. Full CI includes formal verification and private headless smoke. Separate UI parity and product lifecycle boundary runs are outside this scanner-only change. No merge, land.sh, install or release is authorized.

