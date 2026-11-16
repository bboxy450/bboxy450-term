---
author: seat:01M6D1P97MBKV7ZG21RYJQ3YSW
authorship:
  kind: seat
  process_generation: '1'
  public_key: SyV9MMpCU/mLhMN4qtweRkA6IceuecX18S9xBXLzLjo=
  signer: seat:01M6D1P97MBKV7ZG21RYJQ3YSW
commits: []
created: 2026-11-02T04:28:56.357Z
created_by: seat:01M6D1P97MBKV7ZG21RYJQ3YSW
id: 01M6D88Y83GK5Z01CFPY3M4SM4
number: 3162
provenance:
  digest: sha256:6a6eac473efc3614838ac2e088185165fc753fc753aa13407bbcdadf4d089961
  frontmatter_digest: sha256:a394c2584342ada61c14e6e1aba3d45840a3e72e30ece51655956a7ff9a07c86
  log: .bboxy/log/2026-11-02.jsonl#13907
  sig: clwxGygAelDuPdMZpUZEwJJaX/3uUNNxlWCcQjRX/5wUAu6mDIgEhePDO7fHRLpG8OrBjt4d4Mcx08aMt9+WAA==
  signer: seat:01M6D1P97MBKV7ZG21RYJQ3YSW
task: '[[tasks/2026-11-01-seat-inherits-claude-child-marker]]'
title: Seat launch environment implementation and precommit evidence
type: report
updated: 2026-11-02T04:28:56.357Z
updated_by: seat:01M6D1P97MBKV7ZG21RYJQ3YSW
verify:
  command: make test-boundaries
  exit_code: 0
---
# Seat launch environment — implementation and precommit evidence

Task #2985; Executor seat 01M6D1P97MBKV7ZG21RYJQ3YSW. Worktree `/Users/feral/mydev/bboxy450-term-seat-env`, branch `seat-env`, created from origin/main at 86ce2653. This implementation report records completed precommit checks. Its containing signed commit is identified by the Report trailer; `commits: []` avoids a self-referential SHA. Exact committed-source full CI remains pending.

## Root cause and limits of the supplied observation

- At base 86ce2653, `crates/bboxy450-term/src/ctl/app.rs:74` constructs an app command with the caller's environment and line 94 spawns it. `crates/bboxy450-term/src/cockpit.rs:1588` constructs detached backend commands and overrides PATH/run paths without clearing caller session metadata. `crates/bbt-supervisor/src/supervisor.rs:2392` constructs a PTY command from the supervisor's inherited environment; only bare launches clear it. Ordinary launches removed NO_COLOR and the obsolete admission value, not caller harness identities. Those steps transmit an incoming child/session marker to a new seat.
- `crates/bboxy450-harness-wake-up/src/launch.rs:16` at that base had an eight-key native-runtime denylist. Its Codex paths used it, but it omitted CLAUDECODE, CLAUDE_PID, CLAUDE_CODE_CHILD_SESSION, the messaging token and attended/entrypoint metadata. The ordinary Claude/plain-shell/generic profile path had no equivalent scrub.
- This proves the conditional inheritance defect and a launch from a contaminated parent reproduces the boundary. It does not identify the historical launcher of the operator's two affected laptops. Makefile:597–607 copies and verifies the installed bundle; it does not launch the app. Starting an app executable or `ctl app start` from a Claude tool subprocess is the demonstrated vulnerable path.
- The installed Claude 2.1.304 artifact `/opt/homebrew/Caskroom/claude-code/2.1.304/claude`, SHA256 b9ac5374839f14abf3f1f45610637c332b5daee5ce0d1634726cef87aab78e6a, contains an embedded JavaScript child-environment factory at byte offset 222523598 that creates CLAUDECODE, CHILD_SESSION, SESSION_ID, SESSION_ATTENDED and CLAUDE_PID from the running session. At offset 254882527 the MCP constructor removes CHILD_SESSION and supplies its own session ID/CLAUDECODE. Therefore a tool-shell env dump is not evidence of the parent TUI's startup environment. These are static byte observations, not a native execution trace.
- The installed supervisor 45598, desktop 45693 and logic 46260 had no selected caller session keys in their `ps eww` initial exec snapshots. That observation is bounded to those snapshots, not their mutable runtime environments or historical launches. Initial process selection also matched hook/MCP helpers; those were excluded from the retained owner evidence. The historical warning trigger remains unproven from the supplied dump.

## Implementation decisions

