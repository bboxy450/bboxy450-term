---
allowed_paths:
- scripts/resource-ledger.py
- scripts/test-resource-ledger.py
- scripts/
- verification/
- .bboxy/
assignee: seat:01M6ADTTF9QWWFZFVZJHJ3DD8E
authorship:
  kind: seat
  process_generation: '2'
  public_key: V9Fswn0eQw6hTzOemJw9n9Ceh/T/tel4fwr1bzL3B+o=
  signer: seat:01M6AC3D2ECK4W3F8495PAQTS3
created: 2026-11-02T00:14:12.381Z
created_by: seat:01M6AC3D2ECK4W3F8495PAQTS3
delegated_by: seat:01M6AC3D2ECK4W3F8495PAQTS3
id: 01M6CSPGDM9Z8J1WT9NDTQDKHJ
number: 5117
role: executor
status: done
swarm: 01M6ABW0CX6MY0D5WGCSQ3ADTG
title: 'E1h: fix resource-ledger child-start vs SIGTERM-handler race (bboxy450-term)'
type: task
updated: "2026-11-02T18:30:00Z"
updated_by: "seat:01M6EQFA2G00BFT3TVPY6AMXE6"
verify: make test
reconciliation: [{"by": "seat:01M6EQFA2G00BFT3TVPY6AMXE6", "disposition": "implemented", "evidence": ["https://github.com/bboxy450/bboxy450-term/commit/0e3acf84", "https://github.com/bboxy450/bboxy450-term/commit/e653ebf5"], "fresh_accepted": false, "kind": "historical", "previous_status": "claimed", "reason": "ledger signal forwarding is installed before child creation on main. Hand-edited: task_reconcile refused (see tasks/2026-11-02-bug-task-lifecycle-unowned-records)."}]
---
ROLE: EXECUTOR. Repo bboxy450/bboxy450-term. Worktree: `git -C ~/mydev/bboxy450-term worktree add ~/mydev/bboxy450-term-ledger-race -b resource-ledger-race origin/main`. Open a PR. Do not merge or run land.sh.

Defect, found by E6 on the log-converter full gate. `scripts/resource-ledger.py:442` starts the child before the SIGTERM handlers are installed at `:458`. The child's readiness print can outrun the wrapper's setup, and a SIGTERM in that window kills the wrapper with the default action, giving -15 instead of the forwarded 143. `scripts/test-resource-ledger.py:222` then fails intermittently, which blocks every branch's full gate.

Fix:
- Install the signal handlers before spawning the child. Alternatively, block the signals and unblock them after the handlers are registered, so no SIGTERM is ever handled by the default action.
- A signal that arrives before the child exists must still produce the defined forwarded or teardown outcome.
- Keep the existing semantics otherwise.

Tests:
1. A deterministic witness that forces the window: signal the wrapper immediately after spawn, many iterations, with no sleeps-as-timing. The exit must always be 143 (or the defined code).
2. A broken control that moves the handler install back after the spawn. It must go red at the named assertion.

If this lifecycle has a TLA+ model in verification/, update it with a broken control. Otherwise record it as implementation-only in coverage.toml, one entry per line.

Acceptance:
- Full `make test` exits 0.
- `python3 -I scripts/test-resource-ledger.py` run 50 times in a row exits 0 every time.
- All commits are signed.
- The report is a typed report in bboxy450-term `.bboxy/reports/` plus the task copy, with a `Report:` trailer, following E6's log-converter pattern.
- Send one task_report with verify_result.command exactly `make test`.