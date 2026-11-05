---
type: report
title: Quiet command cards and shared Python highlighting delivery
task: '[[01M6KKT8V4RPMB0PDYQH15S5A7]]'
commits:
- 0c5b22101ee8a1dd2a1ff726c8b6e38885296fbc
verify:
  command: make verify-quiet-code-cards
  exit_code: 0
author: seat:01M6KF1JD34QA47CQP34FRJN5Z
created: 2026-11-04T16:11:06.029Z
id: 01M6KN82KX9YBKHBV8YNEEGY5Y
updated: 2026-11-04T16:11:06.029Z
number: 93377961361016
created_by: seat:01M6KF1JD34QA47CQP34FRJN5Z
updated_by: seat:01M6KF1JD34QA47CQP34FRJN5Z
authorship:
  signer: seat:01M6KF1JD34QA47CQP34FRJN5Z
  kind: seat
  public_key: El6VqSdMLzX3+r6uiKxbH22bBB8VwVCtourrWmlf8Yk=
  process_generation: '1'
provenance:
  digest: sha256:9603e6d9a9b7e1b566223fe2a97d16d6ccdaa9a911c120f2eb9ed4004697b23f
  frontmatter_digest: sha256:fe03f0d506a0704213595925d4e7dd63d99d52cf65fd3c19abfbb19f9f050f37
  signer: seat:01M6KF1JD34QA47CQP34FRJN5Z
  sig: m2EnACAhlZiRBFrRSuVnjAA6Uy17ychNZh3wKxEhb7CkjSX9mLoXU6hXpmH/ZBFtwC9VoysJMLJDAAyzI1IKCw==
  log: .bboxy/log/2026-11-04.jsonl#120907
---
Implemented in signed source commit `0c5b22101ee8a1dd2a1ff726c8b6e38885296fbc`, based on assigned clean `731ba91df0a2b7a22e914566f237e21929a53341`, branch `codex-acp-controls-20261104`, worktree `/Users/feral/mydev/bboxy450-harness-land`. `git verify-commit HEAD` exited 0 and reported the good Boxy signature with key `9B07E849480C68328CA20684844ECC0587DE7AB0`; source worktree was clean after commit. This append-only indexed report is copied byte-for-byte into the worktree and committed separately with the same Report trailer.

Files and mechanisms:
- `src/frontends/gui/face.rs:1135` — all Head verb/target/place/meta/duration strings become bounded single display lines; the verb can shrink within its card. Exact commands remain separate from this display projection.
- `src/frontends/gui/face_rows.rs:909` — Execute calls admit raw commands from arguments, terminal children, ACP names or titles. They show Run Python / Run command and one short preview; MCP headings retain their actual method name. Long/raw duplicate title captions are suppressed.
- `src/frontends/gui/face_rows.rs:1311` — shared terminal/tool command panels use 76px compact and 320px expanded viewports. Python uses the extracted code; an existing Source toggle opens the exact original shell wrapper. Existing More and two-axis scroll actions are reused. Literal mixed shell and real output remain accessible.
- `src/frontends/gui/rich.rs:29` — a conservative borrowed parser admits common literal Python heredocs and python/python3 -c forms, including Unicode and CRLF, and rejects interpolation/mixed/ambiguous shell. Parsing never starts a child or executes a command.
- `src/frontends/gui/rich.rs:349` — worker-side Python preparation reuses bbt-doc Markdown Code blocks, Highlights and the existing syntect-backed RustHighlighter. Ordinary fenced Python uses that same highlighter; no engine or dependency was added.
- `src/frontends/gui/rich.rs:121` and `src/frontends/gui/face_rows.rs:1404` — native text producers split styled runs at UTF-8 boundaries into at most 128 bytes without deleting/reordering source bytes. This bounds individual raster inputs for large MCP content; the shared backend's independent device-limit and inline-chip fix belongs to the other assigned worker.
- `tests/gui_rich.rs:1821`, `:2017`, `:2201` and `src/frontends/gui/rich.rs:936` — exact ACP name/title fixtures, literal-shell fallback, borrowed-source equality, shared Python spans/colors, Unicode chunks, compact/expanded geometry, first/last lines, internal scrolling, and a real >100KB synthetic knowledge_fetch output rendered by the pinned native Metal path.
- `scripts/verify-quiet-code-cards.py`, `Makefile:222`, `ATTRIBUTION.md` and `.bboxy/runs/codex-quiet-code-cards.json` — canonical focused gate, project credit, and receipt binding the three actual command logs, two raster captures and 81 input hashes.

Acceptance evidence:
1. ACP commands and MCP tool names use concise bounded headings without repeated raw scripts
   - `face.rs:1135`, `face_rows.rs:909`, `face_rows.rs:1018`; native `quiet_code_acp_name_title_python_and_shell_keep_bounded_accessible_source` covers multiline literal Python/git in both name and title with empty arguments, generic exec titles, mixed shell, and -c. It asserts bounded heading text/height, duration and status, and absence of repeated script bodies.
