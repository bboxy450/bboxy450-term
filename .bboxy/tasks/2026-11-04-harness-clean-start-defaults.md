---
type: task
title: Unify profile identity and make empty-home Codex startup work
role: executor
worktree: /Users/feral/mydev/bboxy450-term-onboarding
allowed_paths:
- crates/bboxy450-harness/src/onboarding.rs
- crates/bboxy450-harness/src/provider/profile.rs
- crates/bboxy450-harness/src/frontends/headless.rs
- crates/bboxy450-harness/src/frontends/gui/options.rs
- crates/bboxy450-harness/src/frontends/gui/window.rs
- crates/bboxy450-harness/src/frontends/gui/bridge.rs
- crates/bboxy450-harness/src/lib.rs
- crates/bboxy450-harness/src/main.rs
- crates/bboxy450-harness/tests/**
- crates/bboxy450-harness/verification/**
- crates/bboxy450-harness/scripts/verify-clean-start.py
- crates/bboxy450-harness/Makefile
- crates/bboxy450-harness/ATTRIBUTION.md
- Makefile
- scripts/verify-clean-start.py
- ATTRIBUTION.md
- crates/**/ATTRIBUTION.md
- plugins/**/ATTRIBUTION.md
- .bboxy/tasks/2026-11-04-harness-clean-start-defaults.md
- .bboxy/reports/**
- .bboxy/runs/codex-clean-start.json
- crates/bboxy450-harness/src/backend/acp_agent.rs
verify: make verify-clean-start
acceptance:
- Canonical Codex profile identity is codex, its generated file is codex.md and its ACP executable is codex-acp without duplicate profile generation
- A private empty home with fake installed Codex/ACP tools starts through normal GUI and CLI or seat entrypoints without relying on legacy nemotron or pre-existing profiles
- Explicit profile, environment and saved-default precedence is consistent across frontends and existing edited profiles/default files remain preserved
- A private machine with no agent installed reaches usable GUI setup and headless startup gives an actionable configuration error without vendor calls or implicit package installation
- Named clean-start process and GUI checks plus affected compilation pass without touching real profiles or restarting the shared backend
id: 01M6M65A1VYVMQNPSTJ46YD5S2
status: claimed
delegated_by: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-04T21:23:49.309Z
created: 2026-11-04T21:06:40.639Z
number: 93377961363555
created_by: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
updated_by: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
authorship:
  signer: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
  kind: seat
  public_key: JlpXQZ4/Qy0yh5dcFR+iK7RgP5q83vzdNaLijybKm+Y=
  process_generation: '1'
assignee: seat:01M6M69Y5FVMKEHTCVGWKY8EZH
provenance:
  digest: sha256:e3854e604c7798d04eb98c1b59a22948071242cc8ecfd3c75a9230d96ecb82f3
  frontmatter_digest: sha256:151573e820dd7d40e4c15ae806f449f9244bc29d586148ea3c5435ff2f076674
  signer: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
  sig: hBHDPLKRIePYCiAwzHumlgJXbM36CAsWq1p/MOfWn+avU+fa6p5dLdxkdhRdYeAfQPWL9UULlu1vUbiFOIZoAg==
  log: .bboxy/log/2026-11-04.jsonl#130783
---
ROLE: EXECUTOR.

Direct operator: trace whether default is codex or codex-acp, and ensure a clean machine works. Oracle verified generated canonical key/file is codex / profiles/codex.md; agent.command is [codex-acp]. Current BUGS: headless.rs:70-82 falls back to nemotron with no saved default, and GUI options.rs:42 independently hard-codes nemotron and ignores saved default/env. Discovery currently runs through GUI Bridge; clean CLI/seat startup can try loading a profile before it exists.

Own warm checkout /Users/feral/mydev/bboxy450-term-onboarding, producer branch codex-harness-onboarding-20261104 at f1d3a3f; Oracle fast-forwarded parent to current main2f4d2f13. Single writing executor. Use new bboxy450-harness profile codex (old codex-acp/exe profile files were deliberately archived). Oracle owns ALL Git fetch/pull/rebase/merge/pins/push/build/install. Do not do them.

Implement the smallest shared default/bootstrap path so normal clean startup works, rather than fixing only this operator's default-profile file. Canonical generated filename remains codex.md and command codex-acp, never create legacy aliases. With a private empty home and fake codex/codex-acp installed, normal GUI plus CLI/seat startup must discover/create the right profile and select codex without nemotron, user edits, login or network. Honor explicit --profile above nonblank BBOXY450_HARNESS_PROFILE above saved default above canonical first-run Codex behavior, consistently across frontends. Preserve existing valid/custom profiles and saved preferences; no silent overwrite or unrelated agent fallback. If needed initialize a missing saved default atomically only when a usable canonical Codex profile exists; never overwrite an existing user choice. A machine without an installed agent must get the existing working welcome/setup UI; non-GUI must fail with actionable setup guidance rather than an unexplained missing nemotron file. No automatic package installs or paid/live vendor calls.

Keep profile discovery/filesystem work off GUI callbacks; reuse the current bounded discovery/create-new machinery, existing launcher/context/MCP/yolo semantics and restored loading priority. Do not redesign welcome, rename profiles to adapter executable names, touch real profiles/credentials, switch running agents, or restart backend. Current real home already has ONLY generated claude/codex/goose/grok/opencode/pi and saved default codex; leave it untouched.

Add make verify-clean-start with only named private temp-home/PATH/fake-adapter process and actual GUI/Bridge checks, plus affected strict compilation/license checks. Cover missing root/profile/default file, no installed CLI, native CLI without wrapper, proper argv/profile identity, env/saved/explicit precedence, preserved edits, repeated startup and concurrent create-new if relevant. Parsing/layout keep specialized checks; reuse unchanged protocol/runtime/formal evidence and add only a minimal affected model/control if you introduce a new ownership contract. No full CI/parity/formal/runtime suites. Two Cargo jobs, local target; disk92%, Codex96% weekly—keep this bounded and deliver promptly.

Commit a signed producer source checkpoint immediately when the focused gate passes; do not wait for report indexing. Parent holds only Make wrapper, exact indexed task/report and compact .bboxy/runs/codex-clean-start.json; leave its gitlink unstaged. Register append-only report reports/2026-11-04-harness-clean-start.md via knowledge tools, copy exact indexed bytes, signed report commit with Report trailer, task_report using acceptance strings VERBATIM. Save raw logs/fixtures outside Git, only command/exits/hashes in run JSON. Unknown write: retain original op ID and query it, never redispatch an unknown effect. Quiet: no plan/progress/ack DMs, only genuine all-work blocker and final task_report. No root or peer worktree edits, broad staging, reset/stash, OS desktop input or test needing the operator.


## Oracle source-review correction — same bounded startup scope

The current fdaaf876 candidate fails a private existing-behavior witness: an explicit valid custom ACP profile whose executable exists only in the login-shell PATH succeeds with installed f1d3a3f (exit 0 and PRIVATE-CLEAN-START), but candidate exits 1 before launch. Exact private instrument/results: /tmp/bboxy-oracle-recovery-01M6KDR7TE4H4B4AF0PZYK0Q1T/clean-start-login-path-review.json and clean-start-login-path-{installed-baseline,candidate}.log. The process owns a private HOME, fake login shell and fake adapter; no vendor or credentials.

Remove the new narrower inherited-PATH rejection or reuse production ACP resolution for any early availability check. backend/acp_agent.rs is now allowed solely to expose/reuse that existing bounded resolver if needed; preserve its launch semantics. Preserve the actual profile parse/config error when adding setup guidance. Add this regression to the existing focused fixture, run only the changed named checks/affected compile, and refresh the compact exact-source receipt. No full suite, new formal model, backend action or installer run. New-session default discovery must not change saved/environment selection during resume; retain existing Host identity handling. This source correction must be complete before final handoff; the old focused green alone is insufficient.
