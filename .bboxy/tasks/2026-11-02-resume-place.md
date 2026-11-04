---
id: "01M6DQ4R7X2B9K5N3T8W6YCE2F"
type: "task"
number: 5184
title: "ctl seat spawn --resume with --place: resumed seats open in their own tab or space"
status: "open"
role: "executor"
verify: "cargo test -p bboxy450-term --lib ctl && cargo test -p bboxy450-term --lib launcher && cargo test -p bbt-proto --lib gui"
created: "2026-11-02T16:00:00Z"
updated: "2026-11-02T16:00:00Z"
---
# ctl seat spawn --resume with --place

Follow-up to #5183 (`.bboxy/tasks/2026-11-02-ctl-seat-resume.md`). `--resume` refused
`--place`, so every resumed seat took the default placement and the operator's resumed
seats were tiled into one window as unreadable slivers. A resumed seat must be able to
open in its own tab or space.

## Change

- `bbt_proto::gui::UiSpawn` gains `resume: Option<String>` (serde default, skipped when
  absent, so older peers stay compatible); `UiRequest::validate` bounds it at 128 chars.
- `ctl seat spawn <claude|codex> --resume ID --place right|down|tab|space` puts `ID` in
  `UiSpawn.resume`; the refusal is removed. The zsh refusal and id validation stay.
- `launcher::operator_placement` copies `pick.resume` into the `SpawnSpec.resume` it
  captures.

## Acceptance

- [x] ctl parse test: `--resume X --place tab` yields `SpawnAt` carrying both values.
- [x] launcher test: the captured placed spawn's `SpawnSpec.resume` equals X.
- [ ] Landed with `make land`.
