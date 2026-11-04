---
type: task
title: Discover installed ACP harnesses, offer interactive setup, and render user Markdown
role: executor
parent: '[[01M6KEVVYNZW5H21RPTFW1RVDR]]'
worktree: /Users/feral/mydev/bboxy450-term-onboarding
allowed_paths:
- crates/bboxy450-harness/src/provider/**
- crates/bboxy450-harness/src/frontends/**
- crates/bboxy450-harness/src/lib.rs
- crates/bboxy450-harness/src/main.rs
- crates/bboxy450-harness/src/onboarding.rs
- crates/bboxy450-harness/src/onboarding/**
- crates/bboxy450-harness/tests/**
- crates/bboxy450-harness/verification/**
- crates/bboxy450-harness/scripts/verify-harness-onboarding.py
- crates/bboxy450-harness/Makefile
- crates/bboxy450-harness/ATTRIBUTION.md
- Makefile
- scripts/verify-harness-onboarding.py
- ATTRIBUTION.md
- .bboxy/tasks/2026-11-04-harness-onboarding.md
- .bboxy/reports/**
- .bboxy/runs/codex-harness-onboarding.json
- crates/**/ATTRIBUTION.md
- plugins/**/ATTRIBUTION.md
- crates/bboxy450-term-desktop-core/src/lib.rs
- crates/bboxy450-term-desktop-core/tests/app_menu.rs
verify: make verify-harness-onboarding
acceptance:
- Installed supported harnesses receive editable ACP profiles automatically, later installations are detected, and existing profile bytes are preserved
- Welcome setup distinguishes missing CLI and adapter and offers verified platform-appropriate install choices with responsive progress and honest failure recovery
- Installation runs only after an explicit UI action through fixed supported commands and private unattended tests prove ownership, cancellation and duplicate guards
- Durable and queued user messages render Markdown with bounded themed layout while prior history, attachment content and causal delivery state remain intact
- Focused private discovery, profile, installer, Markdown and native GUI checks plus affected strict compilation and license verification pass without backend restart or operator input
id: 01M6KT6ZT468RVV2SZ4ANS42KX
status: claimed
delegated_by: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-04T19:11:45.234Z
created: 2026-11-04T17:37:53.334Z
number: 93377961361112
created_by: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
updated_by: seat:01M6KYK93CXV3YMNFVM5WJ3Y3V
authorship:
  signer: seat:01M6KYK93CXV3YMNFVM5WJ3Y3V
  kind: seat
  public_key: L35fG+pdud/b1f8S+WvaBsLGjjdfhVfEOjnvBFsdBc4=
  process_generation: '1'
assignee: seat:01M6KYK93CXV3YMNFVM5WJ3Y3V
artifacts:
- '[[01M6KX8FDQ7X9VZV4YS2BSQ8X3]]'
- '[[01M6KZJVFW9EPZ44F2HK11FNMV]]'
provenance:
  digest: sha256:2946b42b6ad80bddc0eff7afe796f2d9247a9a383f37f9e27070ed2d0a9343cf
  frontmatter_digest: sha256:2cf2f915f7b5a9c5c5e1facf8252fc163bd574a74cb187860e6aa0253801e41c
  signer: seat:01M6KYK93CXV3YMNFVM5WJ3Y3V
  sig: iwPFZJucF9jKp15z7Sbp5lKo2RnYVTQ8n+N8fRTzxeU5Sa9+pq+JY0ViC5QTs9ZHye9f8BTEURiEtrtfOfdyAQ==
  log: .bboxy/log/2026-11-04.jsonl#129923
---
ROLE: EXECUTOR.

Current operator priority: restore the centered bboxy portrait + Loading screen immediately. The installed profile-discovery UI currently wins before an existing session loads; this is a regression. Then compact/center welcome setup and fix menu naming. The Oracle is rotating the old context; all commits/reports and two parent working-file changes are preserved.

