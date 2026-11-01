---
assignee: "seat:01M6AD98G0W194HGDQDAESG484"
authorship: {"kind": "seat", "process_generation": "1", "public_key": "dNWbVeHRHyt/TnV6QHFyDXtWIj+UElZjvaWtEgWhcFg=", "signer": "seat:01M6ABH71TVWGZ621NTD701PSH"}
created: "2026-11-01T01:58:44.127Z"
created_by: "seat:01M6ABH71TVWGZ621NTD701PSH"
delegated_by: "seat:01M6ABH71TVWGZ621NTD701PSH"
id: "01M6AD972977NZH976NK2GSD43"
number: 572
provenance: {"digest": "sha256:66ebff15d2ac457bc45bfeb359bd2a3b12b56d58541bb44a6af8270a7f4fbf13", "frontmatter_digest": "sha256:97961f17643b382639568f8cb5bcff08f77dc1fc0b11ab577faa9853ca5deb0a", "log": ".bboxy/log/2026-11-01.jsonl#321", "sig": "NEEmeCqI4vU2+Azpq+Fr2Rl9/ta9ipGtEyxY4ONCCs8rFrTJzPMB9d6wk216qTtu033GPOy39MHO/rouukEAAg==", "signer": "seat:01M6ABH71TVWGZ621NTD701PSH"}
role: "executor"
status: "claimed"
swarm: "01M649102FBS60QWNPZS5VAGF9"
title: "Plugins continue: land #31, ship the #84 transport contract, close #377's unproven checks"
type: "task"
updated: "2026-11-01T01:58:48.276Z"
updated_by: "seat:01M6ABH71TVWGZ621NTD701PSH"
verify: "make test-settings-plugins test-formal-contracts && make test"
---
# Plugins continue — close every remaining plugin-system gap

ROLE: EXECUTOR. Worktree `~/mydev/bboxy450-term-plugins`, branch `plugins-continue`, based on `0aa36d0a` (cx/plugfix: #31 G3 commit on top of origin/main 42320d93). Only this worktree. Never edit `~/mydev/bboxy450-term-cx-plugfix` (another live harness owns it).

Operator, 2026-10-31: plugin rework and its TLA+ models are on main; close the remaining gaps with one Executor. Handoff written by Oracle Nyx Copperwing from source evidence (the referenced `tasks/plugins-continue.md` was not on origin/main or on disk).

## On main already (do not redo)
Plugin SDK + trait/adapter/example per contract (5819c315), Rust fixtures replacing Python (98cdaf6b, quarantine rows cleared), hooks to plugins (2b9649d3), blocking vetoes (fafd83db), host API (5d21c97a), signed plugin messages (931857a5), measurements (1734a39d, 3868d17e), setup/login row (cca66339, f0f34c98, 4952b675), formal models PluginActivation/Hook/Connection/HostApi/HookGate/Setup in `verification/coverage.toml`.

## Gaps
1. **#31 G3 — land it.** 0aa36d0a adds slow-hook warn+skip (`crates/bbt-mux/src/plugins/hook_budget.rs`), `scripts/e2e/G3.sh`, `scripts/e2e/plugins.py`, model checks. Its log `.bboxy/runs/plugfix-31.log` says "full native/TLC parity and affected presubmit=pending". Run them; fix failures; land.
2. **#84 transport contract — not on main.** `crates/bbt-plugin/src/contracts.rs` has data_source, task_system, event_sink, session, repo_graph, decision_model, provider; no `transport`. Spec promises it: `.bboxy/specs/plugins.md:80` and [[launch-defaults]] §4. Acceptance (issue #84): contract schema + SDK trait/adapter/example (`bboxy450-term-transport-<name>`, declares spawn-parameter fields, `wrap(fields, inner command) -> argv`); Settings › Machines lists plugin transports beside built-ins ssh/mosh/et; launch composes `wrap()` argv; SDK-built fixture transport proves fields→argv with no network; malformed shape refused with a named reason; coverage/contract entries updated, model/negative control if launch arbitration changes. NOTE: cx/plugfix has *uncommitted* transport work (`crates/bbt-plugin/src/transport.rs`, `schemas/transport-1.0.schema.json`, SDK example). First run `git -C ~/mydev/bboxy450-term-cx-plugfix log --oneline -3 cx/plugfix`: if a transport commit has appeared, rebase onto it and finish from there; otherwise implement in your worktree. Do not copy their uncommitted files.
3. **#377 unproven checks** (`.bboxy/tasks/2026-10-30-plugin-sdk-rust.md:49-52`): Harmony read-only probe — list and search pages hold `limit` items within the 2 s budget; `status:active` and `status:running` searches return Harmony's active tasks; architecture gaps recorded with dispositions in `.bboxy/specs/plugins.md`; `make test` recorded on the final commit. Use fixture/private accounts only (Harmony variant of `bbt-plugin-fixture`); no real credentials. If a real-Harmony check truly needs an external account, record it as an explicit unproven gap; do not block on it.
4. **Sweep:** `grep -rn "unavailable\|not implemented\|partial" .bboxy/specs/plugins.md .bboxy/interfaces/plugin-host.md .bboxy/specs/e2e-script-map.md` for plugin rows (G3/G4 etc.) still marked partial/missing; close each or record the disposition.

## Verify
`make test-settings-plugins`, `make test-formal-contracts`, `make test-formal`, new transport tests, strict clippy on touched crates, `make test` on the final committed source. Land each cut with `sh scripts/land.sh` (gap 1 first, then 2, then 3) so progress reaches main early. Signed commits, exact-path staging.

## Communication (operator order: no token burn)
No acknowledgements, progress DMs or channel posts. One `task_report` at the end: per-gap commit SHA on main, files+mechanism, commands+exit codes, gaps left. Message the Oracle mid-task only for a blocker that stops all work, ≤3 lines.
