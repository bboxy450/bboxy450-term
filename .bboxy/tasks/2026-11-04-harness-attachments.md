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
status: accepted
delegated_by: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-04T17:36:17.092Z
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
artifacts:
- '[[01M6KSPGYCXHKQGAD1C2NW34D0]]'
verify_result:
  command: make verify-harness-attachments
  exit_code: 0
  evidence:
  - criterion: Images can be selected or pasted into the shared composer and actual image content reaches the private underlying agent exactly once
    reference: crates/bboxy450-harness/tests/harness_attachments.rs:181 — real InProc/Unix Bridge to private ACP child; exact decoded PNG, one prompt per send, removable preview and durable blob. :501 — old actor gets zero unsupported image UserTurns; custody retained. composer-acp.log — 9 passed.
  - criterion: Workspace @file autocomplete supports usable selection and sends the intended real file content or supported reference
    reference: crates/bboxy450-harness/tests/harness_attachments.rs:181, :652 — keyboard/mouse directory and Unicode/space file selection plus explicit .gitignore/.bboxy/tasks content in one real ACP prompt; Git internals and project-ignored/generated bulk excluded. attachments.rs:34, :62, :124 — canonical workspace and bounded worker lookup.
  - criterion: Standalone and embedded input use the same attachment path without losing queued drafts, previous history or launcher/MCP context
    reference: crates/bboxy450-harness/tests/harness_attachments.rs:416, :467, :720; crates/bboxy450-term/tests/harness_attachments.rs:57 — shared input, concurrent/busy-turn drafts, once-only bootstrap and content-as-data. Actual private ACP queued image/file witness compares both prior user/assistant native RGBA regions exactly during prepared=None; captures and hashes in receipt.
  - criterion: Stale intake, unsupported content and delivery failures are honest and cannot cross session or draft ownership
    reference: crates/bboxy450-harness/tests/harness_attachments.rs:318, :353, :395, :501; bridge.rs:694, :709, :808, :970 — A/B/A/draft and live session/epoch/challenge guards, old-actor refusal, unknown custody retention. Native/TLC ownership 36/66 and capability 16/47 graphs pass; broken controls 65/116 and 32/114 fail safety as expected.
  - criterion: Focused native clipboard/composer/Bridge/ACP witnesses and affected compilation pass without operator input or backend restart
    reference: '.bboxy/runs/codex-harness-attachments.json — make verify-harness-attachments exit 0, 16 named tests, producer/consumer strict compilation, embedded-only check, four full native/Java graph-verdict comparisons, all 53 source hashes matching signed snapshots. Consolidated report: reports/2026-11-04-harness-attachments-dotfiles-history.md. No operator input or shared-backend action.'
report:
  summary: 'Implemented shared image attachments and workspace @file delivery with real ACP bytes/content, explicit current-actor capability admission, retained custody/queued drafts and stale ownership guards. Explicit .gitignore and Unicode/space .bboxy task selections now work while Git internals and ignored bulk stay out of autocomplete. A private held ACP turn proves exact native prior-user/assistant pixel equality during a queued image/file preparation gap. make verify-harness-attachments exited 0: all 16 named tests, strict compilation and four native/Java model/control graph-verdict comparisons. All 53 source hashes match signed producer ff6d0685aad6ed4f0796123e997dfdd66c61d6d9 and unchanged consumer snapshot 1d060fab2ccdf8d6ca67c31c219096440fee5295. Consolidated indexed report, receipt and exact amended task mirror are signed in 266b7f28a785ef4c268c8ab2cdd8867160bd804f. Producer clean; parent gitlink deliberately unstaged; no backend, pin, push or install action.'
  commit: 266b7f28a785ef4c268c8ab2cdd8867160bd804f
  reported_by: seat:01M6KKR5T6NMPBJH9RK6YRMKH9
  op_id: 01M6KSPPJV594ZFYYQQGHT1SD9
  verify_revision: 0
  report: '[[reports/2026-11-04-harness-attachments-dotfiles-history.md]]'
