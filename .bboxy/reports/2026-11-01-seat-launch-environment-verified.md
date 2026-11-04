---
author: seat:01M6D1P97MBKV7ZG21RYJQ3YSW
authorship:
  kind: seat
  process_generation: '1'
  public_key: SyV9MMpCU/mLhMN4qtweRkA6IceuecX18S9xBXLzLjo=
  signer: seat:01M6D1P97MBKV7ZG21RYJQ3YSW
commits:
- ce63a06adb9d24b0e017ad4d2fc005b0428d6d42
created: 2026-11-02T05:55:44.006Z
created_by: seat:01M6D1P97MBKV7ZG21RYJQ3YSW
id: 01M6DD7V2RMCE8Q2BJBPBRG8VD
number: 3178
provenance:
  digest: sha256:473f2231f750beed7cdd0a0aa5109e9621f1b0b95a05c5b2cc2f403a916b6a1c
  frontmatter_digest: sha256:aa48eaa41140c6ff9f82bd6bf4f1a567c3f794faa7637b30be43c8717adbfb75
  log: .bboxy/log/2026-11-02.jsonl#14081
  sig: DMNlBp56l/DcbpkQsbxLAi68n0+ItrNEAnI7hfwk+EgZFAhEAbQ2rcMR1KHp8rXFkAhW7E89kFxIIClVKZuABQ==
  signer: seat:01M6D1P97MBKV7ZG21RYJQ3YSW
supersedes: '[[reports/2026-11-01-seat-launch-environment]]'
task: '[[tasks/2026-11-01-seat-inherits-claude-child-marker]]'
title: Seat launch environment verified implementation
type: report
updated: 2026-11-02T05:55:44.006Z
updated_by: seat:01M6D1P97MBKV7ZG21RYJQ3YSW
verify:
  command: make test
  exit_code: 0
---
# Seat launch environment — verified implementation

Task #2985; Executor seat 01M6D1P97MBKV7ZG21RYJQ3YSW. Worktree /Users/feral/mydev/bboxy450-term-seat-env, branch seat-env, based on origin/main at 86ce2653. This supersedes the precommit report. Full canonical CI checked signed implementation commit ce63a06adb9d24b0e017ad4d2fc005b0428d6d42. This report adds evidence only; its containing signed commit is bound by its Report trailer.

## Outcome and acceptance evidence

1. Root cause: at base 86ce2653, crates/bboxy450-term/src/ctl/app.rs:74 constructs an inheriting app command (spawn at line 94), and src/cockpit.rs:1588 constructs inheriting detached supervisor/logic/desktop commands. crates/bbt-supervisor/src/supervisor.rs:2392 captures that environment in CommandBuilder; ordinary seats removed NO_COLOR and a legacy admission value but did not clear caller harness identities. The old eight-key native-runtime list at crates/bboxy450-harness-wake-up/src/launch.rs:16 omitted CHILD_SESSION, CLAUDECODE, CLAUDE_PID and messaging/attended/entrypoint context and was not the ordinary all-profile seat boundary. A polluted tool-shell parent can therefore transmit its marker through app/backend startup to ordinary seats. The new real-process witnesses reproduce that conditional path and verify its closure.
2. Every profile: crates/bboxy450-harness-wake-up/src/guard/environment.rs:17 owns one exact/prefix session rule set; launch.rs:488 attaches its names to every profile plan. crates/bbt-supervisor/src/supervisor/environment.rs:17 interprets those generic rules against captured inherited and explicit environments. supervisor.rs:2424 and supervisor/companion.rs:56 apply them after overrides and consume the control value before exec. claude, codex, zsh, pi and omp plans each receive polluted inherited and stale-override cases. HOME, PATH, user/auth/provider/effort configuration and intended bboxy wiring remain; no product path sets CLAUDE_CODE_FORCE_SESSION_PERSISTENCE.
3. Early defence: crates/bboxy450-term/src/ctl/app.rs:93 scrubs app startup, and src/cockpit.rs:1597 scrubs each detached backend command. No process-global mutation is used. src/cockpit/environment_tests.rs:13 drives the actual backend launcher from a private polluted parent for supervisor, logic and desktop, verifies child environments and leaves the parent's marker intact.
4. Process witness: tests/cockpit/launch_environment.rs:106 starts the real supervisor in two private polluted parents (without a persistence preference and with an existing one). Five profiles, two override modes and both preferences yield 20 PTY plus 20 companion captures. Marker absence, retained config/wiring and consumed rule data are asserted. The control at line 237 omits cleanup and requires the child assertion to fail with exit 101. Both outer tests are intentionally ignored in default workspace execution and are explicitly executed by make test-boundaries: both passed; all nine cockpit boundary cases passed. All tests are unattended and use private homes/processes.
5. Formal: verification/contracts.toml:5 and :15 register SeatLaunchEnvironment and BackendLaunchEnvironment, each with a separately broken scrub control, implementation/source mapping and checked assumptions. verification/corpus.toml and coverage.toml include all four cases and the three changed crates. Native Rust and pinned Java TLC agree on full reachable state/transition sets and verdicts: SeatLaunchEnvironment pass (32 states, 90 edges), BrokenSeatLaunchEnvironment expected safety failure (56, 156), BackendLaunchEnvironment pass (32, 90), BrokenBackendLaunchEnvironment expected safety failure (40, 112). These are bounded safety checks, not a proof of arbitrary Rust or native transcript persistence. Full-corpus completion is recorded with the CI evidence below.

