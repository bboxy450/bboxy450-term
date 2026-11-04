---
type: report
title: Shared image attachments, project dotfiles and retained-history pixels
task: '[[01M6KKWJVF71S4JT0NNF193EWJ]]'
commits:
- 1d060fab2ccdf8d6ca67c31c219096440fee5295
supersedes: '[[reports/2026-11-04-harness-attachments-capability]]'
verify:
  command: make verify-harness-attachments
  exit_code: 0
author: seat:01M6KKR5T6NMPBJH9RK6YRMKH9
created: 2026-11-04T17:27:57.252Z
id: 01M6KSMSBBQ5BWWY3P6XXBR1NV
updated: 2026-11-04T17:27:57.252Z
number: 93377961361081
created_by: seat:01M6KKR5T6NMPBJH9RK6YRMKH9
updated_by: seat:01M6KKR5T6NMPBJH9RK6YRMKH9
authorship:
  signer: seat:01M6KKR5T6NMPBJH9RK6YRMKH9
  kind: seat
  public_key: S3I9J2cN2HkGGxi3t4PZvVBpIllzItN0PkRADkjVqvs=
  process_generation: '1'
provenance:
  digest: sha256:bce80b58089561ace83c82fe73afc8019d0ebcdc017b96fe2b756a2349d8ba01
  frontmatter_digest: sha256:ba56c3229d5f1ba855b1e7fc8770c413b7cd28b77f9c2a905cfdd1cb5a74c048
  signer: seat:01M6KKR5T6NMPBJH9RK6YRMKH9
  sig: cMZT2VnzS3ZZnBQrkconUf2yqaCKZD7tqCgHZOADY1GJ4EcCN0WPq2zNsLKda6/GwpXKYG+dd3zn8Vkx4OIeCw==
  log: .bboxy/log/2026-11-04.jsonl#121802
---
Implemented image attachments and workspace @file completion in the shared HarnessView used by standalone and embedded term. This consolidated current-source report supersedes [[reports/2026-11-04-harness-attachments-capability]] and includes explicit actor-capability admission, project dotfile selection and the real queued image/file history pixel witness.

Verified producer snapshot: `ff6d0685aad6ed4f0796123e997dfdd66c61d6d9`. Verified consumer snapshot: `1d060fab2ccdf8d6ca67c31c219096440fee5295` (consumer source unchanged from `4a940bab010a474278d2fb5fc228b68e3cfbfa5c`). Both snapshots and the preceding implementation commits are signed as Boxy <bboxy450@gmail.com> with key `9B07E849480C68328CA20684844ECC0587DE7AB0`. The parent gitlink remains deliberately unstaged; Oracle owns publication and pinning.