provenance:
  digest: sha256:b6c001b87f05b151e99d2d6bfdc06e54e3bf1bcd3e5879075cd16c6ccfdd39d8
  frontmatter_digest: sha256:212f23ee422f9e71e9a510dc78c6e5312495f06d9f46e5d8c567ba880fd04fd9
  signer: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
  sig: RhHEZXlc35IyQ6PfaG+dr2J4WMQS3uhaSSwYUgemyYzCkpSvR8jqQp4NzwaSe1HP0dMaWqcMRv2tWvn802mbCw==
  log: .bboxy/log/2026-11-04.jsonl#121857
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


## Report from seat:01M6KKR5T6NMPBJH9RK6YRMKH9

Implemented shared image attachment intake and workspace @file completion with real ACP image bytes/file contents, removable previews, queue retention and stale ownership/capability guards. make verify-harness-attachments exited 0: 6 composer/ACP cases, 2 pending/custody checks, native worker and private clipboard witnesses, strict compilation, plus native/Java graph and verdict parity for the 36-state positive model and 65-state broken control. All 52 source hashes match signed producer 958f1d14b40c4b83e50cf405c58efd126b73c976 and consumer source 623fff70c62e7838d56cf10a90fff7c6b27a8a23; signed record checkpoint 94222c5f19b2fdec8659f2a8c1d76fa5f459428d contains the indexed report and receipt. Producer is clean; only the deliberately unstaged parent gitlink remains. No push, pin, install or backend action.

```json
{
  "op_id": "01M6KQ48M59VS58WPZMBS6RG3E",
  "commit": "94222c5f19b2fdec8659f2a8c1d76fa5f459428d",
  "report": "[[reports/2026-11-04-harness-attachments-delivery.md]]",
  "verify_result": {
    "command": "make verify-harness-attachments",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Images can be selected or pasted into the shared composer and actual image content reaches the private underlying agent exactly once",
        "reference": "crates/bboxy450-harness/tests/harness_attachments.rs:169 — real InProc/Unix Bridge to private ACP child, exact decoded PNG bytes, one prompt per send, removable preview and durable blob; target/harness-attachments/composer-acp.log — 6 passed."
      },
      {
        "criterion": "Workspace @file autocomplete supports usable selection and sends the intended real file content or supported reference",
        "reference": "crates/bboxy450-harness/tests/harness_attachments.rs:169 — keyboard directory and mouse Unicode/space file selection, actual file contents in the same ACP prompt; src/frontends/gui/attachments.rs — workspace containment and unavailable/hidden-path refusal."
      },
      {
        "criterion": "Standalone and embedded input use the same attachment path without losing queued drafts, previous history or launcher/MCP context",
        "reference": "crates/bboxy450-harness/tests/harness_attachments.rs:169, :404, :455; crates/bboxy450-term/tests/harness_attachments.rs:57 — shared view, concurrent pastes, middle-turn queue, independent later sends, once-only bootstrap and attachment content never becoming a launcher/profile command."
      },
      {
        "criterion": "Stale intake, unsupported content and delivery failures are honest and cannot cross session or draft ownership",
        "reference": "crates/bboxy450-harness/tests/harness_attachments.rs — stale draft/A-B-A, unsupported capability, unavailable file and closed Bridge cases; bridge.rs:1885 — unknown outcome retains real blocks/custody; HarnessAttachmentIntake 36 states/66 edges passes, broken control 65 states/116 edges fails safety identically in native/Java TLC."
      },
      {
        "criterion": "Focused native clipboard/composer/Bridge/ACP witnesses and affected compilation pass without operator input or backend restart",
        "reference": ".bboxy/runs/codex-harness-attachments.json — make verify-harness-attachments exit 0; producer/consumer strict compile, named native private clipboard/worker tests, all 52 source hashes matching signed source commits. Report: [[reports/2026-11-04-harness-attachments-delivery]]."
      }
    ]
  }
}
```


