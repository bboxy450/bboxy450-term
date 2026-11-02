---
id: "01M6DGQZ3V8R5K2N7T4W9XCB1E"
type: "task"
number: 5183
title: "ctl seat spawn --resume: start a claude or codex seat on an existing native session"
status: "open"
role: "executor"
verify: "cargo test -p bboxy450-term --lib ctl && cargo test -p bboxy450-harness-wake-up --test seat_wiring"
created: "2026-11-02T12:00:00Z"
updated: "2026-11-02T12:00:00Z"
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
