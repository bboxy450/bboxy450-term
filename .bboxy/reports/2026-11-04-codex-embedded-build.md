---
type: report
title: Embedded-only ACP relay compile boundary
task: '[[01M6J13RYDS0C43JRNWR7BFN9H]]'
commits:
- 035e5b29e4ba02cbd2160fc48ad3ad3600d8c6e7
verify:
  command: make check-embedded-build
  exit_code: 0
author: seat:01M6HCYVGE7CJ8KK6PB20SPF3D
created: 2026-11-04T02:19:11.086Z
id: 01M6J5FMA8VMVWW8MGMWYHYAWC
updated: 2026-11-04T02:19:11.086Z
number: 19676
created_by: seat:01M6HCYVGE7CJ8KK6PB20SPF3D
updated_by: seat:01M6HCYVGE7CJ8KK6PB20SPF3D
authorship:
  signer: seat:01M6HCYVGE7CJ8KK6PB20SPF3D
  kind: seat
  public_key: YKv54MXoFa4rl83lF7g/hJHRLfs8fsSGu2+nb6F1XqY=
  process_generation: '1'
provenance:
  digest: sha256:ba041dc1fed72d43977b0aa4b3adc60b074f3e3e0fa8f11e73746078e866bb30
  frontmatter_digest: sha256:d43261eab71b86b4b3d3b757586a5c5f32a3b479829d7814e4a3895bc9d1938f
  signer: seat:01M6HCYVGE7CJ8KK6PB20SPF3D
  sig: s2qFly8lmxUXEo1IWQadwG+Tp1btfxPTXk71Rvnwok4ZqXiD0HAEDQT2xCZJnb9ZBEM8nTjBe0F0H5gGtgKvDw==
  log: .bboxy/log/2026-11-04.jsonl#109955
---
Completed the narrow embedded-build prerequisite in `/Users/feral/mydev/bboxy450-harness-land`, branch `codex-embedded-cfg-20261103`, based on published `a2312959f4ddd4519db13f2194c167705056e532`. Implementation commit `035e5b29e4ba02cbd2160fc48ad3ad3600d8c6e7` carries the approved Boxy signature. The rich-host tree stayed frozen and clean at `2d1642d320397c16a9bcbf1cb7c8629fb093ca1c`, preserved by `preserve/rich-host-2d1642d3`. No publication, installation, credentials or global settings were used.

`src/mcp.rs:61` now selects the existing private relay module only with `acp-agent`, matching its sole consumer `src/backend/acp_agent.rs` and the existing backend feature guard. No relay implementation, runtime, IR, context, permissions or dependency feature list changed; no warning suppression was added. `Makefile:32` adds `check-embedded-build`: strict embedded-only, no-default-features library clippy with `-D warnings`, default library compile, and ACP/standalone library plus actual binary compile. Each uses two Cargo jobs and the worktree target. `scripts/test.py:38` adds the target to the existing canonical stack without removing another check.

Actual regression: before adding the cfg guard, `RUSTC_WRAPPER= make check-embedded-build` exited **2** (cargo101), with exactly the five dead-code errors for Relay, start/remove, serve, line_value and next_line. Raw log `/tmp/codex-embedded-build-red.log`, SHA256 `d240d6d9af21e2831c65a6f654f4cf6d0dea68b37abb573734e9eca6169105ae`. On clean signed implementation035e5b29, the same command exited **0**; its three commands each exited0. Raw log `/tmp/codex-embedded-build-green.log`, SHA256 `28b8112c36da2099b40db710c75c8ec222da6dd3eff6ad87d56d451a9ebbdf3a`. `git diff --check` exited0. The compact command/source/model inventory is `.bboxy/runs/codex-embedded-build.json`. The final report-only commit does not alter these compilation inputs.

The authenticated task amendment `47d51ae59267685fcdeba6398799441a05b6509b21666b6f9eb75ff59f77c29f` explicitly replaces the earlier broad verification requirement with this focused target. Full make test, unit/runtime/GUI suites and Java/TLC/model corpus were **not rerun**, as directed. The earlier119-model/39-contract and launch-context/MCP process evidence in [[reports/2026-11-03-codex-agent-context]] is retained as **prior-source evidence**, not a fresh result for this cfg commit; its recorded formal receipt is `575ebd89cd354ac3393fc0c8350422db7eb448de90db8499217593eec7ee172a`. No stateful behavior changed, so no new model was introduced. Model files and relay implementation remain byte-identical to the published base. Compiles establish feature selection and type correctness; they do not newly establish runtime or arbitrary model correctness.

Oracle owns verification/publication of this signed successor. The term task resumes only on the Oracle-published pin; no private dependency pin is offered. The term final targeted delivery report remains separate and pending.