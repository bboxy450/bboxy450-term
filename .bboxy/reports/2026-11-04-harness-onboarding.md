---
type: report
title: ACP discovery, interactive setup and shared user Markdown
task: '[[01M6KT6ZT468RVV2SZ4ANS42KX]]'
commits:
- 2b14eb718b025a64f9ea2778692e6a34329aa66e
- 3f151e4b001726d99df049796e684e70eee563b4
verify:
  command: make verify-harness-onboarding
  exit_code: 0
author: seat:01M6KT5EV066KVVBAPR7RGJWAX
created: 2026-11-04T18:29:20.493Z
id: 01M6KX56JY0FXY5VY3FNXNXCDQ
updated: 2026-11-04T18:29:20.493Z
number: 93377961362754
created_by: seat:01M6KT5EV066KVVBAPR7RGJWAX
updated_by: seat:01M6KT5EV066KVVBAPR7RGJWAX
authorship:
  signer: seat:01M6KT5EV066KVVBAPR7RGJWAX
  kind: seat
  public_key: Gv4EtBJ4cvTECBqEzBpi8oLdskOwVtHSeaY0BgzOMxg=
  process_generation: '1'
provenance:
  digest: sha256:145d76e61714c15df0fe6a125e10a47c8a50860c951f41e02262510b2a08d1bb
  frontmatter_digest: sha256:0ac3e77947777b4e30082db2f631e6157c8b5034260000c1613530d18616cb60
  signer: seat:01M6KT5EV066KVVBAPR7RGJWAX
  sig: 88uFhQzM8isOdWEQ1SH6/qBFq72e5O8rIdLSPqaI5JiVTzoFR/V5eoCLgokJTOVnKa8t3fy/bkXZlhGuQdjBAA==
  log: .bboxy/log/2026-11-04.jsonl#127974
---
Implemented the assigned shared welcome/setup flow and user Markdown rendering. The small Markdown checkpoint is `2b14eb718b025a64f9ea2778692e6a34329aa66e`; the completed producer is `3f151e4b001726d99df049796e684e70eee563b4`. Both passed `git verify-commit` with Boxy's approved Ed25519 key `9B07E849480C68328CA20684844ECC0587DE7AB0` (exit 0). Consumer base: `5cdfebdb3e207024b22338c935bc4b44e977455d`. The separate parent report commit contains the Make wrapper, exact task/report mirrors, receipt and generated notice copies; its SHA is supplied by task_report. The parent gitlink remains unstaged.

## Installed supported harnesses receive editable ACP profiles automatically, later installations are detected, and existing profile bytes are preserved

`crates/bboxy450-harness/src/onboarding.rs:210` — writes a complete temporary profile, syncs it and publishes with create-new hard-link semantics; existing files, edits and symlinks win. `:246` — discovers known CLIs/adapters using a bounded PATH scan, creates only relevant missing profiles and fingerprints profile metadata. Absent Grok creates no profile. `:379` — performs discovery on the blocking worker, with five-second runtime refresh and immediate post-install refresh. Existing profile/default/auth/model/effort/MCP records are never rewritten.

`tests/harness_onboarding.rs:26` — private absent/present-later CLI, missing wrapper, exact Grok argv, edited/custom byte preservation, eight concurrent discoveries, unsupported OMP and no leftover staging files. `:150` — later adapter additions and automatic refresh without restarting a session/backend. Profiles contain no credentials. Bounds are 64 absolute PATH entries and 256 profile metadata entries; filesystem availability and installed binary authenticity/version remain environmental assumptions.

## Welcome setup distinguishes missing CLI and adapter and offers verified platform-appropriate install choices with responsive progress and honest failure recovery

`src/frontends/gui/setup.rs:87` in the producer — existing portrait, theme inks and toolkit controls; native/adapter status, exact wrapped command, official instructions, progress/output, error/retry, cancel and selection. `setup.rs:57` supplies actual hit actions. `bridge.rs:501` publishes catalogue changes to both standalone and embedded HarnessView; there is no per-frame filesystem/process discovery. `view.rs:562` restricts automatic setup to new conversations, preserves active chat and supports returning to the conversation. The private startup regression covers an unresolved initial profile notice and an empty catalogue during active chat.

