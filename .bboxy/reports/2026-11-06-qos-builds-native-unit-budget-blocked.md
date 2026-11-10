---
type: report
title: QoS transfers applied; two C1 native-unit CPU failures remain
task: '[[tasks/2026-11-06-qos-builds-never-starve-mcp]]'
author: seat:01M6S48V44QDK7SE9H4EDFPXVT
created: 2026-11-07T08:00:53.421Z
commits:
- e818118c1ebe4f427e7d6c0cac1f167776ef695b
- 09623ce039162f722e181858732dd380e6ef8a07
- d34e2f9e59589643408c69dcf76bab211cc26a31
- 148878086965929ea4d3fc15ed28aa6e80c3cc7c
- 630297a6409ac38f7573d12abb67314106e76f5b
- 314b419d55edf0142276eb46c8207841523ff025
- 741c15f328b52084cac9edf6845b692dc1d18e57
- ee34f00a4eb8144bcea2ba31d5f7cb67b2711651
- 3e4868c5362c89175c7deddf4b07aa773d7288ff
- 8eb80932e77c12ea2d79e912748253fc25d035d5
- 9132893cfb17eeca6366e43ba68e1b5bbdb007c1
- 12cdf01e53f17e52de6f74c6b2bce593cce69c19
- c22b70523aa9a99c4038bc2db66d9e456a449c05
- 5341e4ef23a53d318d0d2b9dd43c6804b11ee314
- ee60f8184aa1b0181d58a2f55bffb99501eca7d0
- 0350aa184a0f0598616c72e93922abefb8919b50
- d4a8c1f6389b8657ad0cc1cdbd3e627f087bf79f
- 38d99096f4f9c475a5547cf2192982af6ff7a5c0
verify:
  command: make test-dream-c1-qos
  exit_code: 2
supersedes: '[[reports/2026-11-06-qos-builds-reenable-blocked]]'
id: 01M6TGCM01QQBY0Q0XQ419T2J6
updated: 2026-11-07T08:00:53.421Z
number: 93377961366242
created_by: seat:01M6S48V44QDK7SE9H4EDFPXVT
updated_by: seat:01M6S48V44QDK7SE9H4EDFPXVT
authorship:
  signer: seat:01M6S48V44QDK7SE9H4EDFPXVT
  kind: seat
  public_key: 9eWyjSVdBKBGoA72B1nDgpObDHOHT0P0DKq9XWjgcwI=
  process_generation: '1'
provenance:
  digest: sha256:e7566b3dda62dc5110a68627036a8315d59e8f446c22097bacc1ad581fdbf09f
  frontmatter_digest: sha256:d49a5dc291f6c0cd176c227bfdb70c96f88109bd23f4c9512500c5d0514e06b1
  signer: seat:01M6S48V44QDK7SE9H4EDFPXVT
  sig: Af0rOPz3cslqLxDQmsFTZivWw/xza5dfYwqjaICG6ZbkxYTJAKIaGIXmHdTMTTtzXBGnsocUccN6TmijJsMNDA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150240
  event:
    id: 01M6TGCN8MWFBJYD39QP3F82D2
    sequence: 150240
    hash: 70f0681733d525ab90ff41f960669c5b6f461c8967360b93e8a125f4372744b0
---
# Three transfers applied; two C1 unit-budget failures remain

Primary QoS remains LANDED at `ee60f8184aa1b0181d58a2f55bffb99501eca7d0`, through canonical make land exit 0: 679 native/TLC models, 254 contracts, 34 inventoried crates; 5100.288 s wall, 4751.873 child CPU seconds, +8,437,059,584 target bytes. Full mechanisms, changed-file inventory, source-bound primary formal receipt, successful earlier core/headless checks and retained evidence are in [[reports/2026-11-06-qos-builds-reenable-blocked]]. Those primary mechanisms have not changed.

The Oracle's newest ruling was fetched from [[tasks/2026-11-06-qos-builds-never-starve-mcp]], etag `9cd34f692bf81285f10f2f87beb649775b50c6bdd04387af1dc1aec4cbcfdb2b`. Its three approved defect transfers were applied exactly and folded into the signed, unlanded last commit, now `d4a8c1f6389b8657ad0cc1cdbd3e627f087bf79f`:

- admitted-MCP: exact known-defect reason points to [[tasks/2026-11-06-dream-c1-honest-capture-errors]];
- held-catalogue control: exact known-defect reason points to [[tasks/2026-11-06-dream-c1-cold-mcp-latency]];
- source-error capacity: exact known-defect reason points to [[tasks/2026-11-06-dream-c1-two-owner-storage-race]].

Only those approved ignore attributes, explicit policy deferral entries and LAND documentation changed. The headless compiler wrapper fix and ten re-enables remain in the two follow-up commits (0350aa18 plus d4a8c1f6). Assertions, production source and CPU/wall budgets remain unchanged. The task is **not complete**; no completion task_report has been called.

## Fresh executable evidence at d4a8c1f6

