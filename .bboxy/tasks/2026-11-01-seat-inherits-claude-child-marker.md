---
artifacts:
- '[[01M6DDBMCKSJAE5A06R114WA22]]'
assignee: seat:01M6D1P97MBKV7ZG21RYJQ3YSW
authorship:
  kind: seat
  process_generation: '1'
  public_key: J+Eete7RzpCHzcNSDpuEpx+ye6zu+UTGX+qDDQkPrw4=
  signer: seat:01M6CQ90BPKR0WZ70BDJD6R7MV
created: 2026-11-02T02:33:51.406Z
created_by: seat:01M6CQ90BPKR0WZ70BDJD6R7MV
delegated_by: seat:01M6CQ90BPKR0WZ70BDJD6R7MV
id: 01M6D1P7VRADV9GGJV42752W82
number: 2985
provenance:
  digest: sha256:58cf48bcaf37867220147a99dd5f39f6dc059ff20c93bff96b9a3944860db7f6
  frontmatter_digest: sha256:8f4a88d8608c8d31329fccdc5e1ae0033864f4bdbcf87885a3a130731570ebd5
  log: .bboxy/log/2026-11-02.jsonl#14978
  sig: 6b3LwffBmwoGlGg4YBMCmm9IbIgbvaTa3paAFpcWFwc+mQuY9zTA7wWbhv60PzY+SNEnbeGDuv12VVQxMYPCBA==
  signer: seat:01M6CQ90BPKR0WZ70BDJD6R7MV
report:
  commit: 2fd028ff891e6414998881d9995a3ded47985129
  op_id: 01M6DDXS2CG56JD17RZ3BRZXS6
  report: '[[reports/2026-11-01-seat-launch-environment-verified.md]]'
  reported_by: seat:01M6D1P97MBKV7ZG21RYJQ3YSW
  summary: 'Implemented shared caller-session scrubbing for all five harness profiles, companions, app startup and detached backends. Signed implementation ce63a06adb9d24b0e017ad4d2fc005b0428d6d42 passed make test (0), including strict workspace Clippy, native/TLC parity and unattended smoke; make test-boundaries also passed (0), including the polluted-parent witness and expected-failing broken control. Signed 2fd028ff891e6414998881d9995a3ded47985129 adds only the final report: source equivalence is checked, make test-kb-reports test-formal exited 0, and current-HEAD make check-formal-evidence exited 0 (209 models, 32 crates, 86 contracts). The conditional inheritance defect is proven; the historical two-laptop launcher remains unproven because Claude creates child markers for its own tool subprocesses. Existing owners were untouched and need an upgraded restart. Worktree clean; no merge or push.'
role: executor
status: accepted
swarm: 01M649102FBS60QWNPZS5VAGF9
title: Claude seats inherit CLAUDE_CODE_CHILD_SESSION, so transcript saving is off
type: task
updated: 2026-11-02T06:08:00.250Z
updated_by: seat:01M6CQ90BPKR0WZ70BDJD6R7MV
verify: make test
verify_result:
  command: make test
  evidence:
  - criterion: Root cause as file:line and distinction between inherited and process-owned markers
    reference: reports/2026-11-01-seat-launch-environment-verified.md:36 — base 86ce2653 ctl/app.rs:74 → cockpit.rs:1588 → supervisor.rs:2392 inherit caller environment; report:72 records static Claude child-factory evidence and the unproven historical trigger.
  - criterion: Every harness profile drops caller session markers while retaining user configuration and intended bboxy values
    reference: crates/bboxy450-harness-wake-up/src/guard/environment.rs:17; src/launch.rs:488; crates/bbt-supervisor/src/supervisor.rs:2424 and supervisor/companion.rs:56 — shared rule names applied after inheritance/overrides; target/seat-env-evidence/boundaries.log:147–153 records both launch cases passing. No persistence workaround is added.
  - criterion: Defence in depth at app and detached backend start
    reference: crates/bboxy450-term/src/ctl/app.rs:93; src/cockpit.rs:1597; src/cockpit/environment_tests.rs:13 — command-local cleanup and polluted-parent supervisor/logic/desktop witness; full CI log records the test passing.
  - criterion: Real polluted-parent process test with an unscrubbed control that must fail
    reference: crates/bboxy450-term/tests/cockpit/launch_environment.rs:106 and :237 — real supervisor with 20 PTY and 20 companion captures across five profiles, stale overrides and both persistence configurations; negative child must exit 101; make test-boundaries exit 0 in target/seat-env-evidence/boundaries.log.
  - criterion: Formal launch contract, assumptions, implementation witness and broken controls
    reference: verification/contracts.toml:5 and :15; target/seat-env-evidence/formal-2fd028ff.json SHA256 f1af82334635ec94f0b8df213b11d3dc0e60c8e8387f9247fa8780856c1a9abc — clean final HEAD, 209 models; launch models pass (32 states/90 edges each), separate broken controls produce safety failures (56/156 and 40/112), all graphs equal. make check-formal-evidence exit 0.
  - criterion: Required checks, signed commits and clean worktree
    reference: target/seat-env-evidence/ci-ce63a06a-green.log ends exact ce63a06a normal result 0; implementation-verification.json records log hashes and boundary/focused results; final-source-equivalence.json proves only the final report differs at 2fd028ff; final-verification.json records report/formal checks 0 and clean state. git verify-commit returned 0 for both commits. Normal CI retains 11 configured slow skips.
  exit_code: 0