Own worktree /Users/feral/mydev/bboxy450-term-onboarding. Parent branch codex-harness-onboarding-20261104 at5a8d6503a4f65848fdbee2deb0ad650bd04a6d06; producer branch same name at222d64c550574813035d6f29dc1edcd52f343d30 (real empty-home startup follow-up). Read current Git state: parent scripts/verify-harness-onboarding.py and .bboxy/runs/codex-harness-onboarding.json have preserved legitimate changes. Do not reset/stash/discard them. Producer3f151e4 and first report5a8 are already published and installed. Existing profile discovery, private installer, Markdown messages, source views, 10 focused cases and 2 finite model/control graphs are green; reuse them, do not reimplement them.

1. FIRST make the smallest loading fix and signed source checkpoint promptly. In src/frontends/gui/view.rs::transcript, automatic setup currently executes BEFORE state.ready/real session loading and before testing drawn history. Restore the existing face::loading_session portrait/Loading component during load, replay or preparation. Eager profile discovery, empty/deferred catalogue, missing profile metadata or empty turns map must never replace loading or existing out-of-turn history. Automatic setup is only for a settled actual new/unconfigured welcome; explicit setup remains usable. Preserve old history and gray queued tail while preparing. Add only the needed named loader/welcome routing check (including ready=false with onboarding already discovered, deferred metadata/catalogue, drawn history without turns, actual settled welcome). Run that one focused Make check and COMMIT THE LOADING FIX before doing the rest or waiting for report indexing. Oracle watches your signed checkpoint and handles the build/install.

2. Welcome/setup should be compact and centered: a constrained centered panel, centered existing portrait/title, compact agent rows with concise status and clear Use/Install action. Repetitive PATH/auth text, long instructions and manager commands belong behind a working details control. Keep progress/error/cancel and truthful manager choices, light/dark, narrow widths, and separate setup/conversation scroll. Reuse existing toolkit and icons. Native captures at320/800 should show the centered panel and useful first rows, not a page of full-width paragraphs. No new visual system.

3. Cosmetic menu fix: crates/bboxy450-term-desktop-core/src/lib.rs around176 hard-codes Hide/Quit bboxy450-term even in the standalone bboxy450-harness app. Correct shared default runtime app naming for both apps, with a clean fallback. Preserve actions, shortcuts and shutdown/session semantics. Avoid a new producer dependency on an unpublished parent API. Reuse existing menu checks and affected compilation; no full suite or test solely mirroring a cosmetic string.

4. Complete existing supported-agent coverage for Goose: official registry entry/documented ACP command, generated editable profile only when relevant, private present-later/command assertion. Preserve honest unsupported OMP/Pi capability labels. Never overwrite existing profiles/default/model/auth data.

Oracle owns ALL fetch/pull/rebase/merge/pins/push/bundle/install. One writing executor in this checkout; no recruitment, other worktrees, broad staging/reset/stash, real package installs, credentials, desktop clicks/capture or backend actions. Signed Boxy commits. Stage exact paths; producer and parent changes commit separately, parent gitlink stays unstaged. Source checkpoints must not wait on MCP report bookkeeping.

Final command remains make verify-harness-onboarding with only needed named routing/layout/menu/discovery checks and affected compile/license checks. Reuse unchanged install/runtime/formal evidence where inputs are unchanged; no full CI/parity/runtime/formal corpus or paid vendor tests. Native/offscreen captures and logs stay outside Git, compact command/exit/source/capture hashes in .bboxy/runs/codex-harness-onboarding.json. Existing report reports/2026-11-04-harness-onboarding.md is append-only: register a superseding report for the final fixes, commit exact indexed report/task mirrors, then fresh task_report with acceptance strings copied verbatim. Unknown write: keep original operation ID, look it up, never redispatch an unknown effect. Source checkpoint first.

Quiet execution: no acknowledgements/plans/progress DMs. One real all-work blocker with exact decision/evidence, and final task_report. This task and signed delegation are execution permission; old peer messages are historical feedback.
