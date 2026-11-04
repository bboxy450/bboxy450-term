---
type: report
title: Shared Codex defaults and unattended empty-home startup
task: '[[01M6M65A1VYVMQNPSTJ46YD5S2]]'
commits:
- fdaaf876c27962a918a7e87760c7296e7c03681b
verify:
  command: make verify-clean-start
  exit_code: 0
author: seat:01M6M69Y5FVMKEHTCVGWKY8EZH
created: 2026-11-04T21:23:15.637Z
id: 01M6M73NBVFMKS9GAVQRMF4R1T
updated: 2026-11-04T21:23:15.637Z
number: 93377961363604
created_by: seat:01M6M69Y5FVMKEHTCVGWKY8EZH
updated_by: seat:01M6M69Y5FVMKEHTCVGWKY8EZH
authorship:
  signer: seat:01M6M69Y5FVMKEHTCVGWKY8EZH
  kind: seat
  public_key: 36C6in7H3OYeio+A+4Zx6Sm72MSoBpiY8NcA2hCFack=
  process_generation: '1'
provenance:
  digest: sha256:5b0a2fc93172c6e283e5d383f58a3aef08a0b3df1c34a0e3755fcdaa4aee2346
  frontmatter_digest: sha256:dffa436c906de125232558fd3966b5408bec70b126821cf358732cc617cc411d
  signer: seat:01M6M69Y5FVMKEHTCVGWKY8EZH
  sig: Gbt3ER3hJL8QxQhdxBRbb7iWeaGgdiyk3yGFnakgq6WJWHY6lF5Idkk5KVtWE7cKUYPXwkSMC0Xb+PD3N5lQCg==
  log: .bboxy/log/2026-11-04.jsonl#130751
---
Implemented shared local bootstrap and default selection in signed producer checkpoint `fdaaf876c27962a918a7e87760c7296e7c03681b` (Boxy, approved fingerprint `9B07E849480C68328CA20684844ECC0587DE7AB0`; Git signature grade G). Parent base is `2f4d2f13`; its separate signed handoff commit contains only the Make wrapper, exact indexed task/report and compact run evidence. The producer gitlink remains unstaged for the Oracle.

## Canonical Codex profile identity is codex, its generated file is codex.md and its ACP executable is codex-acp without duplicate profile generation

`crates/bboxy450-harness/src/onboarding.rs:333` reuses the existing bounded discovery and atomic create-new profile publication before selection. It retains canonical identity/file `codex`/`codex.md` and command `[codex-acp]`; it creates no executable-name alias. `tests/clean_start.rs:45` checks parsed identity, actual fake-child argv, no legacy alias, missing-profile regeneration and repeated startup. `:155` runs six simultaneous create-new discoveries and asserts one complete preserved file without staging leftovers.

## A private empty home with fake installed Codex/ACP tools starts through normal GUI and CLI or seat entrypoints without relying on legacy nemotron or pre-existing profiles

`src/frontends/headless.rs:277` performs local bootstrap before profile loading. `src/frontends/gui/window.rs:247` starts through `Bridge::create_with_defaults`; `bridge.rs:562` awaits the same bootstrap on its background worker before Host creation. Filesystem/discovery work stays outside GUI callbacks. `tests/clean_start.rs:45`, `:126` and `:181` exercise normal prompt/JSON, hidden native GUI and seat/socket startup with private empty HOME/PATH and fake adapters. Idle GUI/seat startup preserves lazy child launch; the CLI's real local turn produces PRIVATE-CLEAN-START and verifies executable argv.

## Explicit profile, environment and saved-default precedence is consistent across frontends and existing edited profiles/default files remain preserved

The shared selector applies explicit --profile, nonblank BBOXY450_HARNESS_PROFILE, trimmed nonblank saved default, then codex. `gui/options.rs:13` retains explicit intent without an independent hard-coded default. `tests/clean_start.rs:45` verifies all precedence steps, blank environment handling, edited canonical bytes, exact saved-default bytes and a missing saved choice refusing fallback. `window.rs:1161` runs five isolated actual Host/Bridge scenarios for the same selection order. Missing default files remain missing: no new persistence or default ownership protocol is introduced. Scripted/replay/resume paths retain their existing host/loading priority; resume is allowed to restore its saved profile even if the current default is unavailable.

## A private machine with no agent installed reaches usable GUI setup and headless startup gives an actionable configuration error without vendor calls or implicit package installation

`onboarding.rs:358` supplies setup guidance; `headless.rs:298` checks local ACP executable availability and fails before creating a new unlaunchable session. `tests/clean_start.rs:101` covers no CLI and native Codex without its wrapper, with an actionable gui/--profile/codex-acp error and no unrelated fallback. `:126` confirms the normal hidden native GUI reaches the existing usable setup with zero profiles. The existing `onboarding_empty_home_live_bridge_refresh` witness also passes its explicit fake-manager same-view refresh. No real installer, vendor endpoint, login or credential command is used.

## Named clean-start process and GUI checks plus affected compilation pass without touching real profiles or restarting the shared backend

`make verify-clean-start` exited 0 on the source now in producer checkpoint fdaaf876. Its six producer commands all exited 0: five private process/native-GUI tests; four GUI option tests; two actual Bridge/setup tests; strict standalone Clippy for lib/bin/clean_start with -D warnings; embedded without ACP cargo check; license headers. The parent included `make check-licenses` also exited 0 (34 workspace crates; source notices held). Both Cargo jobs and test threads are bounded to two, with the worktree-local target. Source hashes matched before/after the gate and again after the signed checkpoint. Command argv, exit codes, raw-log hashes and source hashes are in `.bboxy/runs/codex-clean-start.json`; raw output stays under `/tmp/bb-clean-start-gate-xae6fnrz`.

State/verification scope: initial root/profile/default presence is absent or present; selection sources are absent/blank/named; owners are each startup worker and the existing create-new file publisher. Safety is no overwrite, canonical identity, precedence and no unrelated fallback. Progress assumes local filesystem calls return; deterministic startup completes before Host creation. Six concurrent publishers witness existing atomic hard-link publication. The create-new implementation, installer admission/cancellation/reap/generation code, protocol, runtime, backend, MCP and formal sources were compared unchanged against f1d3a3f (25 files). Retained native/TLC graphs were reread and compared: HarnessOnboardingInstall 10 states/19 transitions, pass; broken control 22/38, safety failure (TLC exit 12). The prior model-command exit/log hash and formal input hashes were verified. No new owner/lifecycle/persistence contract required a new model/control; this is no arbitrary Rust refinement claim.

Diagnostic failures were resolved before the final gate: the sandbox refused private Unix sockets/native GUI services; rerunning outside that restriction passed. Two new fixture assertions incorrectly expected eager agent launch, and two Bridge test helper names did not match the repository API; those fixtures were corrected. Full CI, parity, runtime/formal suites, live vendor authentication/inference, real profile/credential changes, backend restart, standalone build/install, fetch/rebase/push/merge and gitlink pin were deliberately excluded by the brief. Producer tree is clean. Parent handoff signature and exact mirrored bytes are verified in the final handoff command; the Oracle owns landing/pinning.