## Decisions

- The night launch allowlist intentionally discards authentication and redirects configuration homes; it would break ordinary seats. The existing native-runtime denylist was expanded into one shared session policy instead, also reused by native same-PID cleanup and early app/backend starts.
- Spec #273 and the executable host vocabulary gate require a harness-neutral process host. Provider names stay in the wake-up kit; the host accepts whitespace-separated exact names and trailing-prefix rules through BBOXY_LAUNCH_ENV_UNSET. Only names cross that protocol; inherited secret values are not copied into LaunchSpec or argv.
- Scrubbing runs after both captured inheritance and overrides. Bare plugin login terminals retain their explicit exact-environment contract and are outside ordinary harness seat policy.
- Authentication, homes, provider settings, CLAUDE_EFFORT and an existing persistence preference are configuration, not caller identities. The product preserves them; FORCE_SESSION_PERSISTENCE is never added as a workaround.
- A fresh owner and a retained dirty owner are distinct formal states. Seat cleanup works independently of fresh-backend cleanup. Caller A/B/A changes, owner start/attach/restart and seat exec are atomic snapshots. Finite bounds are one caller bit, one owner with absent/fresh/retained origin, and three seat births. Both properties are safety; no fairness/liveness promise is assumed. String matching, configuration, OS processes and native harness behavior remain implementation witnesses rather than arbitrary Rust refinement claims.
- The initial stand-in exited before its companion captured the environment; it now stays alive until owned supervisor shutdown. The capture deadline was not increased. The host vocabulary and same-tree report failures were corrected without relaxing their gates.
- No code changed for the resource-precondition retry. No disk threshold, test quarantine or operator exception was changed.

## Commands and actual results

| Command | Exit | Evidence |
| --- | --- | --- |
| make test on signed ce63a06adb9d24b0e017ad4d2fc005b0428d6d42 | 0 | target/seat-env-evidence/ci-ce63a06a-green.log; exact source/result lines and successful Make completion footer |
| cargo clippy --workspace --all-targets --locked --target-dir target -- -D warnings, inside make test | 0 | same full CI log; covers all three touched crates |
| make test-boundaries, CI profile environment | 0 | target/seat-env-evidence/boundaries.log; both launch controls actually executed |
| make test-harness-startup test-formal-contracts, CI profile environment | 0 | target/seat-env-evidence/focused.log |
| make test-harness-vocabulary | 0 | target/seat-env-evidence/vocabulary.log |
| make build-recovery-test test-recovery-built, CI profile environment | 0 | target/seat-env-evidence/recovery-retry.log |
| git diff --check and clean-tree/source assertions | 0 | no whitespace errors or pending source changes at full gate completion |

