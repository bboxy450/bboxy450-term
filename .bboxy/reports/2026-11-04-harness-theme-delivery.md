---
type: report
title: Per-view harness appearance and authoritative parent/system theme propagation
task: '[[tasks/2026-11-04-harness-theme-inheritance]]'
commits:
- cb131433b2b9f01080b7cc676ec9df01989dbd49
- ba7e0a5b965142881da1dce1461965dfc095c419
verify:
  command: make verify-harness-theme
  exit_code: 0
author: seat:01M6KKME8YC7J9M3SZRA1J44AP
created: 2026-11-04T17:16:31.886Z
id: 01M6KRZWND4601R7STAHPA2KPD
updated: 2026-11-04T17:16:31.886Z
number: 93377961361076
created_by: seat:01M6KKME8YC7J9M3SZRA1J44AP
updated_by: seat:01M6KKME8YC7J9M3SZRA1J44AP
authorship:
  signer: seat:01M6KKME8YC7J9M3SZRA1J44AP
  kind: seat
  public_key: Z1mnWUzcr/UKeTmWT7DZT4dYe6UKknER5pCBprq8E80=
  process_generation: '1'
provenance:
  digest: sha256:05755157ab3dfec3534b46858c62d521da70ef7d679848a30a107ce504f01560
  frontmatter_digest: sha256:71a0c1145cfdfcf1617514b51ec06f4b2eb91253414f269404a01813121fb41d
  signer: seat:01M6KKME8YC7J9M3SZRA1J44AP
  sig: r4aD1r/GObpcBZcjQslTSR3tkK96+nAbrwR18NL+hpqVBCrfVe8I8B+HEj+5/ubPzUJd8w235yZmT1EE45qKDA==
  log: .bboxy/log/2026-11-04.jsonl#121721
---
# Harness theme inheritance delivery

Task: [[tasks/2026-11-04-harness-theme-inheritance]]. Executor: seat:01M6KKME8YC7J9M3SZRA1J44AP. Parent worktree /Users/feral/mydev/bboxy450-term-ui-status, branch codex-harness-theme-20261104.

Signed source checkpoints:
- Producer bboxy450-harness: cb131433b2b9f01080b7cc676ec9df01989dbd49, based on e557d3641205afe475b89ee1edc67118b7ae255f.
- Parent bboxy450-term: ba7e0a5b965142881da1dce1461965dfc095c419, based on c4a375da60144c913750c9d3cd2fa339e5033eed.

Both use Boxy <bboxy450@gmail.com>, pinned Ed25519 key 9B07E849480C68328CA20684844ECC0587DE7AB0. The parent gitlink remains e557d3641205afe475b89ee1edc67118b7ae255f in the committed tree. The local nested checkout is the separately signed cb131433 producer, so the parent deliberately reports one unstaged gitlink modification. Oracle owns producer publication and the consumer pin.

## Exact acceptance evidence

1. **Embedded harness panes follow the parent Term light or dark theme and update when it changes**
   - crates/bboxy450-term/src/main.rs:3333 — the real apply_preferences path fans the newly resolved scene appearance out immediately, including calls from WindowEvent::ThemeChanged. It does not wait for the next backend/model publication.
   - crates/bboxy450-term/src/rich_faces.rs:94 and :163 — existing panes receive that authoritative scene theme; newly connected panes receive it during rich_sync. The loading/unavailable fallback resolves the same background and foreground palette.
   - crates/bboxy450-term/src/rich_pane.rs:218 — a per-pane latest atomic appearance and notification bypass the bounded input queue. The worker updates view and bridge presentation only. Snapshot appearance revisions fence obsolete paint immediately, including coalesced A→B→A requests (:267).
   - rich_faces::verification::harness_theme_parent_preferences_fan_out_without_model_update — a private-HOME real App fixture changes preferences through the production path and observes an existing loading actor update light→dark→light without model publication, generation change, cursor ownership or queued input.
   - tests/harness_theme.rs:63 — an owned Unix socket drives the real Pane/Bridge protocol and worker. Current themed text must paint. Draft length, session/epoch/cursor, retained item count and generation survive repeated and coalesced transitions; the private peer observes zero agent operations.

2. **Standalone harness follows current system appearance and subsequent system theme changes while honoring an explicit existing override**
   - Producer src/frontends/gui/window.rs:183 — startup resolves the native window's current theme, then applies it to the view and bridge.
   - window.rs:122 and :629 — the production system-event adapter consumes WindowEvent::ThemeChanged and remembers the system appearance even while an explicit override is active. Returning to automatic mode uses the latest native appearance. Embedded mode never calls this standalone adapter.
   - window.rs:555 — the previously ignored title-bar theme action cycles automatic/light/dark and applies it. A late settings read cannot overwrite a choice already made in this window.
   - zoom.rs:150 and window.rs:823 — the existing settings.json light/dark override is read off the UI thread alongside existing zoom settings; absent/auto values follow the window. There is no new settings store.
   - tests/gui_theme.rs:295 — real winit WindowEvent values exercise the same adapter and override/cycle behavior. :315 checks existing light/dark/auto file values and byte-identical preservation of zoom/settings.

