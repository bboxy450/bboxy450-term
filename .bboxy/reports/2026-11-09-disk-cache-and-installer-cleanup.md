---
id: 01M7230JNV7VYS2P48833V46F0
number: 1794292861627427
type: report
title: Reclaim compiler cache and unused installer backups
task: '[[tasks/2026-11-04-codex-trusted-handoff]]'
commits:
- 1881181123d1dca79ec1f2fcfa2d96831243e244
verify:
  command: /Users/feral/Applications/bboxy450-term.app/Contents/MacOS/bboxy450-term ctl app status
  exit_code: 0
author: seat:01M7230JNV5X3C5WTTDWAQF3PD
created: '2026-11-10T06:41:01.627443+00:00'
---
Direct operator request: clean disk space. The author is a standalone execution identifier, not a registered native seat; the historical task is context only.

Removed 10,742 regular compiler-cache content files older than five minutes from the hexadecimal shards of ~/Library/Caches/bboxy450-sccache (10,500,317,184 allocated bytes). Kept the cache root/shard directories, preprocessor metadata, recent writes and observed open files. Each candidate's inode, size and modification time was rechecked immediately before unlinking; concurrent replacements were skipped. No cache daemon was stopped and no compiler output directory, registry source, worktree, credential or toolchain was selected for deletion. Active builds can regenerate missing cache entries through the normal cache-miss path.

Removed 11 unused .bboxy450-term-install directories after checking standard installer layout, matching com.bboxy450.term bundle identity and a fresh lsof open-file check. Kept 5 backups with open files/incomplete unused proof, including the prior Contents used by the preserved Supervisor/harnesses. Exact removed and retained paths are in .bboxy/runs/2026-11-09-disk-cache-and-installer-cleanup.json. The earlier worker PID 9999 had finished before cleanup; the initial historical-PID protection check stopped before any deletion, and protection then used the current live inventory.

Available filesystem space rose from 6,777,249,792 to 18,437,242,880 bytes: a measured increase of 11,659,993,088 bytes, about 11.7 GB. Other machine activity can affect this free-space delta. Terminal runtime/history was inspected for size and retained; it was not pruned.

Before and after ctl app status exited 0 with running/backend_ready/gui_ready/events_ready true. GUI identity/source and the Supervisor/Logic/GUI owner identities match exactly. Current live harness start times remained unchanged; naturally completed processes during cleanup: []. No app, Supervisor, Logic, harness or compiler process was signalled or restarted.

This evidence-only commit uses [skip ci]; no builds, tests, full CI or make land were run for cleanup. The temporary report worktree is removed after its signed commit is on main, its files are clean and lsof shows no users. Runtime data, source and peer worktrees remain intact.
