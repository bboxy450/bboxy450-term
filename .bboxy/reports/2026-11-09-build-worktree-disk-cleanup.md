---
id: 01M71HDSHKM0K402CCQ7NW31QN
number: 1794274420276152
type: report
title: Clean completed build output, owned temporary files and disk space
task: '[[tasks/2026-11-04-codex-trusted-handoff]]'
commits:
- 3da65fd563c86bbd02d7a0f8102656d3b3029509
verify:
  command: codesign --verify --strict /Users/feral/Applications/bboxy450-term.app
  exit_code: 0
author: seat:01M71HDSHKP7A1RSW279T1NQNQ
created: '2026-11-10T01:33:40.275784+00:00'
---
Direct operator request: clean the completed build, worktree and disk space after [[reports/2026-11-09-main-harness-full-restart]]. This standalone execution identifier is not a registered native seat; the historical task is context only.

Removed the completed Term/harness release binaries, app bundle, dependency/build/fingerprint/incremental directories and Cargo output metadata from target/release. Removed this execution pair's two obsolete installation staging directories, two private update/ACP evidence directories, two ordinary-worker temporary build receipts and the owned prior GUI-handoff receipt. Each selected directory passed lsof -F pn +D with exit 1 and no output before deletion; no active compiler output pointed at these artifacts. Exact removed paths and command results are in .bboxy/runs/2026-11-09-build-worktree-disk-cleanup.json.

The prior source worktree and branch were already removed after their commits landed; Git confirmed they were absent. Existing worktrees and formal/peer evidence were preserved. The installed app, source tree, current application runtime and personal Desktop files were preserved. Old report links to the removed private raw build logs now refer to cleaned paths; their committed command/exit/hash summaries remain in Git.

Generated allocated file usage fell by 4,114,558,976 bytes. Available filesystem space rose from 21,122,199,552 to 24,836,599,808 bytes, a measured increase of 3,714,400,256 bytes (about 3.7 GB). Other processes can change shared filesystem free space. target/release retains 96,190,464 bytes of unrelated receipts rather than removing peer evidence.

After deletion, codesign --verify --strict on the installed app exited 0. Its executable SHA256 remained 3c0afada6f990d572aed0e8352190eae48910072df61e897e6688b504577118e. ctl app status exited 0 with running, backend_ready, gui_ready and events_ready all true; GUI 84696 still renders source 3da65fd563c86bbd02d7a0f8102656d3b3029509 at frame 20750. No restart, tests, CI, make land or source-code change was needed. This evidence-only commit uses [skip ci]. Its temporary reporting worktree is removed after the commit is on main and its clean/open-file checks pass.