- Producer `src/frontends/gui/input.rs:125,302`, `attachments.rs:206`, `view.rs:1815,2053` and `window.rs` — asynchronous workspace completion and image/file intake feed one shared composer, removable image previews and actual InputBlock content. Native paste, picker and drop routes use that implementation. Session/epoch, monotonic local birth and draft revision guard delayed completion/intake, including A → B → A.
- Producer `src/store/intake.rs`, `src/store.rs`, `src/backend/acp_agent.rs` and `src/engine/mod.rs` — private same-uid bounded image custody crosses the socket as a checked resource; the actor admits exact bytes into a durable session blob before queueing. Binary storage preserves bytes. Both actor implementations associate the durable initial user item with its original operation cause.
- Producer `src/frontends/gui/bridge.rs:694,709,808,970`, `view.rs:2053`, `src/protocol/mod.rs:71,184`, `src/engine/mod.rs:507` and `src/backend/acp_agent.rs:498` — a fresh session/epoch/connection challenge must receive the explicit live version-1 image-intake response. A bare ACK, durable replay, stale epoch/challenge or old reconnect response cannot grant support. Both the view and direct Bridge guard before pending registration or custody transfer. An old actor receives zero unsupported image UserTurns; the original image draft and custody remain with an honest notice. This negotiation creates no model prompt or permission grant.
- Producer `src/frontends/gui/bridge.rs`, `input.rs` and `view.rs` — real blocks and custody remain attached to each pending operation until causal durable acknowledgement. Unknown delivery retains custody without automatic resend; refused delivery restores the draft. Later independent drafts still flow while an earlier turn is busy.
- Consumer `crates/bboxy450-term/src/terminal_clipboard.rs`, `rich_faces.rs`, `rich_pane.rs` and `main.rs` — both native paste entry points, PNG/TIFF snapshots, dropped files, completion keys and thumbnail installation feed the shared worker with existing identity admission. The private native witness uses injected clipboard values and its own actor/socket; it never reads the general clipboard.
- Producer `src/frontends/gui/attachments.rs:34,62,124` — explicit `@.gitignore` and `@.bboxy/tasks/` paths now work; blanket hidden-component rejection is removed. Canonical workspace containment and Git/private-credential component refusal remain. Autocomplete lists one directory, includes hidden names only for an explicit dot prefix, excludes named dependency/build bulk, and filters project Git ignore rules with read-only NUL-delimited check-ignore on the intake worker. System/global Git configuration and global excludes are disabled. Listings remain bounded to 4096 entries and 64 suggestions.
- Producer `tests/harness_attachments.rs:652` — a private Git workspace completes and submits .gitignore plus a Unicode/space .bboxy task filename. The actual file contents arrive in one private ACP prompt. Git internals, target/node_modules and a project-ignored generated file do not appear as suggestions.
- Producer `tests/harness_attachments.rs:720` and `tests/fixtures/acp-agent/attachments.py` — a real private ACP child produces completed prior user/assistant history, holds its second turn behind an owned file gate, then accepts a queued real PNG plus text file. The Bridge has real image-resource/file blocks while the child still has only two prompts. During prepared=None with a fresh transcript allocation, the native offscreen renderer compares both prior user/assistant RGBA regions exactly with their baseline; each baseline has more than eight distinct colors. The gray pending node and both prior rows are present. Releasing the private gate delivers exactly the third prompt and clears the attachment pending state. This is a pixel assertion on the actual Bridge/ACP path, not a text/layout-only substitute.
- Tool/terminal/card functions, rich.rs, bbt-render and bbt-doc were not changed. Launcher, role/name/task/workspace/MCP/model/approval context remains on the existing path; attached content never becomes a launcher/profile command.

`make verify-harness-attachments` exited **0** unattended on stable current source. Sixteen named tests passed: nine composer/process cases, three Bridge checks, one reconnect/detach regression, one finite-model export/control case, one embedded native-worker witness and one private macOS clipboard witness. Producer and consumer strict Clippy, plus embedded-only compilation, all exited 0. Commands, actual exits, raw-log hashes, pinned tools and all 53 source hashes are in `.bboxy/runs/codex-harness-attachments.json`. Every source hash was independently compared with current files and the signed producer/consumer snapshot bytes; whitespace checks also exited 0. No source changed during the gate.

| Check | Actual result |
|---|---|
| Producer standalone-feature harness_attachments | 9 passed; exit 0 |
| Causal pending, unknown custody and live capability proof | 3 passed; exit 0 |
| Existing stale-dispatch/reconnect/detach | 1 passed; exit 0 |
| Named finite-model export/control | 1 passed; exit 0 |
| Producer strict library/named-test Clippy and embedded-only check | both exit 0 |
| Consumer native clipboard/drop/worker/Unix Bridge | 1 passed; exit 0 |
| Private native PNG/TIFF/text pasteboard | 1 passed; exit 0 |
| Consumer strict library/app/named-test Clippy | exit 0 |

| Finite contract | States / transitions | Native and pinned Java TLC |
|---|---|---|
| HarnessAttachmentIntake | 36 / 66 | pass; TLC exit 0; full graphs equal |
| BrokenHarnessAttachmentIntake | 65 / 116 | expected safety violation; TLC exit 12; full graphs equal |
| HarnessAttachmentCapability | 16 / 47 | pass; TLC exit 0; full graphs equal |
| BrokenHarnessAttachmentCapability | 32 / 114 | expected safety violation; TLC exit 12; full graphs equal |

