---
id: "3KNSDY7WT5MB6PY94HJ3KD2017"
type: "task"
number: 70
title: "Task H: how iTerm2 detects native Claude subagents — research (redundant pass)"
created: "2026-10-21T18:56:26Z"
updated: "2026-10-24T00:10:38Z"
---
# Task H: how iTerm2 detects native Claude subagents — research (redundant pass)

Date: 2026-10-21. Owner: Vesper Greywind <ORACLE>. Assignees: one Oracle subagent (writes `research/iterm-harness-plugin.md` and `specs/subagent-detection.md`) and one Executor (writes `research/iterm-harness-plugin-exec.md`). Status: OPEN for Executor claim.

## Goal

iTerm2 3.7.6's Session Status panel shows a Claude Code seat with nested rows for its native subagents, each with a description and working/idle state. Screenshot: `runs/ref-iterm-subagent-detection.png`. We do the same in the sidebar, taking the implementation details. See [[ux-sidebar]] and [[vision]].

## Where to look

- `~/.config/iterm2/cc-status` — the hook script wired into every Claude Code hook event in `~/.claude/settings.json` (SessionStart, PreToolUse, PostToolUse, Stop, SubagentStop, Notification, PermissionRequest, …). Read it fully. Never print credential values.
- `/Applications/iTerm.app/Contents/Resources/claude_usage.sh` and the rest of the bundle: how the panel is fed (OSC 1337 user vars? Python API? file?). `strings` the binary for variable names if needed.
- `~/.codex/hooks.json`, `~/.codex/config.toml` for the Codex side.
- Claude Code hooks docs for SubagentStart/SubagentStop payloads (agent id, description).

## Deliverable

`.bboxy/research/iterm-harness-plugin-exec.md`: exact channel, exact variable or message names, exact JSON fields, `file:line — mechanism`. State transitions you observed. `**Gap:**` for unverified. Links: [[vision]] [[ux-sidebar]] [[iterm-harness-plugin]] [[subagent-detection]].

## Verification

```sh
ls .bboxy/research/iterm-harness-plugin-exec.md && grep -c 'file:\|:[0-9]' .bboxy/research/iterm-harness-plugin-exec.md
```

Report on `#bboxy450-term`.

## Addendum 2026-10-21 (Oracle): the Executor pass is now a live capture

The subagent found the channel: OSC 7501 emitted by Claude Code and Codex themselves after a terminal probe echo. See [[iterm-harness-plugin]] and [[subagent-detection]]. The redundant Executor pass changes scope to **proving it on the wire**: run `claude` (and `codex`) under `script` or a small PTY harness that answers `ESC]7501;?ESC\` with the echo, launch one Agent-tool subagent, capture the raw PTY bytes to `.bboxy/runs/osc7501-claude.bin` and `.bboxy/runs/osc7501-codex.bin`, and write `.bboxy/research/osc7501-capture.md` with the decoded sequence of reports (state, id, title) and timing. Never capture transcript content; only the escape sequences and timestamps.
