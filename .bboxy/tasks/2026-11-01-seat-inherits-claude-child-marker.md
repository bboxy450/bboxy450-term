---
assignee: seat:01M6D1P97MBKV7ZG21RYJQ3YSW
authorship:
  kind: seat
  process_generation: '1'
  public_key: J+Eete7RzpCHzcNSDpuEpx+ye6zu+UTGX+qDDQkPrw4=
  signer: seat:01M6CQ90BPKR0WZ70BDJD6R7MV
created: 2026-11-02T02:33:51.406Z
created_by: seat:01M6CQ90BPKR0WZ70BDJD6R7MV
delegated_by: seat:01M6CQ90BPKR0WZ70BDJD6R7MV
id: 01M6D1P7VRADV9GGJV42752W82
number: 2985
provenance:
  digest: sha256:51db3f318bf2761fcaf0928fa6a7cc883769f39e3b3ae731b26786bdff9bcd83
  frontmatter_digest: sha256:9cd4fde2783088e9b38eb4fadbc776795dd1c88f5fe615bf1daab9f18dcdc262
  log: .bboxy/log/2026-11-02.jsonl#12741
  sig: Q8L+j9o9HfpEbV/vKKDjGLosSUAoXRMveUDzEwET0gENlM2Pa19HJmHgGh0RZLz/jg9f48V64Lf8XCZ/69FsCw==
  signer: seat:01M6CQ90BPKR0WZ70BDJD6R7MV
role: executor
status: claimed
swarm: 01M649102FBS60QWNPZS5VAGF9
title: Claude seats inherit CLAUDE_CODE_CHILD_SESSION, so transcript saving is off
type: task
updated: 2026-11-02T02:33:56.199Z
updated_by: seat:01M6CQ90BPKR0WZ70BDJD6R7MV
verify: make test
---
# Seats inherit the parent Claude session's markers

ROLE: EXECUTOR. Work only in a new worktree `~/mydev/bboxy450-term-seat-env` on branch `seat-env`, created from `origin/main`. Work on your own: investigate, decide, fix and prove. Record each decision in your report. Message the Oracle only for a blocker that stops all work.

## Symptom (operator, 2026-11-01, two laptops on main)

Every Claude seat shows: `⚠ Transcript saving is off — inherited CLAUDE_CODE_CHILD_SESSION marker · restart with CLAUDE_CODE_FORCE_SESSION_PERSISTENCE=1 to keep future transcripts`. The seat's sessions are not saved, so `--resume` and history are lost.

## Evidence (Oracle)

- This seat (`01M6CQ90BPKR0WZ70BDJD6R7MV`, Claude Code 2.1.304, launched by bboxy) has in its environment `CLAUDE_CODE_CHILD_SESSION=1`, `CLAUDECODE=1`, `CLAUDE_PID=74330`, `CLAUDE_CODE_MESSAGING_SOCKET`, `CLAUDE_CODE_MESSAGING_TOKEN`, `CLAUDE_CODE_SESSION_ID`, `CLAUDE_CODE_ENTRYPOINT` and `CLAUDE_CODE_SESSION_ATTENDED`. Some of these are this process's own; find out which are inherited from the app, supervisor or launch path.
- The night launcher already strips caller harness markers through an environment allowlist: `crates/bboxy450-term/src/e2e_night.rs:727-750`, commits `688d88a6` and `1ceccce2`. The ordinary seat launch path evidently does not. Likely areas: `crates/bboxy450-harness-wake-up/src/launch.rs:790-910` (`env_remove`/`env_clear`), the supervisor spawn and the per-seat wiring kit.
- Probable trigger: the app or backend was started, installed or restarted from inside a Claude Code session (`make install`, `ctl app start`, or an Oracle seat), so the supervisor inherited the markers and passed them to every seat it launched. Confirm or refute this with evidence.

## Required outcome

1. Root cause as `file:line — mechanism`: which process first carries the marker and which launch step passes it to seats.
2. The fix: seat launch, for every harness profile (claude, codex, zsh, pi, omp), drops caller-harness markers and session-scoped values: `CLAUDECODE`, `CLAUDE_CODE_*` session/child/messaging/entrypoint markers, `CLAUDE_PID`, `CODEX_THREAD_ID`, and similar. Reuse the night allowlist or denylist mechanism instead of a second list, unless you record why it doesn't fit. Keep the variables seats need: PATH, HOME, user config, and the bboxy values set on purpose. Do not set `CLAUDE_CODE_FORCE_SESSION_PERSISTENCE` as a workaround.
3. Defence in depth: if the app's own process environment has these markers, strip them at the earliest owner (app or supervisor start) too, so detached backends don't keep them.
4. A real process test: start a supervisor or seat from a parent environment that has the markers, and assert the spawned seat's environment lacks them. Add a broken control that removes the scrub and must fail.
5. Formal (d365): if launch environment is modelled in `verification/contracts.toml`, update it. Otherwise record the contract (no caller-harness marker crosses seat launch) with its implementation witness.

## Allowed paths

`crates/bboxy450-harness-wake-up/**`, `crates/bbt-supervisor/**` (launch and environment only; #2187 is editing input dedup in `supervisor.rs`, so don't touch that region), `crates/bboxy450-term/**` (launch, cockpit and env only), `verification/**`, `.bboxy/reports/**`, and this exact task record copied byte-for-byte to `.bboxy/tasks/2026-11-01-seat-inherits-claude-child-marker.md` (the report gate needs it in-tree).

## Verification

`make test` on the exact committed source, exit 0, plus `make test-boundaries` (process boundary). Strict clippy on touched crates.

## Done

Signed commits on `seat-env`, a report in `.bboxy/reports/` with a `Report:` trailer, and `task_report`. Do not merge or push.