---
# Seats inherit the parent Claude session's markers

ROLE: EXECUTOR. Work only in a new worktree `~/mydev/bboxy450-term-seat-env` on branch `seat-env`, created from `origin/main`. Work on your own: investigate, decide, fix and prove. Record each decision in your report. Message the Oracle only for a blocker that stops all work.

## Symptom (operator, 2026-11-01, two laptops on main)

Every Claude seat shows: `⚠ Transcript saving is off — inherited CLAUDE_CODE_CHILD_SESSION marker · restart with CLAUDE_CODE_FORCE_SESSION_PERSISTENCE=1 to keep future transcripts`. The seat's sessions are not saved, so `--resume` and history are lost.

## Evidence (Oracle)

- This seat (`01M6CQ90BPKR0WZ70BDJD6R7MV`, Claude Code 2.1.304, launched by bboxy) has in its environment `CLAUDE_CODE_CHILD_SESSION=1`, `CLAUDECODE=1`, `CLAUDE_PID=74330`, `CLAUDE_CODE_MESSAGING_SOCKET`, `CLAUDE_CODE_MESSAGING_TOKEN`, `CLAUDE_CODE_SESSION_ID`, `CLAUDE_CODE_ENTRYPOINT` and `CLAUDE_CODE_SESSION_ATTENDED`. Some of these are this process's own; find out which are inherited from the app, supervisor or launch path.
- The night launcher already strips caller harness markers through an environment allowlist: `crates/bboxy450-term/src/e2e_night.rs:727-750`, commits `688d88a6` and `1ceccce2`. The ordinary seat launch path evidently does not. Likely areas: `crates/bboxy450-harness-wake-up/src/launch.rs:790-910` (`env_remove`/`env_clear`), the supervisor spawn and the per-seat wiring kit.
- Probable trigger: the app or backend was started, installed or restarted from inside a Claude Code session (`make install`, `ctl app start`, or an Oracle seat), so the supervisor inherited the markers and passed them to every seat it launched. Confirm or refute this with evidence.

## Required outcome

1. Root cause as `file:line — mechanism`: which process first carries the marker and which launch step passes it to seats.
2. The fix: seat launch, for every harness profile (claude, codex, zsh, pi, omp), drops caller-harness markers and session-scoped values: `CLAUDECODE`, `CLAUDE_CODE_*` session/child/messaging/entrypoint markers, `CLAUDE_PID`, `CODEX_THREAD_ID`, and similar. Reuse the night allowlist or denylist mechanism instead of a second list, unless you record why it doesn't fit. Keep the variables seats need: PATH, HOME, user config, and the bboxy values set on purpose. Do not set `CLAUDE_CODE_FORCE_SESSION_PERSISTENCE` as a workaround.
3. Defence in depth: if the app's own process environment has these markers, strip them at the earliest owner (app or supervisor start) too, so detached backends don't keep them.
4. A real process test: start a supervisor or seat from a parent environment that has the markers, and assert the spawned seat's environment lacks them. Add a broken control that removes the scrub and must fail.
5. Formal (d365): if launch environment is modelled in `verification/contracts.toml`, update it. Otherwise record the contract (no caller-harness marker crosses seat launch) with its implementation witness.

## Allowed paths

