# Task E: mux daemon, crash survival, mosh-style remote seats — research (redundant pass)

Date: 2026-10-21. Owner: Vesper Greywind <ORACLE>. Assignees: one Oracle subagent (writes `research/cloud-and-mosh.md`) and one Executor (writes `research/cloud-and-mosh-exec.md`). Status: OPEN for Executor claim. Converged by the Oracle into `specs/mux-daemon.md`.

## Goal

Seats survive terminal crash and restart. Remote seats over ssh or in a cloud VM reconnect like mosh. One mux must hold 1000 seats. See [[vision]] and [[roadmap]].

## Questions

1. mosh: SSP, what state syncs, prediction, roaming, ssh bootstrap then UDP handoff. What transfers to a QUIC transport we own.
2. tmux, zellij, wezterm mux server, Eternal Terminal: where the PTY-owning server/GUI split is, resume protocol, scrollback on reconnect. Clone and cite `file:line`.
3. Where the VTE state machine runs at 1000 seats: daemon (ship screen diffs) or GUI (ship bytes). Pick one with per-seat memory/CPU numbers, measured if possible (`alacritty_terminal` grid with 10k scrollback).
4. Remote bootstrap: one binary dropped over ssh, identity bound to the user key, dials home via our NAT-punching transport. Fly.io, Hetzner, AWS, GCP, plain ssh.
5. Future: Codex → Claude session migration. Where each stores transcripts (list structure under ~/.codex and ~/.claude/projects; do not read content).

## Deliverable

`.bboxy/research/cloud-and-mosh-exec.md`. Sections per question. Facts as `file:line`, `command — result`, or `source: url`. `**Gap:**` for unknowns. Links: [[vision]] [[roadmap]] [[cloud-and-mosh]] [[terminal-core]].

## Verification

```sh
ls .bboxy/research/cloud-and-mosh-exec.md && grep -c '^## ' .bboxy/research/cloud-and-mosh-exec.md
```

Report on `#bboxy450-term`: file, commands + exit status, measured vs estimated.
