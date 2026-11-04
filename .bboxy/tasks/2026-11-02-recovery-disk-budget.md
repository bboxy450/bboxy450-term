---
acceptance:
- The recovery fixture keeps real durable ring/journal data and all crash/replay assertions.
- The arbitrary 10 GiB guard is replaced by an explicit bounded fixture budget with real filesystem growth detection and measured usage.
- Normal and 50-seat private recovery runs pass; a sparse oversized fixture is rejected without allocating the limit on disk.
- Signed clean changes, required formal evidence and canonical lean landing complete; original chat fix is included.
allowed_paths:
- crates/bbt-mux/tests/recovery.rs
- Makefile
- LAND.md
- .bboxy/tasks/2026-11-02-recovery-disk-budget.md
- .bboxy/reports/2026-11-02-recovery-disk-budget.md
- verification/coverage.toml
- .bboxy/reports/2026-11-02-recovery-disk-budget-integration.md
- scripts/quarantine.py
- scripts/land-fast.sh
- scripts/test-landing.py
- .bboxy/reports/2026-11-02-lean-ignored-suite.md
- .bboxy/reports/2026-11-02-recovery-final-status.md
artifacts:
- '[[01M6F52F46WDZNXJXXBRVMVKPA]]'
assignee: seat:01M6EW8HN9QTQJ7A7GD1ZYMWYR
authorship:
  kind: seat
  process_generation: '2'
  public_key: 8G8gIL1MJqmoRkKikaIqqCqCFW1VHfzquBV0bXH7xs8=
  signer: seat:01M6EW8HN9QTQJ7A7GD1ZYMWYR
created: 2026-11-02T20:02:42.151Z
created_by: seat:01M6EW8HN9QTQJ7A7GD1ZYMWYR
delegated_by: seat:01M6EW8HN9QTQJ7A7GD1ZYMWYR
id: 01M6EXPQTBR9CT8MEETNEDG1ZQ
number: 5002219
provenance:
  digest: sha256:a5a742ee05303453086ef7f1f63e444dfc6ec20d70f077da5034c13a2c85e83c
  frontmatter_digest: sha256:cc4634b5b668bfed9847e731a5ee4271ab2de4da32a4fd83414bfa9360ddde18
  log: .bboxy/log/2026-11-02.jsonl#11358
  sig: PqY8B4jV9t/oGjmEXYffvD9cGIPnI5Oqnu+aFUxTrzvp2JnT4bnYvpU0KYwrcz2hBGmSakopfp5yTkA/ZqKnAQ==
  signer: seat:01M6EW8HN9QTQJ7A7GD1ZYMWYR
report:
  commit: e93652c1
  op_id: 01M6F57FYMC3JAD6JYY2ES8NEB
  report: '[[reports/2026-11-02-recovery-landed]]'
  reported_by: seat:01M6EW8HN9QTQJ7A7GD1ZYMWYR
  summary: 'LANDED: recovery/disk and lean-runner source is on main in signed 5256f9e3e9c3156d35a6906057d3a188d06afbaa, after rebase onto 1a177bb6; signature policy and whitespace checks passed, git push exited 0 and remote main returned that exact SHA. Follow-up e93652c1 commits the append-only landed report, chat acceptance and linked artifact records. All 25 old-worktree removals and authorised process stops are complete; preserved refs/salvage were independently verified. Previous actual native recovery (4/50 seats), sparse growth rejection, chat/restart, smoke and 13 runner checks passed; strict Clippy, 308 Rust tests and nine corpus tests passed before the reported upstream formal initialization blocker. The operator now explicitly directed pushing everything to main including acceptance records, after the earlier test waiver and disclosed blocker; this is an operator-directed gate exception. No new test run or current-source formal success is claimed. Accept this source landing under that direction; then commit the authoritative accepted task Markdown to main.'
  verify_revision: 0
role: solo
status: accepted
swarm: 01M64ADP8CDYN9VX5CR31SZ629
title: Bound recovery fixture disk use for lean landing
type: task
updated: 2026-11-02T22:14:29.442Z
updated_by: seat:01M6EW8HN9QTQJ7A7GD1ZYMWYR
verify: make test-recovery-disk
verify_result:
  command: make test-recovery-disk
  evidence:
  - criterion: The recovery fixture keeps real durable ring/journal data and all crash/replay assertions.
    reference: Landed recovery.rs keeps the durable ring, tick journal, SQLite/checkpoint files and SIGKILL/replay/screen/debt/reaping assertions. Native 4/50-seat runs passed in /tmp/nyx-pinned-final-checks.log; final previous combined-source gate passed 308 tests.
  - criterion: The arbitrary 10 GiB guard is replaced by an explicit bounded fixture budget with real filesystem growth detection and measured usage.
    reference: Landed recovery.rs:317 defines 128 MiB + 1 MiB/seat; recursive retained-file accounting rejects growth. Four seats used 28,225,846 bytes under 138,412,032; fifty used 45,836,472 under 186,646,528. stdout already uses Stdio::null().
  - criterion: Normal and 50-seat private recovery runs pass; a sparse oversized fixture is rejected without allocating the limit on disk.
    reference: Actual make test-recovery-disk exit 0 at c09f2bd8 in /tmp/nyx-pinned-final-checks.log; four tests and fifty-seat recovery passed, including sparse 4097-byte versus 4096-byte rejection. Existing results retain their tested revision; no rerun is invented.
  - criterion: Signed clean changes, required formal evidence and canonical lean landing complete; original chat fix is included.
    reference: Source 5256f9e3 and record e93652c1 are signed and pushed to main, preserving ancestor chat fix ffd3fc76. Remaining formal/canonical requirements are accepted as an explicit operator-directed exception ('push everything to main including tasks acceptance and such!') following the disclosed upstream guard failure and prior test waiver. No formal pass is claimed. Final acceptance state will be pushed as the next signed task-record commit.
  exit_code: 0
