---
type: report
title: 'Codex Oracle takeover: operational cleanup and standalone delivery; rich panes pending'
task: '[[tasks/2026-11-03-codex-oracle-takeover]]'
commits:
- ea753072c7ec2a0a278e5086ef21458efa0a08f0
- af65c6cf82d9dde6314b1771b9314061b0e24212
verify:
  command: python3 -I /tmp/bboxy-oracle-takeover-01M6H97SAX3GN1BYB4YG76P3P0/verify.py --operational
  exit_code: 0
author: seat:01M6H97SAX3GN1BYB4YG76P3P0
created: 2026-11-03T20:23:21.118Z
id: 01M6HH95QSSAXPSV41HFYVMT4Q
updated: 2026-11-03T20:23:21.118Z
number: 17033
created_by: seat:01M6H97SAX3GN1BYB4YG76P3P0
updated_by: seat:01M6H97SAX3GN1BYB4YG76P3P0
authorship:
  signer: seat:01M6H97SAX3GN1BYB4YG76P3P0
  kind: seat
  public_key: B3hFSjBIE1Nnj7mw9Eg9pqyRpYHF/GQaKXPyV1KAod0=
  process_generation: '1'
provenance:
  digest: sha256:56499823aff2aea470d6ccd93c25788d93afcf3d8e84b31b758680c55d2c7aa8
  frontmatter_digest: sha256:7a663c1d2d0d95af0987d05262f735cbe8c60ba4ec60d37dba350e7a5627a063
  signer: seat:01M6H97SAX3GN1BYB4YG76P3P0
  sig: x5JLwRz4Rnelx80rL0YnS8LL7tlCOGqM8No6vJ1zNtGJPzB6s4X4/deW1upRP8X96w47LJoDb6L6cdmdKcIYBg==
  log: .bboxy/log/2026-11-03.part-86213.jsonl#98530
---
ROLE: ORACLE. Intermediate report for [[tasks/2026-11-03-codex-oracle-takeover]]. This reports actual operational cleanup and the delivered standalone repair. The shared rich-pane product outcome remains pending; the parent task is not complete.

MCP orientation authenticates seat 01M6H97SAX3GN1BYB4YG76P3P0, generation 1, Oracle class, swarm 01M649102FBS60QWNPZS5VAGF9. The operator explicitly assigned this Codex Oracle. No implementation source was written by this Oracle; three signed, scoped Executor assignments carried the product work.

| Evidence | Observed result |
|---|---|
| MCP usage_read at 2026-11-03 20:17:53 UTC, sampled two seconds old | Claude weekly 98%, current session 13%; Codex weekly 69%; CPU 100%, memory 69%, disk 91%. Initial disk was 97%. Provider values are usage snapshots, not account changes. |
| Original current-swarm Claude Oracle/Architect/Creativity seats | 01M6H9704RM1WGA5MZYG8YBJBR, 01M6H970AZDNH8MPVMAFNFPYMZ, 01M6H9714ZDG7QEARG33YVJNQN closed_retained; adapter source exit, no open turn. MCP close ops 01M6HCJZJYEW1EEKP8069WMKX0, 01M6HCJZQFX1XDJ59NRJ13XHDE, 01M6HCJZTRZNWZZPA215SXQHAZ. |
| Latest explicit Vesper request | Current-swarm Vesper Yarn 01M6HEDKGMGNSJYV4KEYAH4C70 closed_retained, PID 77009 absent, adapter exit. Confirmation op 01M6HEYVA22T4Y7J6763Q86V3F. |
| Standalone selector | Backed up the nonsecret default-profile selector, atomically set codex-exec, preserved 0644 ownership/mode; MCP harness_profiles confirmed the effective default. No credential/profile-definition changes. |
| Original interrupted source | All 20 initial dirty files recovered byte-exact from their signed original/rebased snapshots, archived outside Git. Six old branch tips remain under refs/bboxy/takeover/20261103; extra interrupted pane commit 433107e remains reachable. |
| Safe worktree cleanup | Only clean, fully merged, unused harness-nyx and term-restart-build removed after Git/submodule/lsof checks. Six unfinished source worktrees remain. |
| Retired compiler caches | Reclaimed 64.94 GiB from exactly authorized target/{debug,release}/{incremental,build,deps,.fingerprint} paths after lsof/symlink checks. Logs, captures, packages, formal receipts, source and refs retained. Disk free rose from about 15 to 68 GiB before ongoing builds. |
| Operational instrument | verify.py --operational exit 0: 20 files, six refs, three original closed Claude seats, three historical Codex replacement assignments and two safe removals. This check explicitly does not prove final rich embedding. |

