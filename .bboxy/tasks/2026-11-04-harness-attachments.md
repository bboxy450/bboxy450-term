---
type: task
title: Image attachments and workspace file autocomplete in shared harness composer
role: executor
parent: '[[01M6KEVVYNZW5H21RPTFW1RVDR]]'
worktree: /Users/feral/mydev/bboxy450-term-composer
allowed_paths:
- crates/bboxy450-harness/src/frontends/gui/view.rs
- crates/bboxy450-harness/src/frontends/gui/bridge.rs
- crates/bboxy450-harness/src/frontends/gui/model.rs
- crates/bboxy450-harness/src/frontends/gui/window.rs
- crates/bboxy450-harness/src/frontends/gui/mod.rs
- crates/bboxy450-harness/src/frontends/gui/input.rs
- crates/bboxy450-harness/src/frontends/gui/attachments.rs
- crates/bboxy450-harness/src/frontends/gui/face_rows.rs
- crates/bboxy450-harness/src/protocol/**
- crates/bboxy450-harness/src/engine/**
- crates/bboxy450-harness/src/store/**
- crates/bboxy450-harness/src/backend/acp_agent.rs
- crates/bboxy450-harness/tests/**
- crates/bboxy450-harness/verification/**
- crates/bboxy450-harness/scripts/verify-harness-attachments.py
- crates/bboxy450-harness/Makefile
- crates/bboxy450-harness/ATTRIBUTION.md
- crates/bboxy450-term/src/rich_pane.rs
- crates/bboxy450-term/src/terminal_clipboard.rs
- crates/bboxy450-term/src/main.rs
- crates/bboxy450-term/src/live.rs
- crates/bboxy450-term/src/rich_faces.rs
- crates/bboxy450-term/tests/harness_attachments.rs
- crates/bboxy450-term/tests/formal_contracts/**
- verification/contracts.toml
- verification/corpus.toml
- scripts/verify-harness-attachments.py
- Makefile
- ATTRIBUTION.md
- .bboxy/reports/**
- .bboxy/runs/codex-harness-attachments.json
- .bboxy/tasks/2026-11-04-harness-attachments.md
- crates/bboxy450-harness/src/store.rs
verify: make verify-harness-attachments
acceptance:
- Images can be selected or pasted into the shared composer and actual image content reaches the private underlying agent exactly once
- Workspace @file autocomplete supports usable selection and sends the intended real file content or supported reference
- Standalone and embedded input use the same attachment path without losing queued drafts, previous history or launcher/MCP context
- Stale intake, unsupported content and delivery failures are honest and cannot cross session or draft ownership
- Focused native clipboard/composer/Bridge/ACP witnesses and affected compilation pass without operator input or backend restart
id: 01M6KKWJVF71S4JT0NNF193EWJ
status: claimed
delegated_by: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-04T16:18:03.462Z
created: 2026-11-04T15:47:20.405Z
number: 93377961360922
created_by: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
updated_by: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
authorship:
  signer: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
  kind: seat
  public_key: JlpXQZ4/Qy0yh5dcFR+iK7RgP5q83vzdNaLijybKm+Y=
  process_generation: '1'
assignee: seat:01M6KKR5T6NMPBJH9RK6YRMKH9
provenance:
  digest: sha256:e57886c9e5b0a5060fa4d887a5a1a4b86c9eff0eb5388d058577baea45292bfe
  frontmatter_digest: sha256:62f516d4ba091232b9f8a3174bc77700176fb2fbceec1794d06e74d52ca6c928
  signer: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
  sig: wppl6xgiNgTNlbIrVg5/TKx8US5fKkyHoXDISk6t8bm8oZxZRL94yx3dPsOW0XCMewx0REE/Dp9tgcOyeM8OBw==
  log: .bboxy/log/2026-11-04.jsonl#121080
---
ROLE: EXECUTOR.

Implement image attachments and @file completion for the SAME shared HarnessView in standalone and embedded term. Own single checkout /Users/feral/mydev/bboxy450-term-composer, parent branch codex-composer-20261104 at bd820ef3558f6e0f9dbbab78d7402ef8db12e793; nested crates/bboxy450-harness branch codex-composer-20261104 at a832bd6ff4b31a36064508632c47840a58cfe56c is explicitly your scoped producer checkout. Oracle initialized both. Orient/fetch task, read instructions/source. Real bboxy450-harness worker on codex-acp. Oracle owns ALL main updates, Git integration, fetching, dependency pins, push and install. Commit your producer and host changes separately in their existing repositories, exact paths only; do not stage/update the parent's gitlink. Parent publishes producer then pins it. No waiting for operator/backend restart.

The user wants pasted/attached images and @file with autocomplete. Existing InputBlock::Image/Resource and shared composer attachments/images support already exist. Reuse them rather than text-only placeholders. Confirm the actual end-to-end path: select/paste/drop supported images, show a removable composer preview, submit real image bytes/blob/resource to the underlying ACP/model, and retain them in queued/durable history. For files, typing @ should query the current admitted workspace off the UI thread, offer usable keyboard/mouse path suggestions, handle directories/spaces/Unicode and insert/select the intended file; submit its real content/reference through the supported agent path. Do not scan unrelated homes or secrets, execute selected files, fake unsupported capabilities or add another model prompt. Empty/error/loading states are honest and stale async lookups/intake cannot alter a changed draft/session/profile. Preserve the current gray pending tail and full previous history until causal durable acknowledgement, with exactly one prompt/effect per send and attachments retained on refused/unknown delivery.

Confirmed embedding gap: crates/bboxy450-term/src/terminal_clipboard.rs:29 handles only ClipboardContent::Text when rich_face is active and returns, discarding image input. rich_pane::Input currently has Edit/Enter/Cancel/Click/Wheel/Focus/Size only. Wire image/file input through that worker-owned boundary and existing identity admission. Native standalone and embedded must reach the same shared implementation. Use existing clipboard/file-picker/drop abstractions and private injected fixture data; never access the operator's actual clipboard or require OS clicks in tests. Keep role/name/task/workspace/launcher/MCP/model/approval context unchanged. A file attachment is content, not a new permission or launcher prompt.

Parallel ownership: producer seat01M6KF1JD34QA47CQP34FRJN5Z owns tool/terminal card composition and Python/highlighting in face.rs/face_rows.rs/rich.rs. You own bridge/model/view composer-input/window integration and may change ONLY the UserMessage/image display arm of face_rows.rs if needed; do not edit tool/terminal/card functions or rich.rs. Term seat01M6KKME8YC7J9M3SZRA1J44AP owns bbt-render/bbt-doc long-text crash and inline-code fixes; leave those crates alone. Prefer new small GUI input/attachment modules. Parent handles any integration; do not recruit.

Add make verify-harness-attachments running only named private native composer/clipboard/real Bridge/ACP-process checks and affected strict compilation, two jobs/local target. Test actual image bytes and file content reaching the private ACP child once; preview/remove, @file selection, middle-turn queue, stale/delayed completion and session A→B→A, unsupported capability and unavailable files. Reuse unchanged context/pending-model evidence; if you introduce a new concurrent ownership/retry contract, add only the smallest affected finite model plus one broken guard control. No full test/parity/formal/runtime corpus or paid provider probe. Unpublished nested gitlink is deliberately unstaged; do not run unrelated clean-tree/corpus gates or claim their acceptance. Report exact producer/consumer source hashes and genuine focused exits.

Signed Boxy commits, indexed append-only report reports/2026-11-04-harness-attachments-delivery.md copied to own parent .bboxy/reports with Report trailer; compact parent run JSON, raw logs/images/fixtures outside Git unless a small intentional test fixture. task_report evidence.criterion strings must copy frontmatter.acceptance VERBATIM. No broad staging/reset/stash, other worktrees, credentials, OS automation, backend actions, merge/push/install. Quiet execution; one real blocker only with evidence and exact missing decision.


Handoff clarification: copying this exact indexed task record into .bboxy/tasks/2026-11-04-harness-attachments.md in your own worktree is explicitly authorized for the local report/task linkage gate. Preserve its indexed fields and signed bytes; do not edit task lifecycle or import unrelated records. A signed source checkpoint may precede the indexed report; commit the report afterward as a report-only checkpoint so metadata cannot hold up Oracle's build. Reuse existing exact-source green checks; do not rerun suites for this record-only change.


The exact store entrypoint crates/bboxy450-harness/src/store.rs is also authorized to wire the scoped intake module; keep changes limited to attachment ingestion. Preserve the same verification and ownership boundaries.
