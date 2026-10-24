---
id: "5BHTDHEKE7T9N5X271QGX1AMVH"
type: "task"
number: 102
title: "W2.1 — bbt-mux: supervisor, PTYs, lease protocol"
created: "2026-10-21T22:20:03Z"
updated: "2026-10-24T00:10:38Z"
---
# W2.1 — bbt-mux: supervisor, PTYs, lease protocol

Owner: Vesper Greywind <ORACLE>. Assignee: free Executor. Status: OPEN. Spec: [[mux-daemon]] (Ownership, Supervisor ↔ logic protocol, Restart), [[terminal-core]] D4 (portable-pty). Plan [[plan]] W2.1. v0.1 default: one process that speaks the supervisor protocol internally; the split is a later refactor.

Worktree `~/mydev/bboxy450-term`. Allowed: `crates/bbt-mux/**`, `Cargo.lock`, `.bboxy/runs/w2-1-*.log`.

1. `supervisor` module: spawn a seat via `portable-pty 0.9` (`CommandBuilder`, cwd, env incl. `BBOXY_SEAT_ID`), hold the master, one poller over raw fds (`as_raw_fd`, mio or polling crate) for all seats, resize, write with a single writer lease per seat, child exit tracking with process generation (ULID seat id, u64 generation).
2. Lease protocol over a unix socket in `$XDG_RUNTIME_DIR`-equivalent (`~/Library/Application Support/bboxy450-term/run/mux.sock`): `Attach{seat, generation}`, `Output{seat, generation, seq, bytes}`, `Input{seat, generation, input_id, bytes}`, `Resize`, `Exited`, length-prefixed frames, versioned. In-process channel implementation for v0.1 plus the socket framing so the split is mechanical.
3. Tests: spawn `/bin/zsh -f`, write `echo hi\n`, read back `hi`; resize propagates (`stty size`); kill the child, `Exited` carries the generation; two seats interleave without cross-talk; 50 seats spawn and echo under one poller.
4. Logs to `.bboxy/runs/w2-1-*.log`.

Verify: `cargo test -p bbt-mux --locked` green. Report: commit, module map, test list, what is not covered (no ring yet, no adapters).