The CI profile environment is CARGO_PROFILE_DEV_DEBUG=0, CARGO_PROFILE_TEST_DEBUG=0, CARGO_PROFILE_DEV_OPT_LEVEL=1, CARGO_PROFILE_TEST_OPT_LEVEL=1, CARGO_PROFILE_DEV_DEBUG_ASSERTIONS=true and CARGO_PROFILE_TEST_DEBUG_ASSERTIONS=true.

The initial formal-contract attempt exited 2 because the fresh worktree lacked its pinned submodule; make init-submodules exited 0, and subsequent contract checks passed. The superseded unpublished cdc8e58a full run exited 2 for the same-tree report, host vocabulary and capture-lifetime defects above; ci-initial-cdc8e58a.log retains it. A full run on ce63a06a exited 2 because checkpoint_bounded_recovery observed only 7,187,099,648 available bytes, below its unchanged 10 GiB minimum; ci-ce63a06a-low-disk.log retains it. Later the same statvfs filesystem reported 36,007,546,880 available bytes. No peer files were removed. The retained canonical recovery workflow then passed, and the full retry passed on unchanged ce63a06a.

Full CI includes workspace strict Clippy, Rust tests, capability controls, native harness checks, native/Java TLC parity, licence/invariant checks and private unattended headless smoke. The full log ends with `ci-local: result ce63a06adb9d24b0e017ad4d2fc005b0428d6d42 normal 0`; the wrapper has its successful Full gate log footer. SHA256 of ci-ce63a06a-green.log is 3895dfc73e35ad52a183933da52b6df149ed00c706916b787246e4b291d1c269; boundaries.log is b4ab297d36a1dc89a72f061e08c75dae6b2e5c379d85a44da4e23666eb96417a. The clean source receipt, copied to target/seat-env-evidence/formal-ce63a06a.json, has SHA256 bad86384e240a217414cbe4d09597784c712adadbdb38c6fff45d2694cb238a4: 209 models, 32 crates and 86 contracts. Its content-addressed copy is retained under the repository common Git directory at bboxy-formal-evidence/bad86384e240a217414cbe4d09597784c712adadbdb38c6fff45d2694cb238a4/evidence.json. target/seat-env-evidence/implementation-verification.json records all log hashes. The final invariant result was 27 checks, zero failures and 11 configured slow checks skipped; this is the normal gate, not the extended scale/performance run.

## Observation limits and handoff

The historical launcher of the two reported laptops remains unproven. Makefile:597–607 installs/copies/verifies the bundle and does not start the app. Installed Claude 2.1.304 SHA256 b9ac5374839f14abf3f1f45610637c332b5daee5ce0d1634726cef87aab78e6a contains a child-environment factory at byte offset 222523598 that creates CHILD_SESSION, CLAUDECODE, SESSION_ID, SESSION_ATTENDED and CLAUDE_PID for tool subprocesses. At offset 254882527 its MCP constructor removes CHILD_SESSION and supplies its own session context. These static bytes show why a tool-shell environment dump cannot establish the TUI's startup environment; they are not a native execution trace. REA deep string analysis was unavailable (no native analysis provider); ordinary local byte inspection supplied the bounded artifact evidence. Current installed backend initial-exec snapshots contained none of the selected session keys; they do not establish historical or mutable runtime state.

Existing installed owners were not replaced, stopped or modified. The upgraded owner must restart to use the generic removal protocol; a legacy host ignores it. Arbitrary raw host callers without rules and user shell code are outside the contract. A live authenticated Claude transcript-persistence session was not run, and old transcripts cannot be recovered by this patch. UI geometry did not change; visual parity was not run. Raw logs and private fixture data remain outside Git under target/seat-env-evidence. The exact task record was copied byte-for-byte. No merge, push, publication or credential change was performed.

The final report-only commit will be checked with the existing report corpus target and a current-HEAD formal receipt; task_report carries those actual post-report results and the signed final SHA. The full Rust CI evidence above is explicitly bound to ce63a06a, and source equivalence must show that the later commit changes only this report.
