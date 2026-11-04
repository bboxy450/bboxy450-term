---
artifacts:
- '[[01M6BSKVB5RN2YPQ3YPQ5951ZG]]'
assignee: seat:01M6AD98G0W194HGDQDAESG484
authorship:
  kind: seat
  process_generation: '1'
  public_key: dNWbVeHRHyt/TnV6QHFyDXtWIj+UElZjvaWtEgWhcFg=
  signer: seat:01M6ABH71TVWGZ621NTD701PSH
created: 2026-11-01T01:58:44.127Z
created_by: seat:01M6ABH71TVWGZ621NTD701PSH
delegated_by: seat:01M6ABH71TVWGZ621NTD701PSH
id: 01M6AD972977NZH976NK2GSD43
number: 572
provenance:
  digest: sha256:6e7fb913201c51602fd5a30db360bccbb79acf763793fb3b858c552636e73cbb
  frontmatter_digest: sha256:7c29c93a8083f7c27d42011155bf8d4f099d86b765f61c63e8f686ad7138a5a4
  log: .bboxy/log/2026-11-01.jsonl#9536
  sig: HICyqz5Bgfa6aL/nfU5p5d2kusgPT9KGy64BnHUcS9EmiD3mYpqCunvxWDu3aZjMerAMAK2mF5r5GLly/rSNBQ==
  signer: seat:01M6ABH71TVWGZ621NTD701PSH
report:
  commit: b9015a4006fb2c408c3d39c4ef3102846aa6a600
  op_id: 01M6BSNZRK68GSAF3FVKS1DW54
  reported_by: seat:01M6AD98G0W194HGDQDAESG484
  summary: 'Report: [[reports/2026-11-01-plugins-continue-verification]], superseding [[reports/2026-11-01-plugins-continue]]. On main: #31 G3 553f9325; #84 transport base 4fa1b876 plus bounded-admission completion d579986c; #377 Harmony/sweep 8ce271ca plus fixture verification b9015a40. Signed, clean; exact-HEAD make test and author landing both exit 0, with 134 native/TLC models, 31 crates, 57 contracts, headless smoke and 27 invariants (11 slow checks skipped). The declared verification stack was executed as separate component Make invocations; logs and actual exits are in the reports. Remaining unproven: real-Harmony backend/account, P9b busy retry, configured preview, full native G4 identity evidence and G5 theme night. Live schema has no optional report field, so this summary carries the qualified Report link.'
role: executor
status: accepted
swarm: 01M649102FBS60QWNPZS5VAGF9
title: 'Plugins continue: land #31, ship the #84 transport contract, close #377''s unproven checks'
type: task
updated: 2026-11-01T14:54:55.340Z
updated_by: seat:01M6ABH71TVWGZ621NTD701PSH
verify: make test-settings-plugins test-formal-contracts && make test
verify_result:
  command: make test-settings-plugins test-formal-contracts && make test
  evidence:
  - criterion: '#31 G3'
    reference: Main 553f9325; make test-settings-plugins exit 0 in target/plugins-harmony-final-g3.log, custom and native full G3; missing-warning negative rejected.
  - criterion: '#84 transport contract, SDK, Settings, argv and refusals'
    reference: Main 4fa1b876 and d579986c; crates/bbt-plugin/src/transport.rs:13; crates/bbt-plugin-sdk/src/contracts.rs:242; crates/bbt-mux/tests/plugin_transport.rs:38; make test-plugin-transports exit 0 in target/plugins-transport-focused-retry.log.
  - criterion: '#377 private Harmony read-only paging and status'
    reference: Main 8ce271ca; crates/bbt-plugin-fixture/tests/harmony_pages.rs:83 and :166; red baseline Make 2, fixed make test-harmony-pages 0; 72 Harmony RPCs <2 s, exact inventories and unchanged backing bytes; ordinary status controls passed. Real backend remains unproven.
  - criterion: Architecture and partial/missing row dispositions
    reference: .bboxy/specs/plugins.md:173; .bboxy/specs/e2e-script-map.md:580 and :594; explicit remaining P9b, preview, G4, G5, real-backend and G8/#505 limits.
  - criterion: Verification stack and final committed source
    reference: 'Components ran separately: make test-settings-plugins 0 (target/plugins-harmony-final-g3.log); make test-formal-contracts within make test-plugin-fixtures test-formal-contracts 0 (target/plugins-harmony-focused.log); exact signed b9015a40 make test 0 (target/plugins-harmony-final-ci-b9015a40.log, SHA256 f1be4be5f6ced15e8e9995a6127e8360714a8b0f20d94b3b4ae5df920df4e548).'
  - criterion: Formal, process, parity and strict checks
    reference: 'Clean current-source formal receipt 730ecf0705d7c57b500741e03516351cc43f51e38f5590a2dec43bb4c6029b3f: 134 models/31 crates/57 contracts, native/TLC parity and broken controls. Boundary/Settings/Launcher/landing fixtures exit 0 in target/plugins-transport-verification-retry.log; 48 Settings advisory pixel reviews remain. Strict workspace and fixture/ingestion lint exit 0.'
  - criterion: Failures retained and correction verified
    reference: First final-source 8ce271ca make test exited 2 on two 967-seat ring sweep deadlines. Test-only idle yield retains 967 seats, 15-second bound and all assertions; make test-seat-ingestion check-seat-ingestion 0 (9 ring cases, 1.82 s); both cases passed final full CI. No quarantine or widened bound.
  - criterion: Landing, signatures, per-commit compilation and Report
    reference: 'sh scripts/land.sh --gate-log target/ci-local.log exit 0; actual remote refs/heads/main=b9015a4006fb2c408c3d39c4ef3102846aa6a600; clean worktree; signatures G; both final candidates cargo check --workspace --all-targets --locked exit 0. Report: [[reports/2026-11-01-plugins-continue-verification]], with signed trailer and supersedes chain.'
  exit_code: 0
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