Reviewed official sources on 2026-11-04:
- [ACP registry](https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json) supplies the reviewed pinned packages: Codex 2.5.0, Claude adapter 0.100.0, pi-acp 0.0.34, Grok 1.0.74 and Gemini 0.66.0. Grok uses `grok agent stdio`; no guessed ACP flag or dynamically downloaded registry recipe is executed.
- [Codex adapter instructions](https://github.com/agentclientprotocol/codex-acp), [Claude package manifest](https://github.com/agentclientprotocol/claude-agent-acp/blob/main/package.json) and [Pi adapter instructions](https://github.com/svkozak/pi-acp) verify their executable/package contracts. Pi CLI prerequisites use the documented `@earendil-works/pi-coding-agent`; the offered CLI pin is 0.81.0.
- [Homebrew codex-acp](https://formulae.brew.sh/formula/codex-acp) verifies the offered Brew formula. No Claude/Pi/Grok Brew formula is invented.
- [mise npm backend](https://mise.jdx.dev/dev-tools/backends/npm.html) verifies global npm-backed tool installation. It is offered only with mise, Node and active shims already available.
- [OpenCode ACP](https://opencode.ai/docs/acp/) verifies `opencode acp`. [OMP's official repository](https://github.com/can1357/oh-my-pi) documents ACP client tool routing; automatic OMP setup is conservatively unavailable for this client's unimplemented terminal/filesystem services, with official/custom-profile guidance. Pi's community adapter explicitly warns that it does not forward MCP servers.

## Installation runs only after an explicit UI action through fixed supported commands and private unattended tests prove ownership, cancellation and duplicate guards

`src/onboarding.rs:399` — validates a concrete UI recipe against the available static allowlist and shares duplicate suppression between views. `:443` — generation checks reject stale completion. `:462` — direct argv, cleared environment with explicit HOME/PATH/CI settings, null stdin, the existing OwnedChild process-group fence, an 8 KiB output tail and a 180-second deadline. Cancellation retains ownership until reap; different agent installs are independent. No shell interpolation, sudo or manager bootstrap is offered.

`tests/harness_onboarding.rs:150` and `:247` — private fake-manager argv, no discovery execution, shared-view duplicate suppression, failure/retry, cancellation, process-group death, success and refresh, invalid/stale recipe refusal, environment filtering and output bound. `src/onboarding.rs:544` and `:583` — shortened private deadline/process witness and A/B/A stale-completion witness. The short fixture deadline was increased from 100 ms to two seconds after it raced process startup; product timeout remains 180 seconds.

`tests/formal/models.rs:4355` and `verification/contracts.toml:464` — smallest new Yarn-fronted install ownership model and broken admission/generation control. Complete native and pinned Java TLC graphs match: good model 10 states/19 transitions, verdict pass, TLC exit 0; broken model 22 states/38 transitions, expected safety failure, TLC exit 12. Progress assumes weak fairness of Reap, abstracting the local deadline. OS behavior, manager internals, bytes and wall time remain implementation witnesses. This is no claim of arbitrary Rust refinement or full TLC completeness.

## Durable and queued user messages render Markdown with bounded themed layout while prior history, attachment content and causal delivery state remain intact

`src/frontends/gui/face_rows.rs:304` and `view.rs:2930` — both forms reuse the existing Documents/StreamingMarkdown/highlight preparation on the worker, retaining bubble styling and attachment captions. `face.rs:1443` provides selectable original-source views. Shared inline layout retains separators between emphasis, links and code. The protocol/durable input bytes and causal confirmation state machine are unchanged.

`view.rs:3363` — actual light/dark native frames at 280/440 px assert headings, lists, bold, links, inline code, fenced Python, source bytes, render bounds, retained history and gray queued state. The existing `gui_polish_pending_requires_causal_durable_user_in_current_birth` check also passes. Rendering executes no shell/code. Eight offscreen Metal captures (four setup, four user Markdown) were retained outside Git and hashed in the receipt. The sandbox had no Metal adapter; native checks used offscreen GPU access, never desktop capture or injected input.

## Focused private discovery, profile, installer, Markdown and native GUI checks plus affected strict compilation and license verification pass without backend restart or operator input

`make verify-harness-onboarding` — exit 0 on the final source. Six private integration checks, four selected library checks, the dedicated formal export/control, strict producer standalone/library/test compilation, embedded-without-ACP compilation, strict consumer library/binary compilation, producer license headers and parent license/source-notice verification all pass. Full commands, real exits, tool pins, source hashes, eight capture hashes and log hashes are in `.bboxy/runs/codex-harness-onboarding.json`; raw logs/captures remain under the private local target and /tmp.

The additional parent license audit initially exited 2 because base 5cdfebd already had stale generated attribution copies. The authenticated 17:41 task amendment authorizes those outputs: `make sync-licenses` exited 0 and the final gate's `make check-licenses` exited 0. Copies were generated by Make, never independently edited.

Not run: full CI/parity/runtime/formal corpus, live vendor inference/authentication, real package/profile installation, production bundles, backend restart, operator clipboard/desktop input, fetch/pull/merge/push/pin/install. No credentials were read or committed. Producer worktree is clean; parent is committed separately with only the instructed gitlink change left unstaged.