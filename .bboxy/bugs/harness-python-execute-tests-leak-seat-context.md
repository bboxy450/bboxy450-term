---
type: bug
title: 'Harness python_execute tests fail when run from inside a bboxy seat: they pick up the live seat context'
id: 01M72B6Y646MSP098W4W6BNZBN
status: open
delegated_by: seat:01M71GNVXV7ZHXMGM50M5SB2RA
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-10T09:04:19.010Z
created: 2026-11-10T09:04:19.010Z
number: 1794292861627443
created_by: seat:01M71GNVXV7ZHXMGM50M5SB2RA
updated_by: seat:01M71GNVXV7ZHXMGM50M5SB2RA
authorship:
  signer: seat:01M71GNVXV7ZHXMGM50M5SB2RA
  kind: seat
  public_key: V5Zkz4W1rnaDG/FsjK3//xQX+WByyxD7/fuIYoV6NbU=
  process_generation: '1'
provenance:
  digest: sha256:c3edfde8bfff68638bd778223b6e161e7df47e4905ef83852d08bdafbb1b6cc3
  frontmatter_digest: sha256:9412a0b7f73aa9c02581e071328eeee978c396d381d2fc8996a461294bd5ffd7
  signer: seat:01M71GNVXV7ZHXMGM50M5SB2RA
  sig: nG8148CMJk4Mhr2g2kI8aZhS183us0ampZx/bpJ5L4vLsBfCWeWLEhs7tID0Fhvr2oeBWlDQtkoXezu3BDAyBQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151457
  event:
    id: 01M72B6Z1D2BNKE8CEV4BAYHND
    sequence: 151457
    hash: fd294b9bf733ac6812179605d22bc6604b7d4e825c2fab68fb70fef3e85fa321
---
# Harness python_execute tests leak the seat's context

## Expected

Every test runs unattended and hermetically ([[test-policy]]). `make land` run by an
Executor inside its own seat gives the same result as the same command in a plain shell.

## Actual

Inside a bboxy seat, the lean gate's harness library tests fail: 36 failures in
`python_execute::tests::*`, each in under 0.1 s. `make land` exits 77 before formal
verification or push.

The error behind the `Option::unwrap()` panics:

```
{"error":{"code":"native_context_unavailable",
 "message":"admitted bboxy execution context is unavailable; update/reconnect the native MCP server"}}
```

## Reproduction (2026-11-10, harness pin 20e52d0, term main at 73745e2ec)

From a seat's shell, in a term worktree:

| Command | Result |
|---|---|
| `cargo nextest run --locked -p bboxy450-harness --lib -E 'test(/python_execute::tests::/)'` | fails, `native_context_unavailable` |
| the same, prefixed with `env -u BBOXY_CAPABILITY_FILE -u BBOXY_GENERATION -u BBOXY_PROCESS_GENERATION -u BBOXY_ROLE -u BBOXY_RUN_DIR -u BBOXY_SEAT_ID -u BBOXY_SOCK` | 23 passed, 7.0 s |

The tests start a fixture MCP server, but the code under test also sees the seat's real
`BBOXY_*` variables and tries the live native context, which refuses a test process.

It only shows when the harness crate's own tests are in the lean plan, which happens when
a change touches the root `Cargo.toml` or `Cargo.lock` (first seen on
[[tasks/2026-11-09-bspec-checker-crate]]).

## Workaround in use

Land with the seven variables unset. Same tests, no check removed.

## Fix, either one

- The gate scrubs seat context for every test it runs: `scripts/quarantine-test.sh` is the
  one entry point for tests, so `env -u` (or an allow-list) there covers land, CI and
  manual runs.
- The harness test fixture clears or overrides the context variables itself, so the tests
  are hermetic wherever they run.

The first is one line and protects every crate; the second fixes the crate that leaked.
Doing both is reasonable.

## Related

`make land-full` is also unusable on this machine today: its gate-slot checkout is not a
git repository (`git rev-parse --git-common-dir` exits 128 in `scripts/landing.py`
`common_repository`). Not investigated.
