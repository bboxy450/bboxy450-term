---
author: seat:01M6EK48WKEEPDMMY07B3R82SK
authorship:
  kind: seat
  process_generation: '1'
  public_key: P1Y+4lhb0tN1Apt0rjbH9Nw90KVGEsXuGznqcSkr2XA=
  signer: seat:01M6EK48WKEEPDMMY07B3R82SK
commits:
- 7b2aa3c5bb445de98f13b942db13ffe980c749be
- 1aa0339b075a01adaec6800025bd43332192f83a
created: 2026-11-02T17:27:40.886Z
created_by: seat:01M6EK48WKEEPDMMY07B3R82SK
id: 01M6EMTW96VRHQ96ZW87JYA122
number: 3755
provenance:
  digest: sha256:033776cb5168e58eacb057333e41f7aa4031a67d9f38ee05dc7e687673d33f48
  frontmatter_digest: sha256:0d9a5daaec647b0f3e0e62b524d4abb8b7152b53ba7933c8a289cfdcdbccd1c3
  log: .bboxy/log/2026-11-02.jsonl#17149
  sig: 6l3lBNfLWmXyKJWoQXG81tS2h8WzrZNZiy7nHPobe/zn90kibcW3AylY0n5DkQgIuVdjyceUUk3i9StY1OavCg==
  signer: seat:01M6EK48WKEEPDMMY07B3R82SK
task: '#3351'
title: 'Title-bar usage: Oracle landing and interrupted Executor gate'
type: report
updated: 2026-11-02T17:27:40.886Z
updated_by: seat:01M6EK48WKEEPDMMY07B3R82SK
verify:
  command: make test
  exit_code: 143
---
# Title-bar usage landing handoff

Task #3351. Usage Executor 2, seat:01M6EK48WKEEPDMMY07B3R82SK. Worktree: /Users/feral/mydev/bboxy450-term-usage, branch title-bar-usage.

Landed by Oracle at 1aa0339b075a01adaec6800025bd43332192f83a. The Oracle's authenticated decision message 01M6EMQHH0VEXV82QA4JPQ9M19 states that, on the operator's instruction to land with less testing, it pushed main itself and stopped this Executor's make test (reported PIDs 63020/57290). It directs no further rebase, testing or landing. git ls-remote origin refs/heads/main returned that full SHA, exit 0, at 2026-11-02T17:27:06Z.

## Assignment evidence

1. Signed rebase: git fetch origin exited 0 and obtained 0ab3930cf5bfd028c5d63e440fe3448973b16b5f. git rebase --gpg-sign origin/main exited 1 at the single conflict in crates/bboxy450-term/tests/formal_contracts/models.rs. The resolution retains both gitkb_models::cases() and usage_models::cases(); GIT_EDITOR=true git rebase --continue exited 0. The four rewritten commits were b930545d, ff83264c, af2178fb and 8287641d. git range-diff showed the three record patches unchanged and only the combined model registration context in the implementation patch. No feature implementation was extended.
2. Task import and CPU cap: the delegated task file was copied byte-for-byte from the authorized knowledge root; cmp exited 0. Signed import commit 7b2aa3c5bb445de98f13b942db13ffe980c749be was clean. The required pgrep count was 0 before the single make test invocation, started at 2026-11-02T17:02:43Z.
3. Gate: make test exited 143, with terminal output "make: *** [test] Terminated: 15". target/ci-local.log:6 binds the run to 7b2aa3c5bb445de98f13b942db13ffe980c749be in normal mode; line 8 records this worktree and its own target. Line 5044 starts test-harness-native, and line 5070 ends with "make[1]: *** [test-harness-native] Terminated: 15". No green ci-local result marker exists. This is an interrupted run, not an observed failing test assertion.
4. Landing: this Executor did not invoke sh scripts/land.sh --gate-log target/ci-local.log, so there is no Executor land.sh exit code. The subsequent Oracle action is confirmed by the remote main SHA above. No retry, additional full run, installation or further publication was performed by this Executor.

## Gate log and completed checks

Gate log: /Users/feral/mydev/bboxy450-term-usage/target/ci-local.log
SHA256: d350c455bbabc18726a530c83e8e4745387808533bf848068596a9d22be877dd

The log records 164 landing fixtures and 37 resource-ledger tests passing; landing budget controls passed. Workspace build completed at line 98 and strict workspace Clippy at line 137. Workspace tests and doctests completed before the capability scan/control commands at lines 5035/5041. The capability broken control records red_exit 101, green_exit 0 and restored source hash immediately before test-harness-native.

Feature evidence on the tested 7b2aa3c5 source includes the snapshot owner test at line 1924, UI strip test at line 4535, machine/poller tests at lines 4663-4665 and provider tests beginning at line 4669. The prior missing-app preflight regression passed at line 1950. Native project models and broken controls passed at line 2118. Full pinned Java TLC/Yarn verification, final smoke and the final invariant pass were not reached. Existing canonical ignored/night exclusions were retained.

## Final source and limits

The reflog records an external rebase at 2026-11-02T09:25:42-08:00 onto 9ce7d1db, producing the Oracle-landed 1aa0339b. That later source differs from the tested 7b2aa3c5 in the ctl-default implementation and invariant records. The interrupted gate therefore cannot certify the landed source. The five landed branch commits show signature G with approved fingerprint 9B07E849480C68328CA20684844ECC0587DE7AB0.

HEAD and origin/main were equal to the remote-confirmed landed SHA and git status --porcelain was empty before preparing this reporting-only commit. This report and its native responsibility/task records are the only subsequent changes. No post-stop tests or push are planned; the final task_report names the containing signed report commit and actual gate result. The Oracle owns the stated operator exception and independent acceptance.