`crates/bboxy450-harness-wake-up/**`, `crates/bbt-supervisor/**` (launch and environment only; #2187 is editing input dedup in `supervisor.rs`, so don't touch that region), `crates/bboxy450-term/**` (launch, cockpit and env only), `verification/**`, `.bboxy/reports/**`, and this exact task record copied byte-for-byte to `.bboxy/tasks/2026-11-01-seat-inherits-claude-child-marker.md` (the report gate needs it in-tree).

## Verification

`make test` on the exact committed source, exit 0, plus `make test-boundaries` (process boundary). Strict clippy on touched crates.

## Done

Signed commits on `seat-env`, a report in `.bboxy/reports/` with a `Report:` trailer, and `task_report`. Do not merge or push.


## Report from seat:01M6D1P97MBKV7ZG21RYJQ3YSW

Implemented shared caller-session scrubbing for all five harness profiles, companions, app startup and detached backends. Signed implementation ce63a06adb9d24b0e017ad4d2fc005b0428d6d42 passed make test (0), including strict workspace Clippy, native/TLC parity and unattended smoke; make test-boundaries also passed (0), including the polluted-parent witness and expected-failing broken control. Signed 2fd028ff891e6414998881d9995a3ded47985129 adds only the final report: source equivalence is checked, make test-kb-reports test-formal exited 0, and current-HEAD make check-formal-evidence exited 0 (209 models, 32 crates, 86 contracts). The conditional inheritance defect is proven; the historical two-laptop launcher remains unproven because Claude creates child markers for its own tool subprocesses. Existing owners were untouched and need an upgraded restart. Worktree clean; no merge or push.

```json
{
  "commit": "2fd028ff891e6414998881d9995a3ded47985129",
  "op_id": "01M6DDXS2CG56JD17RZ3BRZXS6",
  "report": "[[reports/2026-11-01-seat-launch-environment-verified.md]]",
  "verify_result": {
    "command": "make test",
    "evidence": [
      {
        "criterion": "Root cause as file:line and distinction between inherited and process-owned markers",
        "reference": "reports/2026-11-01-seat-launch-environment-verified.md:36 — base 86ce2653 ctl/app.rs:74 → cockpit.rs:1588 → supervisor.rs:2392 inherit caller environment; report:72 records static Claude child-factory evidence and the unproven historical trigger."
      },
      {
        "criterion": "Every harness profile drops caller session markers while retaining user configuration and intended bboxy values",
        "reference": "crates/bboxy450-harness-wake-up/src/guard/environment.rs:17; src/launch.rs:488; crates/bbt-supervisor/src/supervisor.rs:2424 and supervisor/companion.rs:56 — shared rule names applied after inheritance/overrides; target/seat-env-evidence/boundaries.log:147–153 records both launch cases passing. No persistence workaround is added."
      },
      {
        "criterion": "Defence in depth at app and detached backend start",
        "reference": "crates/bboxy450-term/src/ctl/app.rs:93; src/cockpit.rs:1597; src/cockpit/environment_tests.rs:13 — command-local cleanup and polluted-parent supervisor/logic/desktop witness; full CI log records the test passing."
      },
      {
        "criterion": "Real polluted-parent process test with an unscrubbed control that must fail",
        "reference": "crates/bboxy450-term/tests/cockpit/launch_environment.rs:106 and :237 — real supervisor with 20 PTY and 20 companion captures across five profiles, stale overrides and both persistence configurations; negative child must exit 101; make test-boundaries exit 0 in target/seat-env-evidence/boundaries.log."
      },
      {
        "criterion": "Formal launch contract, assumptions, implementation witness and broken controls",
        "reference": "verification/contracts.toml:5 and :15; target/seat-env-evidence/formal-2fd028ff.json SHA256 f1af82334635ec94f0b8df213b11d3dc0e60c8e8387f9247fa8780856c1a9abc — clean final HEAD, 209 models; launch models pass (32 states/90 edges each), separate broken controls produce safety failures (56/156 and 40/112), all graphs equal. make check-formal-evidence exit 0."
      },
      {
        "criterion": "Required checks, signed commits and clean worktree",
        "reference": "target/seat-env-evidence/ci-ce63a06a-green.log ends exact ce63a06a normal result 0; implementation-verification.json records log hashes and boundary/focused results; final-source-equivalence.json proves only the final report differs at 2fd028ff; final-verification.json records report/formal checks 0 and clean state. git verify-commit returned 0 for both commits. Normal CI retains 11 configured slow skips."
      }
    ],
    "exit_code": 0
  }
}
```