3. **Menus, loader, composer, Markdown and syntax colors use the matching palette without changing session, pending input or transcript ownership**
   - Producer view.rs:393 — HarnessView::set_theme owns a local theme/revision, invalidates color-dependent frame/control/menu caches and retains composer, scroll and transcript identity. Frame.theme is also used by the native material/composition path in window.rs and smoke.rs.
   - rich.rs:349 — Documents invalidates prepared layouts and highlighter selection on a local palette change while retaining parsed source/stream ownership. Markdown composition, reasoning ink, inline code and Python highlighting use that context and RustHighlighter's matching light/dark theme.
   - face_rows.rs — resolved context reaches code/output viewports, command panels and rich document insets. face.rs:835 resolves plain output to palette foreground, verdict ink to the current theme, and terminal/header surfaces to palette-derived colors.
   - menu.rs:161 — both menu families share the context-resolved Surface2 card; the search field tints the underlying palette instead of an opaque dark literal. Native capture review found this remaining dark-menu defect, and the final pixel assertion checks the actual light Surface2 pixels.
   - tests/gui_theme.rs:128 — the native view witness retains the exact composer draft and pending-prompt/transcript Arc owners across theme changes, preserves a nonzero scroll offset and captures the gray queued tail. Its independent light view's RGBA is byte-identical before and after another view's theme changes.
   - :267 checks Python span palette differences and dark→light→dark restoration, and requires actual plain output runs to equal palette foreground.

4. **Independent views and repeated theme switches pass focused native paint/cache checks and affected compilation**
   - view.rs:799 — prepared rows must match both theme and presentation revision as well as the existing session/epoch/cursor fence. A dark completion after light, and an earlier dark completion after a return to dark, are rejected.
   - view.rs:2743 and bridge.rs:733 — Preparer::set_theme and the bridge's captured presentation revision rebuild color-dependent rows/documents on the existing blocking preparation pool. Returning to a previous palette is a fresh revision; no process-global mutable theme is introduced.
   - make verify-harness-theme — exit 0: four selected theme tests, the existing shared Python-highlighting native witness (one test), and two real parent/worker tests, seven total. Strict Clippy (-D warnings) passes for producer lib/theme integration target and parent lib/bin/actor integration target.
   - All Cargo commands ran sequentially in the parent workspace with two jobs and its local target, so the fixed local bbt-doc/bbt-render crates were used rather than registry-only standalone dependencies.

## Commands and retained artifacts

- make verify-harness-theme — exit 0; target/harness-theme/final.log.
- make test-licenses — exit 0; six license fixtures, the 34-crate policy/notice inventory, source notices and nine notice fixtures; target/harness-theme/licenses.log. Subsequent menu edits retained the same source headers and inventory.
- git diff --check in both repositories — exit 0.
- Parent pinned public-only signature verifier over c4a375da60144c913750c9d3cd2fa339e5033eed..ba7e0a5b965142881da1dce1461965dfc095c419 — exit 0, one verified commit.
- The same verifier from the producer repository over e557d3641205afe475b89ee1edc67118b7ae255f..cb131433b2b9f01080b7cc676ec9df01989dbd49 — exit 0, one verified commit. A direct machine-keybox read was sandbox-denied; verification instead used the repository's pinned public-only verifier without a keybox retry.

.bboxy/runs/codex-harness-theme.json commits source SHA-256s for both cuts, the task-byte hash, actual gate exits/log hashes and nine native PNG hashes. Raw logs and captures remain outside Git under target/harness-theme. The 640×480 captures cover dark/current, light/current, both delayed/ABA loaders, queued tail, independent-view equality and the light profile menu. Native captures were inspected; menu and overall background colors also have pixel assertions.

Earlier selected iterations exposed a byte-array assertion compile error and a queued-tail assertion outside its virtualized viewport; both fixture issues were corrected. Capture review then exposed the actual hardcoded menu card/search surface. The final source uses a palette token and the corresponding pixel assertion passes. Producer unit-test compilation and the selected existing Python witness also pass.

## Limits and handoff state

No full test/parity/formal/runtime corpus, vendor call, installation, OS automation, operator input, backend restart, Git synchronization, merge or publication was performed. These are bounded presentation/cache/worker witnesses, not an exhaustive model or timing claim. Native system-event delivery was exercised through the real adapter with constructed winit events, not by changing the operator's OS appearance or opening a live window. The initial window.theme getter and event-handler wiring are compiled source evidence.

Existing persisted theme overrides are honored. The title-bar override cycle is window-local and does not write a new preference. Attachments/composer ingestion was not edited or separately re-ingested; draft, pending-prompt ownership and zero-op preservation are covered at the stated boundaries. Existing session/epoch/generation and semantic IR are unchanged.

The gate verified the coupled local parent source and producer cb131433 source. A checkout of the parent source checkpoint alone still carries the old recorded gitlink by instruction; Oracle must publish/land the producer and update that pin before consumer deployment. The report-only checkpoint reuses the exact green source evidence without rerunning suites.