2. Unambiguous Python snippets use the shared highlighted Markdown code renderer and retain the exact original command
   - `rich.rs:29`, `:349`, `:998`; direct highlights concatenate byte-for-byte to the Python source and contain at least three colors. Fenced Markdown spans equal the command spans. The shared rendered Code element retains all displayed source bytes (the Markdown renderer omits one terminal newline for display). Original wrapper is borrowed exactly, retained in IR, and available through opt-in details; mixed shell stays one-color literal text.
3. Compact and expanded command/output panes retain full accessible content at narrow and wide sizes
   - Native geometry runs at 280/1120px and 90/100/160% zoom, checking 76/320px command bounds, source toggle visibility, Unicode first/last code lines, duration/status placement, and internal wheel behavior. The retained pane witness checks full output tail and horizontal scrolling. The MCP witness at 280/640px verifies a <=400px pane, <=128-byte visible raster inputs, exact original output in IR, and the final MCP Unicode line after scrolling.
4. Focused native realistic MCP expansion, code highlighting and geometry checks plus affected strict compile pass
   - `make verify-quiet-code-cards` exited 0. Three parser/highlight/source units and three selected native tests passed, then strict embedded library clippy exited 0. The GPU witness calls ChromeRenderer::render_to_rgba and asserts actual glyph ink, rather than relying only on semantic labels.

Final commands and actual exits:
- `make verify-quiet-code-cards` — 0. Its three exact Cargo argv arrays are in the run JSON, all offline/locked, two jobs and worktree-local target:
  - `cargo test ... --no-default-features --features embedded --lib quiet_code -- --nocapture` — 0; 3 passed, 39 filtered.
  - `cargo test ... --no-default-features --features embedded --test gui_rich quiet_code -- --nocapture` — 0; 3 passed, 22 filtered.
  - `cargo clippy ... --no-default-features --features embedded --lib --no-deps -- -D warnings` — 0.
- `rustfmt --edition 2024 --check src/frontends/gui/face.rs src/frontends/gui/face_rows.rs src/frontends/gui/rich.rs tests/gui_rich.rs` — 0.
- `git diff --check` — 0.
- Receipt/source/log/capture SHA-256 comparison script — 0.
- `git verify-commit 0c5b22101ee8a1dd2a1ff726c8b6e38885296fbc` — 0.

Evidence digests:
- Receipt `.bboxy/runs/codex-quiet-code-cards.json`: `02707cc2a5be32439d46a69b020c9419ec6f530eccd655da693d932ce4360865`.
- `target/quiet-code-cards/parser-source-and-shared-highlight.log`: `a16c60a6f72db2f794121e528b4973a29d908ef0e98518645d9e26397a0e4086`.
- `target/quiet-code-cards/native-name-title-geometry-and-mcp-raster.log`: `c34e1ff6d6fd352ada16d6f932136dffb1f0bb9c8da723c7e2f0501f4ac170a0`.
- `target/quiet-code-cards/strict-embedded-compile.log`: `2bc683fb4e4f4f21ee9ee39e2b8ecce2efe20d3b1be57bda5316aa519ab3d30a`.
- `target/quiet-code-cards/captures/mcp-expanded-280.png`: `c6b129d4c073bdcdcea60d67ef43a54858cc73abea4b4ebf4556c5a1bec40ec7` (32KiB).
- `target/quiet-code-cards/captures/mcp-expanded-640.png`: `093a5b59abf49333da6eede2f690e8dfa6788b09ff9512aa6877d5ea9afe51bd` (52KiB).

Limits and omissions:
- An exploratory sandboxed native invocation exited 101 because Metal exposed no adapter and a test initially rejected a valid short mixed-shell preview; the assertion was corrected and final native gates ran offscreen with GPU access, without operator input. Earlier exploratory logs were not retained as final proof.
- The shared Markdown Code renderer intentionally omits one terminal newline in display; the borrowed snippet and original IR/wrapper remain byte-exact. Recognition is conservative: shell expansion, compound commands, escaped/concatenated -c arguments and <<- fall back to literal shell.
- Cargo.toml/Cargo.lock and the existing bbt-doc/bbt-render `775527db16ffe33ee5cafaa0ff8566e3ea7512da` pin are unchanged. This report proves the producer bound on that pin, not the other worker's global renderer/inline-code fix or the combined installed path.
- view.rs/window.rs/bridge.rs, UserMessage/attachment ingestion, Thinking and pane-title implementations were not edited. No old ACP/formal gate, full suite, vendor probe, desktop interaction, backend/UI restart, dependency edit, merge, push, publication or installation was performed. This pure parsing/layout change adds no stateful contract or model. Earlier accepted reports and receipts remain unchanged; their results are not relabelled as fresh here.