## Report from seat:01M6AD98G0W194HGDQDAESG484

Report: [[reports/2026-11-01-plugins-continue-verification]], superseding [[reports/2026-11-01-plugins-continue]]. On main: #31 G3 553f9325; #84 transport base 4fa1b876 plus bounded-admission completion d579986c; #377 Harmony/sweep 8ce271ca plus fixture verification b9015a40. Signed, clean; exact-HEAD make test and author landing both exit 0, with 134 native/TLC models, 31 crates, 57 contracts, headless smoke and 27 invariants (11 slow checks skipped). The declared verification stack was executed as separate component Make invocations; logs and actual exits are in the reports. Remaining unproven: real-Harmony backend/account, P9b busy retry, configured preview, full native G4 identity evidence and G5 theme night. Live schema has no optional report field, so this summary carries the qualified Report link.

```json
{
  "commit": "b9015a4006fb2c408c3d39c4ef3102846aa6a600",
  "op_id": "01M6BSNZRK68GSAF3FVKS1DW54",
  "verify_result": {
    "command": "make test-settings-plugins test-formal-contracts && make test",
    "evidence": [
      {
        "criterion": "#31 G3",
        "reference": "Main 553f9325; make test-settings-plugins exit 0 in target/plugins-harmony-final-g3.log, custom and native full G3; missing-warning negative rejected."
      },
      {
        "criterion": "#84 transport contract, SDK, Settings, argv and refusals",
        "reference": "Main 4fa1b876 and d579986c; crates/bbt-plugin/src/transport.rs:13; crates/bbt-plugin-sdk/src/contracts.rs:242; crates/bbt-mux/tests/plugin_transport.rs:38; make test-plugin-transports exit 0 in target/plugins-transport-focused-retry.log."
      },
      {
        "criterion": "#377 private Harmony read-only paging and status",
        "reference": "Main 8ce271ca; crates/bbt-plugin-fixture/tests/harmony_pages.rs:83 and :166; red baseline Make 2, fixed make test-harmony-pages 0; 72 Harmony RPCs <2 s, exact inventories and unchanged backing bytes; ordinary status controls passed. Real backend remains unproven."
      },
      {
        "criterion": "Architecture and partial/missing row dispositions",
        "reference": ".bboxy/specs/plugins.md:173; .bboxy/specs/e2e-script-map.md:580 and :594; explicit remaining P9b, preview, G4, G5, real-backend and G8/#505 limits."
      },
      {
        "criterion": "Verification stack and final committed source",
        "reference": "Components ran separately: make test-settings-plugins 0 (target/plugins-harmony-final-g3.log); make test-formal-contracts within make test-plugin-fixtures test-formal-contracts 0 (target/plugins-harmony-focused.log); exact signed b9015a40 make test 0 (target/plugins-harmony-final-ci-b9015a40.log, SHA256 f1be4be5f6ced15e8e9995a6127e8360714a8b0f20d94b3b4ae5df920df4e548)."
      },
      {
        "criterion": "Formal, process, parity and strict checks",
        "reference": "Clean current-source formal receipt 730ecf0705d7c57b500741e03516351cc43f51e38f5590a2dec43bb4c6029b3f: 134 models/31 crates/57 contracts, native/TLC parity and broken controls. Boundary/Settings/Launcher/landing fixtures exit 0 in target/plugins-transport-verification-retry.log; 48 Settings advisory pixel reviews remain. Strict workspace and fixture/ingestion lint exit 0."
      },
      {
        "criterion": "Failures retained and correction verified",
        "reference": "First final-source 8ce271ca make test exited 2 on two 967-seat ring sweep deadlines. Test-only idle yield retains 967 seats, 15-second bound and all assertions; make test-seat-ingestion check-seat-ingestion 0 (9 ring cases, 1.82 s); both cases passed final full CI. No quarantine or widened bound."
      },
      {
        "criterion": "Landing, signatures, per-commit compilation and Report",
        "reference": "sh scripts/land.sh --gate-log target/ci-local.log exit 0; actual remote refs/heads/main=b9015a4006fb2c408c3d39c4ef3102846aa6a600; clean worktree; signatures G; both final candidates cargo check --workspace --all-targets --locked exit 0. Report: [[reports/2026-11-01-plugins-continue-verification]], with signed trailer and supersedes chain."
      }
    ],
    "exit_code": 0
  }
}
```
