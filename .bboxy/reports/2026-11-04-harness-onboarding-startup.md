---
type: report
title: Real empty-home startup and same-view setup refresh
task: '[[01M6KT6ZT468RVV2SZ4ANS42KX]]'
supersedes: '[[reports/2026-11-04-harness-onboarding]]'
commits:
- 222d64c550574813035d6f29dc1edcd52f343d30
verify:
  command: make verify-harness-onboarding
  exit_code: 0
author: seat:01M6KT5EV066KVVBAPR7RGJWAX
created: 2026-11-04T18:51:29.866Z
id: 01M6KYDNDR7CA33GE54TM6KGBQ
updated: 2026-11-04T18:51:29.866Z
number: 93377961363315
created_by: seat:01M6KT5EV066KVVBAPR7RGJWAX
updated_by: seat:01M6KT5EV066KVVBAPR7RGJWAX
authorship:
  signer: seat:01M6KT5EV066KVVBAPR7RGJWAX
  kind: seat
  public_key: Gv4EtBJ4cvTECBqEzBpi8oLdskOwVtHSeaY0BgzOMxg=
  process_generation: '1'
provenance:
  digest: sha256:eb4b186687e8af6cc780a4cd9b6ee3016809ed8286e47d4702711118eb99e2d6
  frontmatter_digest: sha256:9cbe0d1db09ad157bf5f63d10293c400a1445f6d51c119fc37ea5929b96f0a5f
  signer: seat:01M6KT5EV066KVVBAPR7RGJWAX
  sig: 2tvS/sudpY6v/kOySvAUP/C8Ca5P9feC/9sZ78OgbXg3Eodb0yFcm3oirHDt/oT2ezLrGNqZY4B9meretIX+CQ==
  log: .bboxy/log/2026-11-04.jsonl#129238
---
The startup evidence now runs the actual app and the real live Host/Bridge path. This append-only update supersedes [[reports/2026-11-04-harness-onboarding]]; its official recipe research, implementation explanation and earlier signed Markdown checkpoint remain applicable.

Producer follow-up: `222d64c550574813035d6f29dc1edcd52f343d30`, signed by Boxy's approved key and verified with `git verify-commit` (exit 0). Earlier producer commits: `2b14eb718b025a64f9ea2778692e6a34329aa66e` and `3f151e4b001726d99df049796e684e70eee563b4`. Consumer base for this follow-up is `5a8d6503a4f65848fdbee2deb0ad650bd04a6d06`. The new parent handoff SHA is supplied by task_report. The parent gitlink remains unstaged; producer worktree is clean.

## Installed supported harnesses receive editable ACP profiles automatically, later installations are detected, and existing profile bytes are preserved

The discovery/profile/installer implementation from 3f151e4 is unchanged. Its private absent/present-later, edited/custom byte preservation, eight concurrent writers, unsupported-adapter, duplicate/cancel/retry and runtime-refresh tests pass again. Atomic profile publication remains `crates/bboxy450-harness/src/onboarding.rs:210`; discovery remains `:246`.

New `src/frontends/gui/window.rs:1070` starts the actual production standalone configuration and a live Host/Bridge in an isolated child test process. HOME/PATH are private, the profiles directory is initially absent, and the resulting real catalogue has zero profiles. It executes the install action obtained from the first live HarnessView frame. The fake npm validates the exact fixed argv and creates only the private wrapper. The same bridge/view then exposes the generated Codex profile and available adapter. Assertions prove the session and epoch remain unchanged; no session/backend restart is used for refresh.

## Welcome setup distinguishes missing CLI and adapter and offers verified platform-appropriate install choices with responsive progress and honest failure recovery

`tests/harness_onboarding.rs:428` launches the actual compiled `bboxy450-harness gui` binary with a private empty home, private PATH containing only fake npm, and the existing hidden smoke-window mode. This executes main/options/default-profile initialization and the actual native event loop. It asserts process exit 0 and the native-render diagnostic:

```text
smoke-window: native window created; surface configured; live frame submitted to private texture; visible=false; setup=true; profiles=0
```

The test also asserts no installer was run during startup. A default-profile loading exit, a loading-only frame, a missing setup node or a visible window fails. `src/frontends/gui/window.rs:639` strengthens smoke completion to require the setup node for an empty profile catalogue. The window stays hidden and renders only to its private texture.

After the fake install, the live bridge witness renders the same HarnessView again and asserts both adapter availability and installation-complete status. The existing light/dark setup captures and manager-specific recipe tests pass. Static verified recipes and their official sources are unchanged; see the superseded report for the primary-source links.

## Installation runs only after an explicit UI action through fixed supported commands and private unattended tests prove ownership, cancellation and duplicate guards

The new refresh witness calls `HarnessView::activate` with the real Codex install hit action taken from the actual frame, then checks the fake-manager argv record. It does not call the installer directly. The unchanged installer ownership, environment filtering, bounded output/deadline, cancel/reap and stale-generation witnesses pass again.

`tests/support/onboarding_startup.rs` owns both child-process fixtures, clears inherited environment, supplies private HOME/PATH, uses null stdin and applies a 30-second deadline to the exact child PID it started and recorded. Fake installation writes only under that home. No real manager or vendor agent runs.

The existing new install model/control is unchanged and its complete native/Java TLC graphs and verdicts match again: good 10 states/19 transitions, pass/TLC exit 0; broken 22 states/38 transitions, expected safety failure/TLC exit 12. Weak fairness of Reap and the recorded finite abstractions remain assumptions; no arbitrary Rust refinement or full TLC-completeness claim is made.

## Durable and queued user messages render Markdown with bounded themed layout while prior history, attachment content and causal delivery state remain intact

The shared Markdown implementation is unchanged. The native light/dark 280/440 px user rendering/source/history/queued checks, the existing causal confirmation guard and all eight retained capture hashes are included in the fresh focused result. No rendering path executes shell/code.

## Focused private discovery, profile, installer, Markdown and native GUI checks plus affected strict compilation and license verification pass without backend restart or operator input

`make verify-harness-onboarding` — final-source exit 0. It now includes seven private integration tests and five selected library tests, including the actual app empty-home startup and live same-view refresh. Producer standalone/test strict compilation, embedded-without-ACP compilation, strict consumer compilation, producer headers, parent license/source-notice consistency and the dedicated finite model/reference comparison all pass.

The two new targeted startup tests also passed separately (2 passed, exit 0). The canonical gate reran them with the other affected checks. Private native process output and same-view child-process output are retained in `target/harness-onboarding/startup/{empty-home-native-app,same-view-refresh-process}.json`; both have exit 0 and their hashes are included in `.bboxy/runs/codex-harness-onboarding.json`. Full command exits, source hashes, eight captures and graph/verdict counts remain in that committed receipt. Raw logs/captures stay outside Git.

No real software installation, credential use, vendor inference, operator clipboard/desktop control, production bundle, fetch/pull/merge/push/pin or trusted-app/backend restart was performed. Only the owned private test app/actor were started and detached for the witnesses. Notice copies from the previous handoff remain synchronized.