---
type: report
title: 'Clean-start correction: preserve ACP login PATH, diagnostics and resumed identity'
task: '[[01M6M65A1VYVMQNPSTJ46YD5S2]]'
supersedes: '[[reports/2026-11-04-harness-clean-start]]'
commits:
- fdaaf876c27962a918a7e87760c7296e7c03681b
- 87f0a3e7c2ed4c5e61eec3b9b89767547c052b50
verify:
  command: make verify-clean-start
  exit_code: 0
author: seat:01M6M69Y5FVMKEHTCVGWKY8EZH
created: 2026-11-04T21:30:54.489Z
id: 01M6M7HMHES8NTJVKZXR5QANMB
updated: 2026-11-04T21:30:54.489Z
number: 93377961363612
created_by: seat:01M6M69Y5FVMKEHTCVGWKY8EZH
updated_by: seat:01M6M69Y5FVMKEHTCVGWKY8EZH
authorship:
  signer: seat:01M6M69Y5FVMKEHTCVGWKY8EZH
  kind: seat
  public_key: 36C6in7H3OYeio+A+4Zx6Sm72MSoBpiY8NcA2hCFack=
  process_generation: '1'
provenance:
  digest: sha256:f31c29fcc23181a2517376771974257d7323bbe87469cfe3deb36b384b4821e2
  frontmatter_digest: sha256:3b60f90a80aea38ec923389eef752e8de4c9cd86396938833614015d55f67efc
  signer: seat:01M6M69Y5FVMKEHTCVGWKY8EZH
  sig: O2ZQfbOqXmsXhIfiBZlciiFKgDl0zAD3bDi7YKxKk2Zt9rlNcKfGmJDqJ2I6ju7ci8zPFwBUirBAYonLyxxMBA==
  log: .bboxy/log/2026-11-04.jsonl#130827
---
Supersedes [[reports/2026-11-04-harness-clean-start]]. The initial candidate missed the Oracle's login-shell PATH compatibility case. Final signed producer: `87f0a3e7c2ed4c5e61eec3b9b89767547c052b50`, following checkpoint `fdaaf876c27962a918a7e87760c7296e7c03681b`. `git verify-commit` exited 0 with Boxy's approved key `9B07E849480C68328CA20684844ECC0587DE7AB0`. Parent handoff follows c43a47be and leaves the producer gitlink unstaged.

## Canonical Codex profile identity is codex, its generated file is codex.md and its ACP executable is codex-acp without duplicate profile generation

`tests/clean_start.rs:45,155` retains passing canonical codex/codex.md/[codex-acp], actual argv, no-alias, missing-profile regeneration, repeat-startup and six-publisher create-new assertions. Shared bootstrap is `src/onboarding.rs:333`; no default file is written.

## A private empty home with fake installed Codex/ACP tools starts through normal GUI and CLI or seat entrypoints without relying on legacy nemotron or pre-existing profiles

Seven fresh process tests pass, including the normal prompt/JSON CLI, seat/socket and hidden native GUI with private empty HOME/PATH/fake adapters (`tests/clean_start.rs:45,126,181`). Idle child launch remains lazy. The shared GUI startup worker remains `bridge.rs:562`.

## Explicit profile, environment and saved-default precedence is consistent across frontends and existing edited profiles/default files remain preserved

Shared selection remains explicit > nonblank environment > saved > codex. `headless.rs:306` now uses production login_path/find_agent instead of the narrower inherited-PATH check. `backend/acp_agent.rs:74,115` changes only those functions' visibility to pub(crate); normalizing those two declarations reproduces the entire f1d3a3f file exactly. Real spawn implementation is unchanged.

`tests/clean_start.rs:233` proves an explicit valid custom ACP profile launches a fake adapter found only in its private login-shell PATH, preserving profile bytes. `:277` crashes its recorded private idle-seat PID and resumes with missing unrelated saved/environment selections. Existing Host handling retains the original profile identity and edited executable. Saved bytes remain intact; no Grok profile is generated despite its newly installed fake executable. Original CLI precedence and five isolated actual Bridge precedence scenarios remain covered.

## A private machine with no agent installed reaches usable GUI setup and headless startup gives an actionable configuration error without vendor calls or implicit package installation

`headless.rs:312` preserves the actual profile parse/config error before appending setup guidance. `tests/clean_start.rs:233` checks missing frontmatter and missing agent.command; `:101` checks no CLI and native CLI without its wrapper. Bare executable checks share the existing bounded login-shell resolution; direct paths retain Command's cwd-relative semantics. Fixtures own SHELL as well as HOME/PATH, preventing access to real installed tools. Normal no-agent GUI setup remains green.

## Named clean-start process and GUI checks plus affected compilation pass without touching real profiles or restarting the shared backend

Final `make verify-clean-start` exited 0 with CLEAN_START_REUSE_RECEIPT=/tmp/bb-clean-start-initial-receipt.json. Fresh commands: seven process/native GUI tests; strict standalone lib/bin/clean_start Clippy with -D warnings; embedded without ACP compilation; license headers; parent check-licenses. Reused without rerun: four unchanged GUI option tests and two unchanged actual Bridge/setup tests. Source guards and SHA256 log checks verify this evidence; no installer witness reran. Cargo jobs/test threads are bounded to two. Final gate source hashes matched the signed producer checkpoint.

Compact evidence: `.bboxy/runs/codex-clean-start.json`. Fresh raw logs: `/tmp/bb-clean-start-gate-in0puufy`. Unchanged create-new, installer ownership, protocol/runtime/MCP/formal implementation and retained native/TLC graph evidence remain as documented in the superseded report: positive model 10 states/19 transitions passes; broken control 22/38 fails safety. The backend's two visibility-only changes are explicitly recorded instead of claiming byte identity. No new model/control or full CI/parity/runtime/formal suite ran; no arbitrary Rust refinement claim.

The first resume fixture correctly failed because it tried reopening an ordinarily closed CLI session; the corrected fixture creates a real resumable crash log by killing only its owned idle-seat PID. The Oracle's login-PATH regression now passes. Report creation initially returned a definitive invalid_report rejection for a missing created timestamp; the corrected write is a fresh operation, not a retry of an unknown effect.

No real profiles/credentials, vendor calls, real package installs, shared backend restart, standalone build/install, fetch/rebase/push/merge or gitlink pin occurred. Producer tree is clean. The separate signed parent correction commit contains only the exact amended task, append-only correction report and refreshed evidence. The earlier report stays immutable; the Oracle owns landing/pinning.
