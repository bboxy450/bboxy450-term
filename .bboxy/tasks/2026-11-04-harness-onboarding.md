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
updated: 2026-11-04T17:41:41.694Z
created: 2026-11-04T17:37:53.334Z
number: 93377961361112
created_by: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
updated_by: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
authorship:
  signer: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
  kind: seat
  public_key: JlpXQZ4/Qy0yh5dcFR+iK7RgP5q83vzdNaLijybKm+Y=
  process_generation: '1'
assignee: seat:01M6KT5EV066KVVBAPR7RGJWAX
provenance:
  digest: sha256:3cd62fe9422b2a136cd308c5faf1179aea1da6299cf7fb5400f3d9db3803aef7
  frontmatter_digest: sha256:679bf36e34bc7e75ff58bf493ab5cb5aaad1b91de6b03b727938910a12221baf
  signer: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
  sig: CbtS11qNFuQB03dQjpjnKmoknmu4UiuA7r3Bn+7KlkIz2Ac+AwDqB6rue45Ia5DQEp3/IHC0o+Xi4edcQiwOAA==
  log: .bboxy/log/2026-11-04.jsonl#122707
---
ROLE: EXECUTOR.

Direct operator requests: automatically discover installed coding harnesses and provide editable on-disk ACP profiles; when a missing harness is installed later, discover it and create its missing profile without overwriting user edits. Explain/install missing ACP adapters through an attractive interactive welcome/setup screen, offering truthful OS/manager-specific choices (npm, Homebrew or mise where supported). New steering: render user-sent messages as Markdown too, including the gray queued tail, preserving causal gray-to-blue confirmation and old history.

Own parent worktree /Users/feral/mydev/bboxy450-term-onboarding, branch codex-harness-onboarding-20261104 at 5cdfebdb3e207024b22338c935bc4b44e977455d; nested producer branch of same name at a1ad9fbd796d6f847426d8d7c66cc325f4e7f955. Oracle created both and owns ALL fetch/pull/merge/pins/push/build/install. Work only here, commit producer and report separately; leave parent gitlink unstaged. This is a real bboxy450-harness Codex ACP dogfood seat. No recruitment, backend restart, real software installation, operator clipboard, credentials, OS desktop control or vendor inference tests.

First make a small signed checkpoint for Markdown user bubbles: reuse the existing shared Markdown/highlight renderer for durable and queued user messages. Keep bubble styling, selectable/copyable original text, attachment captions and one causal delivery transition. Test headings/lists/bold/link/inline code and fenced Python at narrow widths in light/dark; render bounds, prior history and gray queued state remain intact. Literal shell/code is never executed by rendering. Do not duplicate a Markdown engine.

Then implement live bounded harness/adapter discovery off the UI thread and a setup/install experience shared by standalone and embedded HarnessView. Distinguish native CLI installed, ACP adapter available, missing wrapper, unavailable and install in progress/error. Create missing profiles only when the installed CLI/verified ACP adapter makes the profile relevant; absent Grok must not silently acquire a generated profile. Discover additions at runtime and after installation without restarting sessions/backend. Preserve every user-owned/custom/edited profile, never rewrite/delete existing profiles, no credentials in generated records, atomic create-new writes for concurrent views. Avoid repeated filesystem/process work each frame. Match shared machine catalogue and launcher context already fixed; preserve agent/model/effort/MCP identity.

Verify adapter commands/install recipes against primary official sources. Useful official starting points: https://github.com/agentclientprotocol/registry and its documented https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json; https://github.com/agentclientprotocol/codex-acp ; https://github.com/agentclientprotocol/claude-agent-acp . Codex ACP package is @agentclientprotocol/codex-acp; Claude command is claude-agent-acp. Verify current Pi/Grok and other supported installed harnesses rather than inventing package names or ACP flags. Registry includes grok-build; inspect actual official contract before using. An unsupported adapter is honestly labeled with official instructions, never launched with a guessed flag. Do not download/execute arbitrary registry commands dynamically.

Welcome/setup cards use the existing bboxy portrait, layout toolkit and theme palette. Show useful status, exact offered command and official instructions, install action, progress, failures/retry and success/catalogue refresh. Prefer managers actually available and official supported recipes; npm global is a valid fallback. Do not fake brew formulae/mise support or bootstrap a package manager silently. Explicit user click admits a fixed-argv allowlisted install only, with bounded output/time/cancel and no shell interpolation or implicit sudo. Tests use PRIVATE temporary homes/PATH/fake managers exclusively: native present wrapper missing, absent/present later, edited profile byte preservation, concurrent discovery, unsupported agent, duplicate install suppression, failure/retry/cancel/stale completion and runtime refresh. Real host profiles/packages stay untouched by tests.

Add make verify-harness-onboarding as the single focused command: named private discovery/profile/installer/process/Markdown/native GUI checks + affected strict compile and license check. Cover only new ownership/lifecycle contract with smallest finite model+broken control/full native-TLC graph/verdict comparison if introduced; no full CI/parity/runtime/formal corpus, no unrelated rechecks. Respect disk/inference: two Cargo jobs, local target, compact source-bound evidence; do not build production bundles. Retain captures/logs outside Git and hashes/commands in .bboxy/runs/codex-harness-onboarding.json. Reuse existing source-bound evidence where unchanged.

Report indexed append-only reports/2026-11-04-harness-onboarding.md via knowledge tools, copy exact indexed bytes and this exact task mirror into own parent worktree, signed Boxy report commit with Report trailer, task_report with acceptance strings copied VERBATIM. Report producer/consumer hashes, real exits, official recipe sources, what was not tested. No broad staging/reset/stash or other worktree edits. Quiet execution, only one genuine all-work blocker with exact evidence/decision. Checkpoint Markdown first so Oracle can consume it promptly while you finish onboarding.

Canonical generated notice copies are also in scope: when changing root ATTRIBUTION.md, run make sync-licenses and retain its derived crates/**/ATTRIBUTION.md and plugins/**/ATTRIBUTION.md outputs. Do not independently edit those copies. Oracle observed this integration requirement in the attachment landing. Verification remains unchanged.
