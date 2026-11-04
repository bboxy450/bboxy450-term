---
id: 67NN07TGQ38YBQFF41FS87CBH8
number: 5002232
type: report
title: Launch fix landing and persistent macOS 1Password access setup
author: "seat:01M6HCFJ8QFC193R49JW3G0VQX"
task: "[[tasks/2026-10-21-w2-3-adapters-claude-codex-zsh]]"
supersedes: "[[reports/2026-11-03-codex-launch-formal-coverage]]"
commits: ["3eafd2be0c5fef791da4428ca4f6c26782d75c27"]
created: "2026-11-03T20:13:19Z"
verify:
  command: "make privacy-1password && make help && git diff --check"
  exit_code: 0
---
# Launch fix landing and persistent macOS 1Password access setup

The operator explicitly requested a quick main push with CI skipped on
2026-11-03. This is the authorization for a direct signed Git push rather than
rerunning the previously blocked landing gate. It is a one-time exception;
the repository's normal Make landing gate is unchanged. The source branch was
rebased on origin/main d110be5b without conflicts. No CI or full suite was run
after this rebase. Same external-conversation author identifier as the previous
reports; no native bboxy seat grant is claimed.

Makefile:635 — `make privacy-1password` verifies the installed app's approved
bundle/team requirement, reveals it in Finder and opens the Full Disk Access
settings page. It succeeded on the installed signed app (exit 0). An initial
attempt used codesign's file-requirement syntax and failed before opening
anything; the final command uses an inline requirement and passed.

packaging/macos/README.md:12 — documents the persistent supported remedy.
Apple's app-data permission resets when the responsible app quits; Full Disk
Access suppresses this additional prompt. The operator must add/enable the
installed app once. This command does not grant permission, change 1Password
authentication or establish that the operator has enabled the switch. No
private entitlement, TCC database edit or undocumented 1Password environment
override was added. Existing harness/backend processes were left running.
Source: https://developer.apple.com/videos/play/wwdc2023/10053/.

`make privacy-1password`, `make help`, and `git diff --check` each returned 0.
CI was deliberately skipped under the current operator instruction. Prior
source verification is recorded in the preceding reports: focused launch/Bridge
tests, affected Rust tests and strict lint, plus the clean full formal run at
528d6d03 (270 models, 34 crates, 111 contracts). That receipt is for the earlier
source revision, not a new exact-HEAD receipt. The prior landing attempts
stopped on the import timeout and inherited report-corpus errors; this report
does not claim those gates passed.

The rebased launch commits are 3f3a77d5 (shared preparation and required MCP),
6f2eddce (startup-modal fixes), 7b7b0fc2 (Codex readiness model) and 3eafd2be
(pinned harness coverage). The final commit also includes this setup command
and report. Push and subsequent cleanup are checked against origin/main after
the commit; this pre-push record does not substitute for that remote evidence.
