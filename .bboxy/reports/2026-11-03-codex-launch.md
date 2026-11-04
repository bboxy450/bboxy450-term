---
id: 34A2M3CBSY9RQVGWRMHJVG4GT5
number: 5002228
type: report
title: Unified Codex launch and required bboxy MCP startup
author: "seat:01M6HCFJ8QFC193R49JW3G0VQX"
task: "[[tasks/2026-10-21-w2-3-adapters-claude-codex-zsh]]"
commits: []
created: "2026-11-03T19:12:18Z"
verify:
  command: "cargo nextest run --locked -p bboxy450-term-mux --test mcp -E 'test(codex_launch_doors)'"
  exit_code: 0
---
# Unified Codex launch and required bboxy MCP startup

Direct operator follow-up, 2026-11-03: unify ctl, MCP and UI launch; land the fix
quickly. Worktree `/Users/feral/mydev/bboxy450-term-codex-mcp`, branch
`fix/codex-mcp-bootstrap`, base `1affb986`.

This external Codex conversation is `01a1a2c7-c917-7b02-91e0-899707006efd`. The report author is that
conversation's same 128-bit identifier encoded as a ULID for the record schema;
it does not claim an authenticated bboxy seat or delegated assignment. No native
task status was changed. The bboxy task_report tool is absent from this session.

- `crates/bbt-mux/src/logic.rs:5091` — MCP converts its authorized request to the
  same SpawnSpec and prepare_spawn helper as UI and operator ctl. The existing
  deterministic MCP seat ID, separate authority checks and operation journals
  remain. Plugin transports use the same preparation helper.
- `crates/bbt-mux/src/logic.rs:9684` — shared preparation resolves role, local
  canonical cwd, resume identity, placement and launch inputs. Explicit UI
  command/argument selections remain inputs; no global settings were changed.
- `crates/bboxy450-harness-wake-up/src/launch.rs:637` — the per-launch bboxy server
  is enabled and required, including the shared-daemon thread configuration.
  Required connection failure prevents an apparently usable thread with no tools.
- `crates/bbt-mux/tests/mcp/launch.rs:15` — eight fresh/resume launches through UI,
  ctl, MCP and the MCP ctl alias; replay creates no extra seats. Captured plans
  carry the proxy, hooks, correct native resume and required MCP settings.
  Each actual admission file authenticates tools/list and orient with the exact
  seat identity. The catalog contains orient, skill_list, skill_read, task_report
  and message_send.
- `crates/bboxy450-term/tests/formal_contracts/launch_readiness.rs:14` — bounded
  three-door/two-generation startup contract and optional-startup broken control;
  mapping and abstraction limits are in verification/contracts.toml.

Validation completed before the landing gate:

- Targeted nextest command above — exit 0, one test passed (eight launches).
- Direct installed Codex 0.165.0 app-server, private temporary HOME/CODEX_HOME,
  no turn or model request, MCP command `/usr/bin/false`: optional server allowed
  thread/start; required server returned required-MCP-initialization failure and
  created no thread. Both owned server processes were terminated and reaped.
- Native Codex mcp list accepted enabled=true and required=true — exit 0.
  An initial --strict-config probe exited 1 because that switch is unsupported
  for the mcp command; it was removed for the valid probe.
- Initial targeted compilation exited 101 for the renamed helper's plugin
  transport caller; that caller was updated and the same targeted test passed.
- `git diff --check` — exit 0.

The signed commit containing this report is the candidate. make land supplies
its exact-source Rust, corpus and formal gates before pushing; its result and
commit are reported separately after completion. No desktop input, operator
login, global Codex config changes or live-seat restarts were used. No signed
installation, release or wrapper-runtime repair is part of this fix.
