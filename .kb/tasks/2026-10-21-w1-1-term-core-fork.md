# W1.1 — term-core: fork alacritty_terminal + vte into bbt-core

Owner: Vesper Greywind <ORACLE>. Assignee: first free Executor. Status: OPEN. Spec: [[terminal-core]] D1, D2. Plan: [[plan]] W1.

Worktree `~/mydev/bboxy450-term`. Allowed: `crates/bbt-core/**`, root `Cargo.lock`, `.kb/runs/w1-1-*.log`. Nothing else.

1. Clone alacritty at HEAD (record the commit) into `.kb/runs/clones/` (gitignored). Vendor `alacritty_terminal/src/{term,grid,index.rs,cell.rs,event.rs,vte_handler…}` and the `vte` crate's `ansi` parser (pinned version noted in terminal-core D2) into `crates/bbt-core/src/`, keeping upstream file layout under `src/vendor/` with a `VENDOR.md` naming source commits and licenses (Apache-2.0 / MIT).
2. Cut: tty, event_loop, vi_mode, search, selection, and their tests. Keep the grid, term, parser and their unit tests.
3. Public API: `Term::new(cols, rows, history_size)`, `Term::advance(&[u8])`, `Term::grid()`, `Term::viewport_only()` (history 0), `Term::checkpoint() -> Checkpoint`, `Term::restore(Checkpoint)`, `Term::on_unknown_osc(Fn(&[&[u8]]))` pass-through hook (7501/7502 land here).
4. Verify: `cargo test -p bbt-core` green; a new test feeds `ESC[31m` + 4096 bytes, checkpoints, restores, asserts red foreground survives (Nyx's probe); a new test proves OSC 7501 reaches the hook with its raw params.

Report on `#bboxy450-term`: commit, upstream commit, lines kept/cut/added, test counts, what is not covered.
