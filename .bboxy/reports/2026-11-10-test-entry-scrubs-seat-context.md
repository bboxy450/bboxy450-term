---
type: report
title: Test entry point scrubs a seat's live context
task: '[[bugs/harness-python-execute-tests-leak-seat-context]]'
author: seat:01M71GNVXV7ZHXMGM50M5SB2RA
created: 2026-11-10T19:23:36.009Z
commits: []
verify:
  command: sh scripts/quarantine-test.sh --locked -p bboxy450-harness --lib -E 'test(/python_execute::tests::/)'
  exit_code: 0
id: 01M73EMVM4RQT7ZQ6GC48YTNP1
updated: 2026-11-10T19:23:36.009Z
number: 1794292861627475
created_by: seat:01M71GNVXV7ZHXMGM50M5SB2RA
updated_by: seat:01M71GNVXV7ZHXMGM50M5SB2RA
authorship:
  signer: seat:01M71GNVXV7ZHXMGM50M5SB2RA
  kind: seat
  public_key: V5Zkz4W1rnaDG/FsjK3//xQX+WByyxD7/fuIYoV6NbU=
  process_generation: '1'
provenance:
  digest: sha256:9faae4b261c5cb35cf70b9ded9c486b7e8c39d25c7a70f8153845369d4756408
  frontmatter_digest: sha256:294d52ae501fb1332cd82cd9f7a23bd6d20050178f069a1ee7db06ad0476cc27
  signer: seat:01M71GNVXV7ZHXMGM50M5SB2RA
  sig: QCbYphoEuElhnuHte2nQBiQ/V6cp0lHSh9AR2yo6x3KLXt4RtXPnTryC0vssJY2M5eFNKNgmTRYdRpAwiDR9Aw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151580
  event:
    id: 01M73EMWTEJZW588844BDMM4PH
    sequence: 151580
    hash: 83d7af32e69fda69dec3d085ab01cce5a1cb8aeb04c9b55b296f133d89ba2ef9
---
# Test entry point scrubs a seat's live context

Fixes [[bugs/harness-python-execute-tests-leak-seat-context]].

## Change

`scripts/quarantine-test.sh`, the one entry point every lean land, CI run and Make test
target uses for tests, now unsets the ten variables the launcher sets for a seat before it
starts the test runner: `BBOXY_CAPABILITY`, `BBOXY_CAPABILITY_FILE`, `BBOXY_GENERATION`,
`BBOXY_HOOK_SOCKET_ONLY`, `BBOXY_PROCESS_GENERATION`, `BBOXY_ROLE`, `BBOXY_RUN_DIR`,
`BBOXY_SEAT_ID`, `BBOXY_SOCK`, `BBOXY_USER_ZDOTDIR`. The list is the set the launcher
writes (`crates/bboxy450-harness-wake-up/src/launch.rs`, `crates/bbt-supervisor`).
Variables the gate itself reads (`BBOXY_CARGO_TARGET_DIR`, `BBOXY_LANDING_STATE_DIR`,
`BBOXY_TEST_RUNNER`, `BBOXY_PRESUBMIT_*`) have other names and are untouched.

## Removed

The workaround: landing with seven `env -u` prefixes on `make land`, which every Executor
brief since 2026-11-10 carried. It is no longer needed and should not be copied into new
briefs.

## Evidence

Run from inside a bboxy seat (the seat's `BBOXY_SOCK` set), in a fresh worktree at
`origin/main` 7e0625c75:

| Command | Result |
|---|---|
| main's `scripts/quarantine-test.sh … -E 'test(/python_execute::tests::short_execution_returns_result/)'` | 1 run, 1 failed (`native_context_unavailable`) |
| patched `scripts/quarantine-test.sh … -E 'test(/python_execute::tests::/)'` | 23 run, 23 passed, 5.7 s |
| `sh -n scripts/quarantine-test.sh` | exit 0 |

## Found on the way, not fixed here

The first landing attempt also changed `.bboxy`, which puts `bboxy450-doc`'s corpus test in
the plan. That test is red on `main` for two reasons unrelated to this change:

- A hand-written report lacks the host-assigned `id` and `number` (this report's first
  version; filed through the host instead).
- Two bare links are ambiguous because a task and its report share a slug:
  `[[2026-11-09-canvas-c0]]` in `tasks/2026-11-09-canvas-c1a.md` (closed, so frozen) and
  `[[2026-11-05-dream-mcp-verified-snapshot]]` in
  `notes/2026-11-06-dream-q2-scope-snapshot.md`. Any change to `.bboxy` that runs the
  corpus test fails on these until they are qualified (`[[tasks/…]]`). They belong to the
  canvas and Dream sessions; not edited here, because editing a signed record by hand
  breaks its provenance and the closed task cannot be amended.

## Not checked

No test was found that needs a seat variable inherited from the caller; e2e fixtures that
use `BBOXY_SEAT_ID` get it from the private app they launch, not from the gate. A test
that did rely on an inherited value would now fail, which is the intended behavior.
