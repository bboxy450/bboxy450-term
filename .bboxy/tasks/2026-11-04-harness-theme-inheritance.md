---
type: task
title: Inherit parent or system theme in shared harness GUI
role: executor
parent: '[[01M6KEVVYNZW5H21RPTFW1RVDR]]'
worktree: /Users/feral/mydev/bboxy450-term-ui-status
allowed_paths:
- crates/bboxy450-harness/src/frontends/gui/**
- crates/bboxy450-harness/tests/**
- crates/bboxy450-harness/Makefile
- crates/bboxy450-harness/ATTRIBUTION.md
- crates/bboxy450-term/src/rich_pane.rs
- crates/bboxy450-term/src/rich_faces.rs
- crates/bboxy450-term/tests/harness_theme.rs
- Makefile
- ATTRIBUTION.md
- .bboxy/reports/**
- .bboxy/runs/codex-harness-theme.json
- .bboxy/tasks/2026-11-04-harness-theme-inheritance.md
- crates/bboxy450-term/src/main.rs
verify: make verify-harness-theme
acceptance:
- Embedded harness panes follow the parent Term light or dark theme and update when it changes
- Standalone harness follows current system appearance and subsequent system theme changes while honoring an explicit existing override
- Menus, loader, composer, Markdown and syntax colors use the matching palette without changing session, pending input or transcript ownership
- Independent views and repeated theme switches pass focused native paint/cache checks and affected compilation
id: 01M6KMFKYXNVYZPVN34RDNT3FB
status: claimed
delegated_by: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-04T16:41:16.874Z
created: 2026-11-04T15:57:45.223Z
number: 93377961360994
created_by: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
updated_by: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
authorship:
  signer: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
  kind: seat
  public_key: JlpXQZ4/Qy0yh5dcFR+iK7RgP5q83vzdNaLijybKm+Y=
  process_generation: '1'
assignee: seat:01M6KKME8YC7J9M3SZRA1J44AP
provenance:
  digest: sha256:abbfc8156e74002e3644fa8bc3c6cfb48fd581d47381b5290fde703d71e43431
  frontmatter_digest: sha256:acd2aa800fe7328fc01305b1361220f1fe57417ea6631913b17017838130780b
  signer: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
  sig: UOA3J3ZBXZtJI7GPK4gqtr4coV/R7YKuSPfcpLI1Pvr0XDsXHyxjh16P5xDae5rDA2u6Epjs582bjh011OPbDA==
  log: .bboxy/log/2026-11-04.jsonl#121325
---
ROLE: EXECUTOR.

This is your next scoped assignment after accepted rich-text-safety. Oracle prepared parent branch codex-harness-theme-20261104 in /Users/feral/mydev/bboxy450-term-ui-status at signed c4a375da60144c913750c9d3cd2fa339e5033eed (current delivered text crash/inline-code fixes and Reasoning label); nested harness branch codex-harness-theme-20261104 is at published e557d3641205afe475b89ee1edc67118b7ae255f (compact Python cards). Use this exact base. Execute on the signed Delegate; no new worker or Git synchronization. Parent owns all merges/pins/publication/install. Commit producer and parent code separately, without staging the parent gitlink.

User screenshot: Term is light, embedded harness remains dark. Confirmed hardcodes: harness view::context returns Context::new(Theme::Dark,Density::Cozy), window.rs initializes Dark and ignores Intent::Theme, rich.rs initializes RustHighlighter(false). Implement actual light/dark theme support end to end. Embedded parent appearance is authoritative and must update existing panes on a Term theme change; do not independently poll OS theme in embedded mode. Standalone defaults to the current system appearance and handles subsequent native ThemeChanged events. Honor any existing explicit override and the existing theme affordance without creating a new settings subsystem.

Use a per-view theme entrypoint HarnessView::set_theme(bbt_ui::Theme) and matching Preparer::set_theme; return whether changed as useful. Carry parent appearance through the existing rich-pane worker boundary, and include theme in prepared document/color/cache identity so old dark preparation cannot replace a newer light frame. No process-global mutable theme: two views must retain independent palettes. Preserve session, epoch, generation, semantic IR, launcher/MCP context, pending attachments/messages and scroll state; a color change sends no agent op or model prompt. Resolve header/menu/loader/composer/code/output colors from the existing light/dark palette and use the existing syntax highlighter's matching theme. Keep all work off UI as currently required; no duplicate rendering path.

Another producer owns attachments/composer ingestion; do not overwrite it. Commit exact own producer and parent changes separately; do not stage/change the parent gitlink, fetch/pull/merge/rebase/push/install or signal any backend. Parent lands producer then updates consumer. No OS automation, operator prompts, credential access or extra workers.

Add make verify-harness-theme with only selected native light/dark captures and real parent propagation/system-event adapter witnesses plus affected strict compile. Test independent views, dark→light→dark with delayed preparation, current and loading panes, retained gray queued tail and highlighted Python colors. No full test/parity/formal/runtime corpora or vendor calls. Use existing ownership/cache guards; pure presentation checks do not need a whole-model campaign. Two jobs and warm local target. One indexed append-only report reports/2026-11-04-harness-theme-delivery.md, signed commits and compact source/command/exit/capture hashes; evidence.criterion strings copy frontmatter.acceptance exactly. Report every limit honestly.


Handoff clarification: copying this exact indexed task record into .bboxy/tasks/2026-11-04-harness-theme-inheritance.md in your own worktree is explicitly authorized for the local report/task linkage gate. Preserve its indexed fields and signed bytes; do not edit task lifecycle or import unrelated records. A signed source checkpoint may precede the indexed report; commit the report afterward as a report-only checkpoint so metadata cannot hold up Oracle's build. Reuse existing exact-source green checks; do not rerun suites for this record-only change.


Parent main.rs is also authorized ONLY for actual parent appearance-change fanout to rich panes. It already handles WindowEvent::ThemeChanged at the current source; rich_sync is normally driven from model updates, so appearance changes must notify immediately. Leave the parallel attachment/clipboard code untouched. Existing face::term_output_full plain color is hardcoded gray; use the matching palette for readable light mode. No API or semantic IR changes are needed for color alone. Build/test inside the parent workspace so bbt-doc/bbt-render resolve to the fixed local crates, not an older registry-only standalone dependency. The producer remains a separate signed source commit; parent owns its published pin and both app installations.
