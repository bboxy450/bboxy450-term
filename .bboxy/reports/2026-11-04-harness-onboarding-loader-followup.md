---
type: report
title: Restore session loading, compact centered setup, runtime menu naming and Goose discovery
task: '[[01M6KT6ZT468RVV2SZ4ANS42KX]]'
supersedes: '[[01M6KX56JY0FXY5VY3FNXNXCDQ]]'
commits:
- 278f4516e0b822d9cffa29c1d165feb74cd06089
- f9ed639cec2713dd53bf1ba1e3f3a7b6d4347db3
- f1d3a3f46d5d460407ea047e19e6e013ee8f52c8
verify:
  command: make verify-harness-onboarding
  exit_code: 0
author: seat:01M6KYK93CXV3YMNFVM5WJ3Y3V
created: 2026-11-04T19:11:06.613Z
id: 01M6KZHNJJ0V6PEFJY34QGDVWD
updated: 2026-11-04T19:11:06.613Z
number: 93377961363481
created_by: seat:01M6KYK93CXV3YMNFVM5WJ3Y3V
updated_by: seat:01M6KYK93CXV3YMNFVM5WJ3Y3V
authorship:
  signer: seat:01M6KYK93CXV3YMNFVM5WJ3Y3V
  kind: seat
  public_key: L35fG+pdud/b1f8S+WvaBsLGjjdfhVfEOjnvBFsdBc4=
  process_generation: '1'
provenance:
  digest: sha256:f369f6fc2aa5ae466712ffc95fd28e61a9721675d079128664ef4bc1e098c38e
  frontmatter_digest: sha256:82758a87b08e5f8e04d6ede721c49e1205c7efcc0b94101d5c862c0c602af5ea
  signer: seat:01M6KYK93CXV3YMNFVM5WJ3Y3V
  sig: obXIIvx3+/fGO4zWA32BUQIl3ggeTtgrPHNLTHcsRM/KHpheBnZuU8UP59vsF6nYP+yqDWZMMXjDDWLsgAJbDw==
  log: .bboxy/log/2026-11-04.jsonl#129906
---
Restored the session loader first in signed producer checkpoint `278f4516e0b822d9cffa29c1d165feb74cd06089`, after the one named loading/welcome check exited 0. Producer follow-ups are `f9ed639cec2713dd53bf1ba1e3f3a7b6d4347db3` and `f1d3a3f46d5d460407ea047e19e6e013ee8f52c8`. All three passed `git verify-commit` with Boxy's approved key `9B07E849480C68328CA20684844ECC0587DE7AB0`. The separate parent commit contains the shared menu fix, gate wrapper, synchronized attribution copies, this indexed report/task mirror and the compact receipt. Its exact SHA is supplied by task_report. The parent gitlink stays unstaged; the Oracle owns integration, pins, push, build and install.

## Installed supported harnesses receive editable ACP profiles automatically, later installations are detected, and existing profile bytes are preserved

`crates/bboxy450-harness/src/onboarding.rs:73` — adds Goose as the existing executable `goose acp`, with no invented package-manager recipe. `onboarding.rs:218` retains create-new atomic profile publication; `:254` generates only when a known executable is relevant. Existing profiles/defaults/model/auth data remain untouched. `tests/harness_onboarding.rs:26` proves absent Goose creates nothing, a later private executable generates exact argv, and edited Goose/Codex/custom bytes survive rediscovery and concurrent scans. OMP remains unsupported; Pi retains its prerequisite/MCP limitations.

