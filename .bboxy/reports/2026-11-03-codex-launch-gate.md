---
id: 67V7WE0FAA8FGBB2TW8T3JTBD2
number: 5002229
type: report
title: Unified launch fix and startup-modal landing repair
author: "seat:01M6HCFJ8QFC193R49JW3G0VQX"
task: "[[tasks/2026-10-21-w2-3-adapters-claude-codex-zsh]]"
supersedes: "[[reports/2026-11-03-codex-launch]]"
commits: ["47744aba79e9c7d31e1cdf8ac7746b22032c0068"]
created: "2026-11-03T19:23:12Z"
verify:
  command: "cargo nextest run --locked -p bboxy450-term --lib --bin bboxy450-term -E 'test(keyboard_adapter_uses_neutral_core_and_live_modes) | test(arbitrary_dialog_traps_keys) | test(pointer_cursor_follows_real_controls) | test(selection_pointer_owns_mouse_mode)'"
  exit_code: 0
---
# Unified launch fix and startup-modal landing repair

The launch implementation and native Codex startup evidence are in
[[reports/2026-11-03-codex-launch]]. Same worktree, branch, direct operator brief
and external-conversation author identifier; no native seat grant is claimed.

The first make land attempt passed strict all-target Clippy and 19 harness
unit tests, then stopped before push: the app ran 431 tests, with 427 passed,
four failed and ten skipped. The shell make command returned 2; its inner gate
reported 77. Formal verification had not been reached.

All four failures came from the preceding startup-modal change on main:

- `crates/bboxy450-term/src/main.rs:9148` — the startup key guard now lives in
  the existing sole WindowEvent::KeyboardInput arm, preserving Quit/HardQuit,
  Retry and modal input trapping without introducing a second physical key path.
  The original keyboard adapter assertion is unchanged.
- `crates/bboxy450-term/src/app_overlays.rs:1221`,
  `crates/bboxy450-term/src/pointer_cursor.rs:298` and
  `crates/bboxy450-term/src/terminal_clipboard.rs:280` — these three retained-scene
  fixtures explicitly mark startup attached before testing controls beneath it.
  Their original cursor and selection assertions remain unchanged.

The exact four-case nextest recheck above returned 0: four passed, 437 skipped.
`make test-licenses` also returned 0 (six policy fixtures and nine notice
fixtures). `git diff --check` returned 0. No quarantines or gate bypasses were
added. The next make land attempt still must pass its complete selected gate
before the push to main and requested removal of this worktree.
