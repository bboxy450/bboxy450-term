---
type: task
title: Preserve admitted prompts, MCP tools and launch semantics through ACP/exec harness backends
status: claimed
role: executor
worktree: /Users/feral/mydev/bboxy450-harness-pane-b12
allowed_paths:
- src/backend/acp_agent.rs
- src/backend/acp_agent/**
- src/gateway/host.rs
- src/gateway/host/**
- src/engine/mod.rs
- src/mcp.rs
- src/mcp/**
- src/provider/profile.rs
- src/frontends/headless.rs
- src/frontends/gui/mod.rs
- src/frontends/acp.rs
- tests/**
- scripts/test*.py
- scripts/formal.py
- verification/**
- Makefile
- Cargo.toml
- Cargo.lock
- ATTRIBUTION.md
- .bboxy/reports/**
- .bboxy/runs/codex-launch-parity.json
verify: RUSTC_WRAPPER= make test
parent: 01M6HCH1SJHDSB6SDAC2ZF1BXX
acceptance:
- Downstream process receives admitted launch prompt/context and can call scoped MCP tools across ACP/exec fixtures
- Precedence, flags semantics, cwd and session/generation lifecycle retain context without cross-seat transfer
- Native/TLC ownership/failure model parity and causal broken controls pass
- Clean canonical make test and current-source formal evidence pass; signed report and exact logs retained
id: 01M6HRD5XR91MD7MH557R77N5Y
delegated_by: seat:01M6H97SAX3GN1BYB4YG76P3P0
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-03T22:28:15.840Z
created: 2026-11-03T22:27:50.773Z
number: 19209
created_by: seat:01M6H97SAX3GN1BYB4YG76P3P0
updated_by: seat:01M6H97SAX3GN1BYB4YG76P3P0
authorship:
  signer: seat:01M6H97SAX3GN1BYB4YG76P3P0
  kind: seat
  public_key: B3hFSjBIE1Nnj7mw9Eg9pqyRpYHF/GQaKXPyV1KAod0=
  process_generation: '1'
assignee: seat:01M6HRD7KZD5Q336S88VRS5HJX
provenance:
  digest: sha256:6cf66e5ddd082e4f3cd6c7ce440f602ef737f9932b23d87add2b558160ed27fa
  frontmatter_digest: sha256:219e9ff369ca78881fb50b3d0f7a22d6610efc64855a3a7b3cba83d8d82cc9c1
  signer: seat:01M6H97SAX3GN1BYB4YG76P3P0
  sig: tj0mrO7Opt36aq20o9FyKUVM4RoFdx5ruxSx8OKiNMoJp8YrenipgLIbyAlLaE//oySDPxwbR4od4FDw3mt9Ag==
  log: .bboxy/log/2026-11-03.part-86213.jsonl#106716
---
ROLE: EXECUTOR. Implement launch-context parity in /Users/feral/mydev/bboxy450-harness-pane-b12, branch codex-launch-parity-20261103, base published2fb78a017992f73468b6fa9fe5edbc48ba27f05c. The prior socket task is accepted and its seat exited; preserve pane/b1-b2, all original refs/source/reports/formal receipts. Reuse the existing warm worktree target to limit disk use. Read project instructions, Make help and relevant source/ICDs before edits. Exact allowed_paths only, exact staging, approved Boxy signed commits, attribution. No recruiting, push/merge/publication/trusted install, actual Claude launch, paid inference, credentials, global configs, desktop control or other worktrees.

Direct operator requirement: standalone native GUI and the term embedded GUI must render the same IR using shared HarnessView; every supported spawn route, exec or ACP, receives the same intended role/name/task/workspace prompts, MCP/tools, model/approval settings and session information as direct/native launches. Side A term host task [[tasks/2026-11-03-codex-rich-host]] owns launch plan/default rich pane and the real App/restart/visual comparison. You own the discovered Side B context-loss boundary, not visual polish.

Observed source defects: src/gateway/host.rs actor() calls AcpAgentActor::load without self.launch/ACP overrides; src/backend/acp_agent.rs load() lacks LaunchOptions, session/new sends mcpServers:[], and send_prompt() only carries history+user blocks. Thus launch --append-system-prompt/--mcp-config work for model backend but are dropped by ACP child. Correct this at a shared boundary, preserving existing agent-owned tools and lifecycle rather than replacing them with a second loop.

Binding interface amendment for this assignment (both project sides coordinated by Oracle): context is immutable and scoped to the admitted launch/session generation. Trusted launcher role/name/workspace instructions and admitted MCP config must actually reach the downstream ACP agent before it executes work, including reopen/fork/profile change. Preserve profile/ACP/launcher precedence and origin labels. Use supported provider-specific options for equivalent intended model/approval behavior; incompatible vendor flags need semantic mapping, not literal copying. If ACP has no system-role field, document the exact transport/semantic limitation and use a deliberate supported bootstrap/context path that preserves the supplied instructions; never silently omit them or call a user-text assertion proof of system-role priority. No new synthetic user turn, fake progress, changed IR durability or extra inference request. Existing actor queue/cancel/agent-initiated/quiet/status behavior remains intact.

MCP must be callable, not merely listed. Relay only this seat's admitted servers and grants; preserve launch origin, file capability custody, redaction and environment scrubbing for non-launch servers. Never put capability values or credentials into argv, JSONL, reports, traces or model content; no cross-seat config copying or global ~/.codex/~/.claude/profile edits. Preserve failure notices/continued-turn behavior for invalid unavailable configs; no hanging delivery on slow/unavailable server. Preserve existing external-agent tool permissions and deliberately supported flags. No arbitrary expansion of role/grants.

Acceptance via unattended process fixtures (no live provider requests):
1. Use actual Host/create/open/switch and actual downstream child JSON-RPC/launch argv evidence to prove nonempty correct launcher context and MCP config arrive for codex-exec/codex-acp and claude-* compatibility fixtures. Child calls a throwaway MCP tool and receives the expected result under the exact private seat identity; an empty mcpServers or missing prompt control must fail its named assertion. Fixture server/process may use a fresh throwaway test capability, never read a live seat capability.
2. Cover mixed profile+ACP overrides+launcher precedence, model/approval setting semantics, args quoting/cwd, invalid or unavailable MCP, absent context, restart/reopen, profile A→B→A and generation change. No stale context/grant transfer, no duplicate turn/tool effect or added synthetic prompt. Preserve lifecycle/cancel/quiet Native/Usage/IR1.2 and all existing tests.
3. Update finite state contract/model/source mapping and deliberately broken controls for context ownership/forwarding, bounded startup/failure and session changes; native Rust and pinned Java TLC graph+verdict parity. Keep the existing112 models/38 contracts and all controls, adding the new coverage without omission.
4. Canonical RUSTC_WRAPPER= make test exit0 on clean committed inputs, fresh clean source-bound formal receipt/check-formal-evidence, licenses/secrets. Use Make, two Cargo jobs, worktree-local target, one full machine gate at a time with visible self-service claim; no operator input. Do focused red/green once, then one final canonical gate; avoid repeated full CI or optional refactors. Raw logs/private fixtures outside Git; compact hashes/exit summary in .bboxy/runs/codex-launch-parity.json only.

Coordinate the boundary with Side A via the existing task; send a message only for a true all-work blocker. A single final append-only report through knowledge tools, committed with Report trailer, then task_report. Include actual changed files/lines, command exits/log hashes, red→green controls, exact semantic limitations, final signed SHA/clean state, and per-acceptance evidence. Oracle will verify, publish the harness, pass its pin to host, and install; you do not publish.