## Report from seat:01M6KKR5T6NMPBJH9RK6YRMKH9

Added explicit current-actor image-intake negotiation: fresh session/epoch/connection challenge, live versioned response, and pre-send guards in the shared view and Bridge. A private old actor acknowledges the query but receives zero unsupported UserTurns; image draft and custody remain intact. Replayed/stale capability proof is rejected. Focused Make verification exited 0, including 7 composer/process cases, 3 Bridge checks, the existing reconnect regression, native/private clipboard witnesses, strict compilation and four native/Java model/control graph-verdict comparisons. All 53 source hashes match signed producer f2022dee24c344c5eee487a7a1dff28ae5ad7c42 and consumer 4a940bab010a474278d2fb5fc228b68e3cfbfa5c; superseding report/receipt committed as 1d060fab2ccdf8d6ca67c31c219096440fee5295. Producer is clean, parent gitlink remains unstaged, and the shared backend was not touched.

```json
{
  "op_id": "01M6KRJWZFSESR48MF3P6SMTRQ",
  "commit": "1d060fab2ccdf8d6ca67c31c219096440fee5295",
  "report": "[[reports/2026-11-04-harness-attachments-capability.md]]",
  "verify_result": {
    "command": "make verify-harness-attachments",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Images can be selected or pasted into the shared composer and actual image content reaches the private underlying agent exactly once",
        "reference": "crates/bboxy450-harness/tests/harness_attachments.rs:490 — private old-wire actor acknowledges only the capability query; zero unsupported UserTurns; draft/attachment unchanged and custody bytes retained. Original real ACP exact-byte/one-prompt cases rerun green."
      },
      {
        "criterion": "Workspace @file autocomplete supports usable selection and sends the intended real file content or supported reference",
        "reference": "crates/bboxy450-harness/tests/harness_attachments.rs — original keyboard/mouse directory, space and Unicode file selection with real file content rerun green; image-only capability guard leaves text/file delivery unchanged."
      },
      {
        "criterion": "Standalone and embedded input use the same attachment path without losing queued drafts, previous history or launcher/MCP context",
        "reference": "crates/bboxy450-term/tests/harness_attachments.rs — native shared worker awaits the actual current actor proof; original real ACP, queue/concurrent paste, once-only bootstrap and content-as-data cases remain green; tests/gui_bridge.rs:487 preserves stale-reconnect/detach behavior."
      },
      {
        "criterion": "Stale intake, unsupported content and delivery failures are honest and cannot cross session or draft ownership",
        "reference": "crates/bboxy450-harness/src/frontends/gui/bridge.rs:694, :716, :808, :2016 and view.rs:2053 — image gate precedes pending registration and custody transfer; bare ACK, durable replay, stale epoch/challenge/reconnect proof cannot grant support. HarnessAttachmentCapability positive/control graphs match native/Java: 16/47 and 32/114 states/transitions."
      },
      {
        "criterion": "Focused native clipboard/composer/Bridge/ACP witnesses and affected compilation pass without operator input or backend restart",
        "reference": ".bboxy/runs/codex-harness-attachments.json — make verify-harness-attachments exit 0; 7 composer/process cases, 3 Bridge checks, 1 reconnect case, finite-model export, native worker/private clipboard, strict compilation, four native/Java graph/verdict comparisons, and all 53 hashes matching signed source commits."
      }
    ]
  }
}
```


Final usability requirements within the existing file-completion and history criteria: explicitly typed project dotfile paths such as .gitignore and .bboxy/tasks must work. Remove the blanket starts-with-dot rejection; keep .git internals and normal ignored/bulk directories excluded from automatic lookup. Add only a named explicit-dotfile case. The mid-turn queued image/file witness must include an actual prior-history pixel assertion, not merely item_text/layout labels. Reuse unchanged tests and selected model evidence; run only these missing cases and affected checks. Finish with one consolidated fresh signed report covering the current actor-capability guard and these two cases, linked to this same task; do not reuse the earlier reported snapshot as the final changed-source result.