The ownership model abstracts one captured completion, two draft revisions and three view births; weak fairness of completion ensures settlement, including stale discard. The capability model abstracts three connection births, including A/B/A and same-session reconnect. An old actor can ACK without advertising indefinitely; no eventual-support fairness is assumed. Reconnect revokes support, only a matching live response restores it, and text delivery remains enabled. Each broken control removes its affected guard. Filesystem/decoder behavior, actual nonces, socket framing, GPU pixels, durable blobs and ACP transport remain implementation witnesses; this is not an arbitrary Rust refinement claim. Existing pending/context models are unchanged.

Producer source-manifest SHA256: `66a40dcfd522007c64de1389ec36bb2e6cd3ec8a0fc0b8abc5630769a49defa1`. Consumer: `618b67952413c151e1a7da048d9700fd1fa516a568fc638fce681bdca9e8e400`. Each hashes sorted compact JSON of repository-relative path → SHA256. The receipt distinguishes pre-commit check heads from the final signed source snapshots.

Private captures (raw bytes remain outside Git):
- `target/harness-attachments/captures/prior-history-baseline.png`: `2741cd8be1c7c163e0e9f059c5f28b0e982fa5e5dfdaa345df9a5c5a250f52e6`.
- `target/harness-attachments/captures/queued-image-file-history.png`: `ba4181c8c5ee58c4284e2e397f1c198590c67f2df55e50d9ab2f2b02896038c1`.
- Top-level raw gate log: `/tmp/composer-dotfiles-pixels-gate.log`; detailed raw logs/models under `target/harness-attachments/`.

| Acceptance criterion (verbatim) | Current evidence |
|---|---|
| Images can be selected or pasted into the shared composer and actual image content reaches the private underlying agent exactly once | tests/harness_attachments.rs:181 — real InProc/Unix Bridge to ACP child, exact decoded PNG bytes, removable preview, one prompt per send and durable blob; :501 proves zero unsupported old-actor turns. Native worker also checks stored original bytes. |
| Workspace @file autocomplete supports usable selection and sends the intended real file content or supported reference | tests/harness_attachments.rs:181 and :652 — keyboard directory/mouse filename selection with spaces/Unicode plus explicit project dotfiles; actual selected contents in one ACP prompt; workspace escape, Git internals, ignored bulk and unavailable-file cases. |
| Standalone and embedded input use the same attachment path without losing queued drafts, previous history or launcher/MCP context | tests/harness_attachments.rs:416, :467, :720; consumer tests/harness_attachments.rs:57 — shared input, busy-turn/concurrent drafts, exact prior-history pixels during real queued image/file preparation gap, causal acknowledgement and once-only bootstrap/content-as-data. |
| Stale intake, unsupported content and delivery failures are honest and cannot cross session or draft ownership | tests/harness_attachments.rs:318, :353, :395, :501; Bridge unknown-custody/current-proof checks; positive ownership/capability models and broken controls. A/B/A, replay/epoch/challenge/reconnect, missing files, refused/unknown delivery and old actors are covered. |
| Focused native clipboard/composer/Bridge/ACP witnesses and affected compilation pass without operator input or backend restart | Make exit 0; all 16 named tests, strict compilation, four native/Java graph/verdict comparisons and 53 committed-byte source comparisons pass. No operator input or shared-backend action. |

Limits: PNG up to 16 MiB / 32 megapixels, native TIFF clipboard conversion, UTF-8 text files up to 64 KiB, combined JSON input below 200 KiB, eight attachments per draft and 64 outstanding local attachment deliveries. Non-PNG binary files, credential/Git-internal paths and workspace escapes are refused. Non-Git workspaces retain bounded directory lookup and named bulk exclusions; Git-based filtering requires Git/project metadata. Explicit file intake may select a project-ignored ordinary file; automatic suggestions exclude it. Custody is local same-uid; no new crash/restart durability claim is made. The old actor is a private absent-capability wire fixture, not a live old-binary/provider probe. Unknown delivery uses the private Bridge witness.

Skipped as instructed: full test/parity/formal/runtime corpora, paid/live providers, operator clipboard, desktop automation/capture, backend restart, other worktrees, dependency pins, merge, push and install. No raw logs/images or credentials enter Git. Producer is clean; after the report/receipt/task-mirror checkpoint the consumer retains only its deliberately unstaged producer gitlink. The latest exact signed task mirror is copied locally without altering lifecycle fields; earlier reports remain append-only.