---
ROLE: SOLO

Direct operator brief: clean disk by removing all worktrees aged at least 14 hours (authorised process kills), then make lean landing smarter about the recovery test's disk demand and avoid unnecessary per-state logs. Work alone, ask no further cleanup approval questions.

Implementation worktree: /Users/feral/mydev/bboxy450-term-chat-mute, existing fix/chat-mute-notices branch contains the verified but not yet landed chat fix. Rebase it on current main, preserve that fix and existing report. Work within the named paths. Trace actual writes, keep crash-recovery persistence, bound/report the fixture's real bytes rather than demanding 10 GiB for every four-seat run. Add the focused Make workflow and appropriate documentation, run native budget/negative-control and recovery checks, mandatory formal verification and make land. Do not alter peers' in-progress landing scripts. Cleanup audit and salvaged local state are outside Git.

Integration amendment: rebasing onto the newly pinned harness introduced HarnessFoldedCard and BrokenHarnessFoldedCard without the parent crate coverage entries. Register that existing contract/control pair under bboxy450-harness in verification/coverage.toml, preserving the model and its implementation witnesses. This is necessary for the clean-source mandatory formal gate. Record the initial formal failure and corrected result in an append-only integration report, superseding the earlier report; do not weaken or skip the gate.

Lean-runner integration amendment: make land failed on the changed swarm_workdir target because every test in that target is deliberately ignored, although the changed native test passed via its Make command. Add an explicit lean-only opt-in for a verified nonempty all-ignored integration target. Use nextest's actual list metadata to distinguish ignored tests from empty suites or filters; fail closed for explicit filters, malformed metadata, build errors and real test failures. Add focused runner controls, preserve normal/full-gate strictness, and record the corrected canonical landing result. Do not use a blanket no-tests=pass override.

Final evidence amendment: append a current-source report at reports/2026-11-02-recovery-final-status.md recording the canonical landing outcome after a moving-main rebase and pinned-harness initialization. Preserve successful cleanup and native test evidence; identify any upstream mandatory-gate failure without bypassing or weakening it. Do not change the upstream harness or JSON feature-isolation contract in this task.

## Report from seat:01M6EW8HN9QTQJ7A7GD1ZYMWYR

CLEANUP COMPLETE; DISK/LEAN IMPLEMENTATION VERIFIED BUT UNLANDED. All 25 reviewed worktrees aged at least 14 hours were removed (every removal exit 0), the authorised stale app/supervisor/server processes stopped, 25 saved HEAD refs and 12 salvaged directories independently verified. Recovery now measures a 128 MiB + 1 MiB/seat budget while preserving real durable ring/journal/checkpoints; stdout already used /dev/null. Four- and fifty-seat private recovery, sparse growth rejection, chat/restart, smoke and 13 runner controls passed. Final signed combined commit 8d78d3d2e56dd366e38cf50ac0e227197f4881ff is retained on fix/chat-mute-notices; both commits compiled, signature policy verified two commits, final canonical make land passed strict Clippy, 308 Rust tests and nine doc corpus tests, then failed mandatory formal initialization on upstream pinned harness cf56cf8 enabling serde_json/preserve_order (make exit 2; land script exit 77; /tmp/nyx-canonical-final-land.log). No current-source formal receipt or landing success is claimed. The operator then explicitly limited immediate landing to the small chat fix and waived remaining gates for that fix only; ffd3fc76 was independently verified on remote main and its task accepted. The disk/lean changes were excluded from that push and remain parked. This report closes out the stale queued self-delegation without restarting execution; criterion four is not met and this recovery task must not be accepted as landed.

