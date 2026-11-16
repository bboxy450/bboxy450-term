---
authorship:
  device_id: 01M677XWZPW1XGTQYYGA50W74M
  issued_by: test-account:01M677XWYWXAQQM5DEQP83TQYY
  kind: device
  principal: operator:01M677XWZPW1XGTQYYGA50W74M
  public_key: cHjhL+5pI9VNQE+Uhb4HJBnYoPqSIH7aCYU+b9MElWs=
  signer: device:01M677XWZPW1XGTQYYGA50W74M
created: 2026-10-25T18:19:09.688181Z
created_by: device:01M677XWZPW1XGTQYYGA50W74M
id: 01M5T4KCZR2N43YC42JQQ5NMDB
issued_by: test-account:01M677XWYWXAQQM5DEQP83TQYY
number: 332
owner: Sable Embertail
provenance:
  digest: sha256:a64ac396b7317df5aee755a876772f65c8590f05dff5bafbd8c4479d4bdabcea
  frontmatter_digest: sha256:8225703d18c0fce7afe8d22db24c21270d2b312e8b813d9021758fb52c53f912
  log: .bboxy/log/2026-10-30.jsonl#707
  sig: j7RjkinUaSPfixK8zhpVc1Jn5ZqrrXM2OZA30a4A3msdkfnOgJRkDvyghqUMAMijzaRpfpm2VOkaJYtWHEX1CA==
  signer: device:01M677XWZPW1XGTQYYGA50W74M
reconciliation:
- by: operator:01M677XWZPW1XGTQYYGA50W74M
  disposition: implemented
  evidence:
  - https://github.com/bboxy450/bboxy450-term/commit/77b190beaa40474f37d434bee46bc971267008d2
  fresh_accepted: false
  kind: historical
  op_id: 01M677Y21F8A9J7GZK53NMTMVT
  previous_status: open
  reason: Operator-authorized historical msgrec reconciliation, 2026-10-30; implementation already landed on main; no new acceptance claimed.
role: executor
status: done
title: Make ctl stdout BrokenPipe exit quietly
type: task
updated: 2026-10-30T20:27:32.390Z
updated_by: device:01M677XWZPW1XGTQYYGA50W74M
verify: cargo test -p bboxy450-term --locked --test ctl
---
# Make ctl stdout BrokenPipe exit quietly

Opened at Vesper's explicit request (ICQ `0f2f1700-cb19-4bec-bb70-7ad716834b49`). This is a separate follow-up; no pipe-handling product change is part of #328 or the spawn diagnostic.

## Evidence

- `scripts/e2e/T6.sh:24` pipes `ctl seat screen <seat> --human` into `grep -Eq`. The consumer may exit after its first match; the pipeline returns grep's status.
- The terminal night on source `223b749c3235f9e9da00b1e9a0d81523f903492a` produced a T6 script verdict `pass`, exit 0, and this stderr: `library/std/src/io/stdio.rs:1168:9` — `failed printing to stdout: Broken pipe (os error 32)`. The individual emitting command was not traced; no application source line is claimed from that library-only trace.
- The Rust CLI executable was `/Users/feral/mydev/bboxy450-term-exec3/target/launch-night/debug/bboxy450-term`, SHA-256 `7e6aa392a3f82b0cb1660e4c7e1cda0638bf2e033a34ebcd6961324fcdf71f67`. The night, all nine script verdicts and all record writes passed; the runner stayed amber for tracked partial coverage. All ten app PIDs and endpoints were gone, with file custody and zero windows.
- Raw command/producer manifest, complete captured T6 evidence and audit: `/var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/sable-launch-night-iloqd6d6/wait-terminal-night.command.json`, `.stdout.jsonl`, `.result.json`. Seraph's source review (`e99c5e5b`) identifies `grep -q` closing early and bare ctl `println!` writes as the mechanism candidate. A controlled closed-reader regression is still owed; the precise application call site is unproven.

## Controlled reproducer (current bug, not a fix)

The retained identical binary at `/var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/sable-launch-night-iloqd6d6/bin/bboxy450-term-wait-223b749c` was launched as `ctl --help` with the stdout pipe reader closed before process spawn. It exited 101 and printed the same BrokenPipe panic; no endpoint, account, desktop or harness was opened. `t6-closed-pipe.result.json` records PID, arguments, producer hash, elapsed time and exit; `t6-closed-pipe.stderr.log` retains the backtrace. This independently proves the CLI closed-reader bug, without claiming which particular T6 command emitted its earlier panic.

## Acceptance

Use a controlled pipe whose reader closes before the CLI writes, for both a small response and a response larger than the pipe buffer. Assert actual CLI exit 0 and no panic on stderr. Keep unrelated output errors visible. Preserve the JSON output and error envelopes when the reader remains open. A named regression must execute (not a zero-test filter), and the original ctl suite and T6 night must pass.

Coordinate with Seraph's #320 ctl noun split before changing print paths. Do not touch the window, identity, credentials or T6's lifecycle assertions. Resolution owner and cut are for the Oracle to assign; this record does not claim the bug fixed.