The exact original terminal Claude controller PID 54827 was retired under the signed brief after checking parent/start/cwd. The immediate one-second probe still saw it and failed; the later process observation proved it and its two MCP children exited. A subsequently started external Claude controller and a Claude seat in another swarm remain outside this seat's lifecycle grant. MCP refused the cross-swarm close as "target outside caller swarm" (01M6HD2733WYQV0PQ9EN2CRAED); operator notice 01M6HD274CFB23J9SXV8P4QRPE records that restriction. No shell kill bypass followed. Historical pre-restart records still labelled open have no adapter/viewport; one close refused "target viewport unavailable". These stale registry labels are not evidence of live Claude processes.

The three replacement assignments were native Codex with exact worktree, allowed paths, verify command and signed Delegate:
- [[tasks/2026-11-03-codex-harness-recovery]], recovery worktree, RUSTC_WRAPPER= make test.
- [[tasks/2026-11-03-codex-rich-socket]], harness-pane-b12, RUSTC_WRAPPER= make test.
- [[tasks/2026-11-03-codex-rich-host]], term-codex-rich, make test-parity plus native/boundary witnesses.

The standalone task completed at signed ea753072c7ec2a0a278e5086ef21458efa0a08f0; af65c6cf82d9dde6314b1771b9314061b0e24212 adds only its indexed report/summary [[reports/2026-11-03-codex-harness-recovery]] (#16395). Both fast-forward pushes exited 0. The root cause is src/frontends/gui/assets.rs:29: shared TitleBar requested bitmap "bboxy", but startup registered only "bboxy-mark". Registering the same existing portrait under both names fixes the real native first frame. The negative control exits native 1 for the missing asset and restores native 0. Quiet Native/Usage geometry suppression, shared capture/chrome/zoom, phase heartbeats and IR 1.2 agent turns were recovered without dropping either formal contract family.

Oracle verification inspected the actual source/diff, logs and GPG signatures. Full RUSTC_WRAPPER= make test exited 0 on ea75307 in 1235.875 seconds, with 20 core and four GUI commands zero. Every one of the 270 canonical input hashes still matches af65c6c. Seven commit signatures match Boxy's approved key. A fresh make check-formal-evidence exited 0 at clean af65c6c: 99 bounded models/controls, 34 contracts, one crate; native/TLC reachable graphs and registered verdicts agree. Current receipt sha256 00a083344fa7b5b33e91c99572fe5125c7f9fef7670bc095a408d321b525fded is retained under the recovery worktree's verification/evidence directory. Seven same-lab native captures are byte-identical; no baseline adoption or universal target parity claim is made.

Canonical make install in harness-land at ea75307 exited 0 in 630.184 seconds. Installed ~/Applications/bboxy450-harness.app has exact BBOXYSourceRevision ea75307, strict/deep codesign success and approved Apple team UQB3368A84. Running the existing scripts/smoke-app.py against that installed bundle exited 0: private HOME, hidden native window, configured Metal surface, live frame submitted to private texture, transcript validation and clean process reap. This does not prove visible surface presentation. A subsequent normal open exited 0; PID 44858 was observed alive with gui --cwd /Users/feral --profile codex-exec.

The standalone author was initially blocked on MCP reads. Its report arrived during close grace after the task was reassigned. Resuming the same Codex thread on fresh seat 01M6HGY3V1KRZQVGFP17JVJ7JM still authenticated the old closed seat in orient; the worker correctly refused further execution. Both old/replacement seats were retired without source changes. Original report op 01M6HGWRQ3CPMH7CF50CMK3HS5 and verified landed source were reconciled as historically implemented (01M6HH4T8JRMQ9NG6KESV96XDS), not fabricated fresh acceptance. The native-resume MCP identity defect remains an explicit gap; no capability file or credential was read or altered.

The socket and term workers remain active. Socket focused GUI/process and 104-model formal checks are green, but its combined recovery/main full gate remains pending. Term has committed shared HarnessView hosting, persisted face/cursor state, zero-PTY socket delivery and harness resume admission, plus a drain-tail rearm fix; final published dependency, full canonical gates, integrated process controls and two native visual passes remain pending. The measured preliminary row mismatch is 0.0 and four-view composition p99 0.148292 ms; memory is only an allocation and under-load behavior is unmeasured. These preliminary numbers are not final acceptance.

Evidence instruments, raw logs, captures and source archive are outside Git at /tmp/bboxy-oracle-takeover-01M6H97SAX3GN1BYB4YG76P3P0; overview.json names exact logs and operations. The committed compact run summary carries hashes/exit results only. No raw transcript, credential change, broad kill, stash/reset, unsigned commit or unverified acceptance claim occurred. This report deliberately leaves the overall product task open.
