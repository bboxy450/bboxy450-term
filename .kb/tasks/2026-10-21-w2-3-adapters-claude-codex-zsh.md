# W2.3 — bbt-mux: harness adapters for Claude Code, Codex, zsh

Owner: Vesper Greywind <ORACLE>. Assignee: free Executor, after W2.1. Status: OPEN. Spec: [[harness-adapters]] (trait §1, per-harness §2, state machine §3, shim §4, PTY fallback §5), [[subagent-detection]] (OSC 7501 wire evidence: BEL-terminated, both accepted), decisions S3–S11 in [[README]] of specs.

Allowed: `crates/bbt-mux/**`, `Cargo.lock`, `.kb/runs/w2-3-*.log`. Fixtures to use: `.kb/runs/osc7501-claude.bin`, `lab/web/logs/*.txt`.

1. Harness record type (open `kind` string; name, command, tile, icon, default role, wake method, state source, hooks) and the `Adapter` trait from §1 instantiated from a record. Built-ins as records with code behind them.
2. Claude adapter: launch with `--append-system-prompt` for the role and a per-launch `--settings` overlay (never the user's files) that binds the shim entry point `bboxy450-term hook` by absolute path; answer the OSC 7501 probe with BEL; parse 7501 reports from `Term::on_unknown_osc` into seat state and nested subagent rows (running only); read `~/.claude/sessions/<pid>.json` for idle/busy/waiting as the positive readiness signal; wake via the messaging socket frames with priority; `seat_interrupt` maps to `now`.
3. Codex adapter: `-c developer_instructions=` for the role; 7501 main state; app-server `thread/status/changed` and `turn/*` events for readiness; wake via `thread/queue/add` or `codex queue`; nested rows from hooks only.
4. zsh adapter: per-seat `ZDOTDIR` overlay emitting OSC 133 and OSC 7; OSC 133 B is the readiness signal; typed input behind the input guard.
5. Hook shim entry point: reads hook JSON on stdin, writes a ≤4 KiB OSC 7502 frame to `/dev/tty` with `raw` fields verbatim, falls back to the unix socket with the seat capability token for larger payloads.
6. Tests: replay `osc7501-claude.bin` through a `Term` and assert the state sequence working→clear→done with the child id and title; the sessions-file reader on a fixture; the zsh adapter on a real `/bin/zsh` seat detects the prompt via 133 B; the input guard refuses to type while a Claude fixture reports a pending dialog.

Verify: `cargo test -p bbt-mux --locked adapters` green. Report as usual; name every fact taken from the spec that you could not exercise live.
