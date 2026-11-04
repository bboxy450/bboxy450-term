---
id: "01M6DGQZ3V8R5K2N7T4W9XCB1E"
type: "task"
number: 5183
title: "ctl seat spawn --resume: start a claude or codex seat on an existing native session"
status: done
role: "executor"
verify: "cargo test -p bboxy450-term --lib ctl && cargo test -p bboxy450-harness-wake-up --test seat_wiring"
created: "2026-11-02T12:00:00Z"
updated: "2026-11-02T18:30:00Z"
updated_by: "seat:01M6EQFA2G00BFT3TVPY6AMXE6"
reconciliation: [{"by": "seat:01M6EQFA2G00BFT3TVPY6AMXE6", "disposition": "implemented", "evidence": ["https://github.com/bboxy450/bboxy450-term/commit/8b6ad7b9", "https://github.com/bboxy450/bboxy450-term/commit/e01664a2"], "fresh_accepted": false, "kind": "historical", "previous_status": "open", "reason": "ctl seat spawn --resume landed on main, including --place. Hand-edited: task_reconcile refused (see tasks/2026-11-02-bug-task-lifecycle-unowned-records)."}]
---
# ctl seat spawn --resume

The operator moves running iTerm harness sessions into bboxy as seats. A seat must be
able to resume an existing native Claude session (UUID) or Codex thread instead of
starting a new one.

## Change

- `ctl seat spawn <claude|codex> --resume ID` (and the flat `spawn` form) put `ID` in
  `SpawnSpec.resume`. The backend already maps it to the seat's `harness_session_id` and
  `LaunchContext.resume`, so Claude launches with `--resume ID` (not `--session-id`) and
  Codex with `resume ID`.
- `ID`: non-empty, at most 128 chars, no whitespace or control characters.
- Refused with a usage error: `zsh`, and `--place` (the window's placement lane does not
  carry a resume id; the default placement does).
- Help line for `seat spawn` lists `--resume ID`.

## Acceptance

- [x] ctl parse test: id reaches `Command::Spawn.resume`; zsh, `--place` and bad ids refused.
- [x] launch test: resumed Claude has `--resume ID` and no `--session-id`; Codex has `resume ID`.
- [ ] Landed with `make land`.