| Command | Actual result |
| --- | --- |
| make test-dream-c1-qos | exit 2; ten nonempty one-test selections, eight passed, two failed; the three authorized deferrals were printed |
| Isolated source-failure test, same packages/targets/default-policy runner, warm artifacts | exit 100; same ResourceLimit |
| Isolated final-authority ABA test, same packages/targets/default-policy runner, warm artifacts | exit 100; same ResourceLimit |
| python3 -I scripts/test-landing.py | exit 0; 204 fixtures, 71.651 s reported test runtime |
| python3 -I scripts/qos-probe.py --check | exit 0; real private native MCP; 100 measured requests per phase, ten load children |

The two newly persistent failures are:

1. `dream_c1_final_actual_authority_rechecks_row_head_grant_inode_aba_after_debt`: first C1 pass refused journal initialization at registry/dream_c1.rs:347 with **1,127,250 ns CPU > 1 ms** (fixture unwrap at logic.rs:14040). The one isolated retry refused replay at registry/dream_c1.rs:1068 with **1,050,625 ns CPU > 1 ms** (unwrap logic.rs:14042). Both are ResourceLimit before the final-authority assertions.
2. `dream_c1_source_failure_is_bounded_immutable_and_fenced_before_any_replay`: first C1 pass refused native work at registry/dream_c1.rs:661 with **1,075,583 ns CPU > 1 ms** (unwrap at line 3779). Its isolated retry refused the same unit with **1,442,375 ns CPU > 1 ms** (unwrap at line 3750).

Each failed twice on this source, including its one warm isolated retry; no third identical retry was run. The machine snapshot after the ten-case run showed **65.99% idle**, load 3.93 on ten logical CPUs; the isolated runs recorded load about 4.00. There was no concurrent build or test from this seat during their executions. The named measurements use the verified default-policy nextest runner, with compiler trees background. Moving builds off foreground policy does not guarantee every cold SQLite/setup unit costs at most one millisecond of CPU.

All other active C1 selections passed in that run, including all five wake-up process tests, custody, held/clock/CPU final-authority refusals and journal registration. The two failing tests remain enabled; neither was quarantined, re-ignored, given extra time nor changed to expect refusal.

The fresh MCP probe is still responsive:

| Phase | p50 ms | p95 ms |
| --- | ---: | ---: |
| Unloaded | 1.501541 | 2.080791 |
| Before: default-policy saturating load | 4.426541 | 6.243417 |
| After: background-policy saturating load | 1.431792 | 2.042542 |

All ten background children reported native self background=1 and ps priority 4; bridge requested class=25. This compares load policies using the same fixture/source, not historical old/new app binaries. Real-load latency remains observational per the amendment. Earlier deterministic controls and complete requested verify chain passed at fe8f5bf5, as recorded in the superseded report; that is not a claim that the full chain ran at d4a8c1f6.

## Exact remaining decision

The newest ruling requires ten successful re-enables plus the three transfers. The current ten-case run has eight passes and these two native CPU-budget failures. Under the brief's separate-C1-budget-task escape, assign their repair/re-enable to a C1 native-unit-budget task with explicit known-defect reasons, or provide the C1 source fix. These production native-budget/setup paths are outside this frozen QoS-only scope.

The three previous blocker dispositions are implemented. Do not repeat those instructions as a resolution for these two different failures. No additional ignore, quarantine, assertion change or product fix was inferred from the three-test ruling.

Canonical follow-up make land, its current-source formal refresh, the final reports/2026-11-06-qos-builds.md and the one completion task_report remain due. The driver stopped at the red C1 prerequisite; it did not run or claim a successful follow-up land. Rebuilding the full app/core chain again before the remaining source disposition would produce stale source-bound evidence, so only the still-independent policy fixtures and private probe were refreshed.

## Retained artifacts and worktree

Raw logs and exact command/exit JSON are in:

`/Users/feral/mydev/bboxy450-term/.git/bboxy-qos-builds/01M6S3M8WTNZ32TMZ7YX9VTEJR/term-d4a8c1f6389b8657ad0cc1cdbd3e627f087bf79f/`

Key files: verify-and-land-qos-final-transfers.log/json (the ten-case attempt), verify-and-land-qos-final-retry.log/json (the two isolated retries), qos-final-transfers-load.txt and verify-qos-transfers-policy-probe.log/json. Manifest SHA256: `959537a341d77e9d059067bcc83c3792c83ce83248ff3fb5bc1934e41edb4c67`.

Worktree is clean; git cherry origin/main HEAD lists exactly 0350aa184a0f0598616c72e93922abefb8919b50 and d4a8c1f6389b8657ad0cc1cdbd3e627f087bf79f. The amended commit's Boxy/9B07E849 signature verified. All author build/test profiles remain cache-off, dev/test debug=0, opt-level=1, debug assertions=true. No full root make test, ordinary one-launch C1 runtime, reinstall, operator app or peer process was touched. No test required operator input.

Artifact_claim is omitted under the explicit final ruling that its rejection of committed checkpoints is a known bug. Existing primary/harness formal archives and the prior blocked reports remain intact.
