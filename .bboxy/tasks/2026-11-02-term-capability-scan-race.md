---
allowed_paths: ["scripts/test-capability-scan.py","scripts/e2e/capability_scan.py","scripts/",".bboxy/"]
assignee: "seat:01M6CTK0S45RPQ1ZZK2P6NABXJ"
authorship: {"kind":"seat","process_generation":"2","public_key":"V9Fswn0eQw6hTzOemJw9n9Ceh/T/tel4fwr1bzL3B+o=","signer":"seat:01M6AC3D2ECK4W3F8495PAQTS3"}
created: "2026-11-02T00:29:40.218Z"
created_by: "seat:01M6AC3D2ECK4W3F8495PAQTS3"
delegated_by: "seat:01M6AC3D2ECK4W3F8495PAQTS3"
id: "01M6CTJVCGKPXXHE43XR7JA1Y3"
number: 5119
provenance: {"digest":"sha256:eebea6a836ca43baa8387e20450e30aa6804b6ce4f51b98fb243fa02cb116a3b","frontmatter_digest":"sha256:26b5fdcd41fd6457e58626dc975ec87391d9aed54ec66410f6ff00941c5d8bc0","log":".bboxy/log/2026-11-02.jsonl#33806","sig":"bi/jWAtClPy6OoN/sk4tV2Lq62DonKHeqqmoKB/mUyvE6V0j1MPlOPAHbbhna9P1UtLAPCtSpLsiuTKVdbzoDg==","signer":"seat:01M6AC3D2ECK4W3F8495PAQTS3"}
role: "executor"
status: "claimed"
swarm: "01M6ABW0CX6MY0D5WGCSQ3ADTG"
title: "E1i: fix flaky test-capability-scan (process snapshot EIO race) on bboxy450-term main"
type: "task"
updated: "2026-11-02T00:29:48.905Z"
updated_by: "seat:01M6AC3D2ECK4W3F8495PAQTS3"
verify: "make test"
---

ROLE: EXECUTOR. Repo bboxy450/bboxy450-term. Worktree: `git -C ~/mydev/bboxy450-term worktree add ~/mydev/bboxy450-term-capscan -b capability-scan-race origin/main`. Open a PR. Do not merge or run land.sh. Do this after or alongside #5117; both block every landing.

Defect, found by the Oracle while landing PR #89: on main, `python3 -I scripts/test-capability-scan.py` fails in 3 of 30 runs.
- The failing test is `test_real_legacy_value_is_detected_without_printing_it` (test-capability-scan.py:51).
- `scan.scan_process(child.pid, …)` raises `OSError: [Errno 5] owned process snapshot unavailable`, from KERN_PROCARGS2 sysctl at capability_scan.py:39-40.
- Likely cause: the scan reads the child before the child has exec'd its final image or settled, or after it has exited. The sibling test may wait for a readiness marker that this test doesn't.

Fix:
- Make the test wait for a deterministic readiness signal from the child (for example, it prints a line after exec and then blocks on stdin) before scanning, and keep the child alive until the scan returns.
- If the product scanner can legitimately meet EIO for a just-spawned process, handle it per the existing labelled-process rules: retry until the process is observable, or prove it exited. Never skip silently. Don't weaken the scan's fail-closed behaviour.

Tests: 200 consecutive runs of `python3 -I scripts/test-capability-scan.py`, all exit 0. A broken control that removes the readiness wait must reproduce the EIO failure (deterministically, for example with an immediate scan of a pre-exec child).

Acceptance:
- Full `make test` exits 0.
- All commits are signed.
- A typed report in bboxy450-term `.bboxy/reports/` plus the task copy, with a `Report:` trailer.
- Send one task_report with verify_result.command exactly `make test`.

