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

`.kb/research/iterm-harness-plugin-exec.md`: exact channel, exact variable or message names, exact JSON fields, `file:line — mechanism`. State transitions you observed. `**Gap:**` for unverified. Links: [[vision]] [[ux-sidebar]] [[iterm-harness-plugin]] [[subagent-detection]].

## Verification

```sh
ls .kb/research/iterm-harness-plugin-exec.md && grep -c 'file:\|:[0-9]' .kb/research/iterm-harness-plugin-exec.md
```

Report on `#bboxy450-term`.