1. `crates/bboxy450-harness-wake-up/src/guard/environment.rs:17` owns one exact/prefix rule set, expanding and reusing the previous native-runtime denylist. It covers Claude child/session/messaging/entrypoint, bridge/remote/container/worker context, Codex thread/session/turn and pi/omp session metadata. Native runtime isolation uses the same rules through launch.rs:17, retaining same-PID exec.
2. `crates/bboxy450-harness-wake-up/src/launch.rs:488` supplies the rule names in the existing launch override map as BBOXY_LAUNCH_ENV_UNSET for every profile. The existing guard adapter re-export lets the app reuse the same policy without a new dependency or edits to bbt-mux/Cargo.lock. Protocol documentation and both crate READMEs describe the seam.
3. The process host must contain no harness-specific vocabulary (spec #273). `crates/bbt-supervisor/src/supervisor/environment.rs:17` interprets caller-supplied exact/prefix names, not provider identities. Supervisor.rs:2424 applies them after overrides to the captured PTY environment. Companion.rs:56 does likewise for owned services. The control value is consumed before exec. Only names are recorded; inherited values and credentials are not copied into LaunchSpec or argv.
4. `crates/bboxy450-term/src/ctl/app.rs:93` and cockpit.rs:1597 scrub before app/backend starts, including supervisor, logic and desktop. No global environment mutation is used. Bare login terminals retain their exact-environment contract; the legacy admission-value guard remains independent.
5. The night allowlist intentionally discards authentication and redirects homes. Applying it to real seats would discard user configuration, so normal launches use the shared targeted rule set. PATH, HOME/config directories, authentication, provider and effort settings remain. Existing CLAUDE_CODE_FORCE_SESSION_PERSISTENCE is configuration and is preserved; the product never sets it as a workaround.
6. `crates/bboxy450-term/tests/cockpit/launch_environment.rs:106` runs the real supervisor from two private polluted parents, without a persistence override and with a pre-existing preference. Five profile plans, each with and without stale overrides, produce 20 PTY and 20 companion environment captures. Assertions cover marker exclusion, preserved config/wiring and consumption of rule data. The negative control at line 237 requires an unscrubbed process assertion to exit 101. The backend-start witness is in src/cockpit/environment_tests.rs:13.
7. The initial stand-in exited before its companion could publish a capture. The corrected stand-in stays alive until its owned supervisor shuts down; the ten-second capture bound was not raised. The first implementation also put provider names in the host and referenced a report before committing it. The vocabulary failure was corrected by moving policy to the wake-up kit; the unpublished source commit will be amended with this report in the same tree. No gate was relaxed.

## Formal contract

`crates/bboxy450-term/tests/formal_contracts/launch_environment.rs:11` declares caller_marker initially 1, owner_marker and seat_marker initially 0, absent/fresh/retained owner_source initially absent, and birth initially 0 bounded by 3. Caller A/B/A changes, backend start/attach/restart and seat exec are atomic actions. FreshBackendHasNoCallerMarker and NoCallerMarkerCrossesSeatExec are safety properties; no fairness or liveness guarantee is assumed. Separate broken backend and seat controls omit one scrub each. verification/contracts.toml, corpus.toml and all three owning crate coverage rows map the models and witnesses. Native/TLC parity on the final committed source is still required. This does not prove arbitrary Rust, native CLI transcript persistence, user shell code or unupgraded host behavior.

## Commands actually completed

| Command | Exit | Evidence |
| --- | --- | --- |
| make init-submodules | 0 | target/seat-env-evidence/submodule.log |
| make test-harness-vocabulary | 0 | target/seat-env-evidence/vocabulary.log; host remains provider-neutral |
| make test-boundaries, with the CI profile environment | 0 | target/seat-env-evidence/boundaries.log; nine cockpit cases passed, including both new controls |
| make test-harness-startup test-formal-contracts, with the CI profile environment | 0 | target/seat-env-evidence/focused.log; native contract tests and strict Clippy completed |
| git diff --check | 0 | no whitespace errors |
| First make test on cdc8e58a541f813939a7b7d3c1851a7f05d9efb7 | 2 | target/seat-env-evidence/ci-initial-cdc8e58a.log; report same-tree, host vocabulary and capture-lifetime failures above |

The first formal-contract attempt exited 2 because this fresh worktree lacked the pinned submodule. Initializing it resolved the prerequisite; the later formal-contract checks exited 0. The failed full run did not reach its later formal/TLC and smoke stages. No failed result is claimed green.

## Remaining verification and operational limits

Run make test on the signed candidate containing this report, then publish the actual result through a superseding committed report and task_report. Strict workspace Clippy, full formal/TLC parity and unattended smoke are included in that gate. No full-CI pass is claimed here.

Existing installed owners were not replaced, stopped or modified. An upgraded owner must restart to use the new generic removal-rule protocol; a legacy host ignores it. Ordinary per-harness plans and private stand-ins are verified, not arbitrary raw host callers that supply no rules. Native Claude was not launched with credentials or model calls, and historical transcripts cannot be recovered by this change. No UI geometry changed, so parity was not run. Raw logs and static artifact observations remain under target/seat-env-evidence, outside Git. No merge or push is authorized or performed.