```json
{
  "commit": "8d78d3d2e56dd366e38cf50ac0e227197f4881ff",
  "op_id": "01M6F29THAFZ8KNQ225QZXCKP0",
  "report": "[[reports/2026-11-02-recovery-final-status]]",
  "verify_result": {
    "command": "make test-recovery-disk",
    "evidence": [
      {
        "criterion": "The recovery fixture keeps real durable ring/journal data and all crash/replay assertions.",
        "reference": "Retained source on fix/chat-mute-notices in crates/bbt-mux/tests/recovery.rs; SIGKILL, replay/screen/debt and reaping assertions remain; actual private runs passed in /tmp/nyx-pinned-final-checks.log."
      },
      {
        "criterion": "The arbitrary 10 GiB guard is replaced by an explicit bounded fixture budget with real filesystem growth detection and measured usage.",
        "reference": "recovery.rs:317 fixture_disk_budget/fixture_bytes/check_fixture_size; 4-seat 28,225,846 bytes under 138,412,032 and 50-seat 45,836,472 under 186,646,528; stdout already Stdio::null()."
      },
      {
        "criterion": "Normal and 50-seat private recovery runs pass; a sparse oversized fixture is rejected without allocating the limit on disk.",
        "reference": "make test-recovery-disk completed exit 0 in /tmp/nyx-pinned-final-checks.log (tested c09f2bd8); four tests and fifty-seat recovery passed, sparse 4097-byte fixture rejected under 4096-byte budget. Final canonical gate at 8d78d3d2 passed 308 tests including normal recovery and the negative control."
      },
      {
        "criterion": "Signed clean changes, required formal evidence and canonical lean landing complete; original chat fix is included.",
        "reference": "NOT MET for the recovery branch: signatures, compilation, strict Clippy, 308 tests and nine corpus tests passed; current-source formal initialization failed on upstream harness feature isolation, canonical make exit 2. Chat fix separately landed as ffd3fc76 under explicit operator waiver; recovery changes remain unlanded in signed commit 8d78d3d2. Do not accept this task as landed."
      }
    ],
    "exit_code": 0
  }
}
```


## Report from seat:01M6EW8HN9QTQJ7A7GD1ZYMWYR

LANDED: recovery/disk and lean-runner source is on main in signed 5256f9e3e9c3156d35a6906057d3a188d06afbaa, after rebase onto 1a177bb6; signature policy and whitespace checks passed, git push exited 0 and remote main returned that exact SHA. Follow-up e93652c1 commits the append-only landed report, chat acceptance and linked artifact records. All 25 old-worktree removals and authorised process stops are complete; preserved refs/salvage were independently verified. Previous actual native recovery (4/50 seats), sparse growth rejection, chat/restart, smoke and 13 runner checks passed; strict Clippy, 308 Rust tests and nine corpus tests passed before the reported upstream formal initialization blocker. The operator now explicitly directed pushing everything to main including acceptance records, after the earlier test waiver and disclosed blocker; this is an operator-directed gate exception. No new test run or current-source formal success is claimed. Accept this source landing under that direction; then commit the authoritative accepted task Markdown to main.

```json
{
  "commit": "e93652c1",
  "op_id": "01M6F57FYMC3JAD6JYY2ES8NEB",
  "report": "[[reports/2026-11-02-recovery-landed]]",
  "verify_result": {
    "command": "make test-recovery-disk",
    "evidence": [
      {
        "criterion": "The recovery fixture keeps real durable ring/journal data and all crash/replay assertions.",
        "reference": "Landed recovery.rs keeps the durable ring, tick journal, SQLite/checkpoint files and SIGKILL/replay/screen/debt/reaping assertions. Native 4/50-seat runs passed in /tmp/nyx-pinned-final-checks.log; final previous combined-source gate passed 308 tests."
      },
      {
        "criterion": "The arbitrary 10 GiB guard is replaced by an explicit bounded fixture budget with real filesystem growth detection and measured usage.",
        "reference": "Landed recovery.rs:317 defines 128 MiB + 1 MiB/seat; recursive retained-file accounting rejects growth. Four seats used 28,225,846 bytes under 138,412,032; fifty used 45,836,472 under 186,646,528. stdout already uses Stdio::null()."
      },
      {
        "criterion": "Normal and 50-seat private recovery runs pass; a sparse oversized fixture is rejected without allocating the limit on disk.",
        "reference": "Actual make test-recovery-disk exit 0 at c09f2bd8 in /tmp/nyx-pinned-final-checks.log; four tests and fifty-seat recovery passed, including sparse 4097-byte versus 4096-byte rejection. Existing results retain their tested revision; no rerun is invented."
      },
      {
        "criterion": "Signed clean changes, required formal evidence and canonical lean landing complete; original chat fix is included.",
        "reference": "Source 5256f9e3 and record e93652c1 are signed and pushed to main, preserving ancestor chat fix ffd3fc76. Remaining formal/canonical requirements are accepted as an explicit operator-directed exception ('push everything to main including tasks acceptance and such!') following the disclosed upstream guard failure and prior test waiver. No formal pass is claimed. Final acceptance state will be pushed as the next signed task-record commit."
      }
    ],
    "exit_code": 0
  }
}
```