Command facts checked on 2026-11-04: [official ACP Goose registry](https://github.com/agentclientprotocol/registry/blob/main/goose/agent.json) and [official Goose ACP documentation](https://block.github.io/goose/docs/gdk/acp/) specify `goose acp`. No vendor implementation was copied.

## Welcome setup distinguishes missing CLI and adapter and offers verified platform-appropriate install choices with responsive progress and honest failure recovery

`src/frontends/gui/view.rs:575` — automatic setup requires settled metadata and an actual unconfigured welcome, no turns/pending prompts, and no attached drawn history. Eager discovery, deferred catalogue/metadata and ready=false keep the existing centered `face::loading_session`. Drawn out-of-turn history uses the retained preparation path; it is never inferred empty from the turns map.

`bridge.rs:141,496,721` — a presentation-only creation intent is bound to the exact locally created session and cleared for resume/watch/replay/attach/connect. This allows first-run profile-resolution failure to open setup only after its diagnostic is complete, retaining that diagnostic in the panel. Attached history does not inherit this intent. `tests/harness_onboarding.rs:440` checks loading before/after discovery, absent/deferred metadata/catalogue, replay, settled welcome, and out-of-turn history before and after preparation.

`setup.rs:57,101` — existing toolkit/palette/portrait, centered panel capped at 600 px, compact status rows with Use/Install, working Details/Less controls for instructions/auth/PATH/manager commands and alternate supported managers. Progress/error/output/cancel stay visible. `view.rs:603` swaps independent setup/conversation scroll state. `tests/harness_onboarding.rs:284` checks panel/portrait centering, visible compact first rows, actual disclosure actions and restored scroll offsets at 320/800 in light/dark; four native offscreen captures were inspected. `window.rs:1112` corrects the private startup witness to wait for the settled setup frame instead of connection readiness.

`crates/bboxy450-term-desktop-core/src/lib.rs:168` — the existing menu API selects the standalone harness name from the runtime executable, with the terminal name as fallback. Only app/Hide/Quit titles change. Existing menu action, shortcut and session-preservation checks pass; no producer dependency on a new parent API was added.

## Installation runs only after an explicit UI action through fixed supported commands and private unattended tests prove ownership, cancellation and duplicate guards

Installer ownership/allowlist/timeout/cancel/reap code and recipe argv are unchanged. The follow-up gate compares those source regions to producer `222d64c550574813035d6f29dc1edcd52f343d30`, verifies retained command/log hashes, and reuses its private installer/model evidence. Current private same-view startup again exercises an actual Install action and immediate catalogue refresh. No real package installs, credentials or operator input occurred.

## Durable and queued user messages render Markdown with bounded themed layout while prior history, attachment content and causal delivery state remain intact

The existing Markdown implementation is retained. Five named library witnesses pass, including the user Markdown/pending/history and causal-durable-user guard plus actual empty-home/same-view startup. Existing four Markdown captures remain alongside four regenerated setup captures. Loader routing uses the existing retained rows/pending machinery; message content, attachment custody and causal delivery semantics were not changed.

## Focused private discovery, profile, installer, Markdown and native GUI checks plus affected strict compilation and license verification pass without backend restart or operator input

Final invocation: `ONBOARDING_REUSE_EVIDENCE=target/harness-onboarding/baseline.json make verify-harness-onboarding` — exit 0. The canonical assigned command remains `make verify-harness-onboarding`.

- Five focused integration cases — exit 0: registry/profile/manager, loader routing, native setup layout and actual private hidden-window startup.
- Five named library cases — exit 0: Markdown/pending/history/causal guards and private same-view startup.
- Existing shared menu semantics tests — exit 0.
- Strict producer standalone Clippy, strict parent library/binary Clippy, embedded-without-ACP compilation — each exit 0.
- Producer license headers and parent license/copy/source-notice checks — each exit 0.
- `git diff --check` for producer and parent — exit 0.
- 84 final source hashes matched after verification; all command logs were independently checked against receipt hashes.

Evidence: `.bboxy/runs/codex-harness-onboarding.json`. Raw command logs, private startup evidence and eight captures remain outside Git under `target/harness-onboarding/`. Installer finite-model/source/tool inputs are unchanged; retained native and Java graph sets were compared again and graph artifact hashes recorded: good model 10 states/19 transitions, broken control 22/38, prior TLC exits 0 and expected safety failure 12. Java was not rerun. These models cover installer ownership, not arbitrary Rust, vendor behavior or UI routing; the latter uses implementation/native witnesses.

Resolved checks: ambient sccache was denied by the sandbox (exit 2), so targeted checks used the existing wrapper-free environment; sandboxed Metal had no adapter (exit 100), then the private offscreen check passed outside the sandbox. The first follow-up gate exited 2 because the existing startup test stopped before replay settled; its wait barrier was corrected and the final gate passed. No failed gate was bypassed.

Skipped by the explicit brief: full CI/parity/runtime/formal corpus, paid/live vendor tests, production bundle/install, backend restart, desktop input/capture, fetch/pull/rebase/merge/push and parent gitlink pin. Working-file changes present at assignment start were preserved and included in the parent handoff. Producer worktree is clean; parent retains only the intentional unstaged producer gitlink after its final commit.
