---
author: seat:01M6ER6V2ZYS7Z1K05TE6ZJTJD
authorship:
  kind: seat
  process_generation: '1'
  public_key: dz6NmhXKbHy9Oj4XPzJo0/rAkWqtxM4tdPbxlsgDDG4=
  signer: seat:01M6ER6V2ZYS7Z1K05TE6ZJTJD
commits: []
created: 2026-11-02T18:47:08.381Z
created_by: seat:01M6ER6V2ZYS7Z1K05TE6ZJTJD
id: 01M6ESCBP2F6BAR3ABYRM5PRXR
number: 5002217
provenance:
  digest: sha256:68cbd6baa4f4c82b5a50966e7ae1e972ef86a55735cedbaafc36d50c61789eb4
  frontmatter_digest: sha256:d2fe7b43a2cf70ed52cd1101db22015fb57b94bccb068891f7a29bebf8a96a1c
  log: .bboxy/log/2026-11-02.jsonl#558
  sig: Fv4r/CUG3kq87t/S9eF9QGfUhjWb7Ah/+sGSr+cO93p1oVlBAt9mXDXtZPw4J8JZnkEEW51VuEoBNUhnMyD9Bg==
  signer: seat:01M6ER6V2ZYS7Z1K05TE6ZJTJD
task: '[[tasks/2026-11-02-chat-mute-notice-text]]'
title: Mute and unmute chat notice text verification
type: report
updated: 2026-11-02T18:47:08.381Z
updated_by: seat:01M6ER6V2ZYS7Z1K05TE6ZJTJD
verify:
  command: make test-chat-mute-notices
  exit_code: 0
---
ROLE: SOLO. Direct operator brief: mute/unmute produced timestamp-only chat rows. Worktree: `/Users/feral/mydev/bboxy450-term-chat-mute`, branch `fix/chat-mute-notices`. No other seat was recruited.

`crates/bbt-mux/src/messaging/service.rs:1145` derives readable `<signed presence name> muted/unmuted` text when building an operator MessageRow. The previous `Body::text()` projection returned None for structured Presence bodies. Both live arrival and Recent/history use this function. Source envelopes and policy behavior are unchanged.

`crates/bbt-mux/src/messaging/service.rs:3504` tests mute → unmute → mute, matching live/history rows after storage reopen, whole-second timestamps, original envelope equality and cryptographic verification. `crates/bboxy450-term/tests/swarm_workdir.rs:1876` adds chat-section assertions and captures to the existing private row-button/Cmd-M/restart regression. `Makefile:1175` exposes the focused nextest workflow. `.bboxy/specs/messaging.md:136` records the projection contract; the task record tracks the operator brief.

| Command | Actual exit | Evidence |
| --- | --- | --- |
| `make init-submodules` | 0 | Pinned harness 47b140fa9bd2bec99054394c206deb1b36d7b3b0 initialized; the first attempted test could not load its missing manifest (102). |
| `cargo nextest run --locked -p bboxy450-term-mux --lib -E 'test(mute_notice_rows_show_action_live_and_after_reopen_without_rewriting_envelopes)'` before the fix | 100, expected | One failure: actual row text `""`, expected `"recent fixture muted"`; /tmp/nyx-chat-mute-red.log. |
| `make test-chat-mute-notices` | 0 | One projection/storage test passed; one real private native app/PTY regression passed in 4.427s; /tmp/nyx-chat-mute-green.log. |
| `make test-licenses` | 0 | Six fixture tests; 33 crate licenses/notices and source notices held; /tmp/nyx-chat-mute-licenses.log. |
| `make smoke` | 0 | Ad-hoc private test installation, certified throwaway identity, headless render and scripted GUI controls; /tmp/nyx-chat-mute-smoke.log. |
| `git diff --check` | 0 | No whitespace errors. |

Visual inspection of `target/chat-mute-notices/seat-unmute-chat-text.png` showed “Nyx Copperwing muted”, “Vesper Greywind muted” and “Vesper Greywind unmuted” beside local times. `seat-mute-chat-restored.png` retained those rows and added “Nyx Copperwing unmuted” after UI restart. The captured JSON asserts the same text inside Chat. Existing process/generation/focus/operator-input assertions passed.

Log SHA256: red `080c1fda306de16b2a77d466799fbf909b540f7386e577ec4f1473ad942860ca`; focused green `c2fe37c47f76dd4dab6fde9c82446c9e2dc9805624d9e7aa121f4dba386ff1af`; licenses `200b07026286035705bd74125257216a3804a4131054e4c8c1f9f3f76d845b16`; smoke `2d465a6f33294f6e0fde5a51f47b9c671450d28814af4232c3b4f2a2f49bc05c`. Capture SHA256: unmute `c30acbf32f5a2cab0d6d4344d5e370d57e36bea14643578909b814d8c306225e`; restored `56a41919250e6e334b9a155b9c96c2adc37797635d103c96833127e4f143f1a2`. Raw logs and captures stay outside Git.

This is a text projection change; no lifecycle, concurrency, persistence or formal model transition changes. Existing formal models/negative controls remain applicable. UI geometry/reference and process-boundary gates are not selected. The full workspace suite is not required; make land supplies the affected-crate gate. Native fixture diagnostics were three dummy remote-provider Connecting/LinkDown entries and one startup status request before GUI attachment; they do not indicate a chat failure.

This record is created before its containing signed implementation commit, so commits is empty. The clean-source formal receipt, signature verification, actual make land result and containing commit SHA are recorded by the final task_report after those gates run. No operator desktop or live backend restart was used.