## Report from seat:01M6KKR5T6NMPBJH9RK6YRMKH9

Implemented shared image attachments and workspace @file delivery with real ACP bytes/content, explicit current-actor capability admission, retained custody/queued drafts and stale ownership guards. Explicit .gitignore and Unicode/space .bboxy task selections now work while Git internals and ignored bulk stay out of autocomplete. A private held ACP turn proves exact native prior-user/assistant pixel equality during a queued image/file preparation gap. make verify-harness-attachments exited 0: all 16 named tests, strict compilation and four native/Java model/control graph-verdict comparisons. All 53 source hashes match signed producer ff6d0685aad6ed4f0796123e997dfdd66c61d6d9 and unchanged consumer snapshot 1d060fab2ccdf8d6ca67c31c219096440fee5295. Consolidated indexed report, receipt and exact amended task mirror are signed in 266b7f28a785ef4c268c8ab2cdd8867160bd804f. Producer clean; parent gitlink deliberately unstaged; no backend, pin, push or install action.

```json
{
  "op_id": "01M6KSPPJV594ZFYYQQGHT1SD9",
  "commit": "266b7f28a785ef4c268c8ab2cdd8867160bd804f",
  "report": "[[reports/2026-11-04-harness-attachments-dotfiles-history.md]]",
  "verify_result": {
    "command": "make verify-harness-attachments",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Images can be selected or pasted into the shared composer and actual image content reaches the private underlying agent exactly once",
        "reference": "crates/bboxy450-harness/tests/harness_attachments.rs:181 — real InProc/Unix Bridge to private ACP child; exact decoded PNG, one prompt per send, removable preview and durable blob. :501 — old actor gets zero unsupported image UserTurns; custody retained. composer-acp.log — 9 passed."
      },
      {
        "criterion": "Workspace @file autocomplete supports usable selection and sends the intended real file content or supported reference",
        "reference": "crates/bboxy450-harness/tests/harness_attachments.rs:181, :652 — keyboard/mouse directory and Unicode/space file selection plus explicit .gitignore/.bboxy/tasks content in one real ACP prompt; Git internals and project-ignored/generated bulk excluded. attachments.rs:34, :62, :124 — canonical workspace and bounded worker lookup."
      },
      {
        "criterion": "Standalone and embedded input use the same attachment path without losing queued drafts, previous history or launcher/MCP context",
        "reference": "crates/bboxy450-harness/tests/harness_attachments.rs:416, :467, :720; crates/bboxy450-term/tests/harness_attachments.rs:57 — shared input, concurrent/busy-turn drafts, once-only bootstrap and content-as-data. Actual private ACP queued image/file witness compares both prior user/assistant native RGBA regions exactly during prepared=None; captures and hashes in receipt."
      },
      {
        "criterion": "Stale intake, unsupported content and delivery failures are honest and cannot cross session or draft ownership",
        "reference": "crates/bboxy450-harness/tests/harness_attachments.rs:318, :353, :395, :501; bridge.rs:694, :709, :808, :970 — A/B/A/draft and live session/epoch/challenge guards, old-actor refusal, unknown custody retention. Native/TLC ownership 36/66 and capability 16/47 graphs pass; broken controls 65/116 and 32/114 fail safety as expected."
      },
      {
        "criterion": "Focused native clipboard/composer/Bridge/ACP witnesses and affected compilation pass without operator input or backend restart",
        "reference": ".bboxy/runs/codex-harness-attachments.json — make verify-harness-attachments exit 0, 16 named tests, producer/consumer strict compilation, embedded-only check, four full native/Java graph-verdict comparisons, all 53 source hashes matching signed snapshots. Consolidated report: reports/2026-11-04-harness-attachments-dotfiles-history.md. No operator input or shared-backend action."
      }
    ]
  }
}
```
