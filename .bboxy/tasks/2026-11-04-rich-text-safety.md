---
type: task
title: Keep expanded MCP text safe and fix inline-code paint artifacts
role: executor
parent: '[[01M6KEVVYNZW5H21RPTFW1RVDR]]'
worktree: /Users/feral/mydev/bboxy450-term-ui-status
allowed_paths:
- crates/bbt-render/src/chrome/**
- crates/bbt-render/tests/**
- crates/bbt-doc/src/markdown/**
- crates/bbt-doc/src/highlight.rs
- crates/bbt-doc/tests/**
- crates/bbt-ui/src/components/**
- crates/bboxy450-term/tests/rich_text_safety.rs
- Makefile
- .bboxy/reports/**
- .bboxy/runs/codex-rich-text-safety.json
- .bboxy/tasks/2026-11-04-rich-text-safety.md
verify: make verify-rich-text-safety
acceptance:
- Expanding a large MCP result no longer exits the UI or exceeds bounded native text rasterization
- Long text and Unicode content remain accessible with correct scrolling, clipping and copy semantics
- Inline Markdown code has aligned text and bounded background without the reported stray artifact
- Selected native reproduction, geometry/paint checks and affected strict compilation pass without live input or backend restart
id: 01M6KKN7FZ8JN8EX1S43J5DVSR
status: accepted
delegated_by: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-04T16:25:44.803Z
created: 2026-11-04T15:43:19.492Z
number: 93377961360918
created_by: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
updated_by: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
authorship:
  signer: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
  kind: seat
  public_key: JlpXQZ4/Qy0yh5dcFR+iK7RgP5q83vzdNaLijybKm+Y=
  process_generation: '1'
assignee: seat:01M6KKME8YC7J9M3SZRA1J44AP
artifacts:
- '[[01M6KN9076RNEEQ4AMYVFY8G7M]]'
verify_result:
  command: make verify-rich-text-safety
  exit_code: 0
  evidence:
  - criterion: Expanding a large MCP result no longer exits the UI or exceeds bounded native text rasterization
    reference: crates/bboxy450-term/tests/rich_text_safety.rs:95 — private production MCP tool disclosure renders full 6039-byte result; line-guard-control.log fails with chrome text line too large; final.log passes.
  - criterion: Long text and Unicode content remain accessible with correct scrolling, clipping and copy semantics
    reference: crates/bbt-render/tests/chrome_long_lines.rs:16 — native Unicode tail pixel equality at 4 scales × 4 phases, clipped horizontal origins and cache budgets; crates/bbt-doc/tests/inline_code.rs:44 exact positioned-run source bytes.
  - criterion: Inline Markdown code has aligned text and bounded background without the reported stray artifact
    reference: crates/bboxy450-term/tests/rich_text_safety.rs:120 — widths 240/640, zoom 90/100/160%, scales 1/2; isolated paint asserts no pixels outside capsule; PNGs and hashes retained.
  - criterion: Selected native reproduction, geometry/paint checks and affected strict compilation pass without live input or backend restart
    reference: .bboxy/runs/codex-rich-text-safety.json — exact source hashes, make verify-rich-text-safety exit 0, 9 selected tests, strict affected Clippy, make test-licenses exit 0; indexed report [[reports/2026-11-04-rich-text-safety-delivery]].
report:
  summary: 'Delivered signed Boxy commit b0c78f2b2692397738c19a02e4f1961d6918ce72 on codex-text-rendering-20261104; worktree clean. Full native line extents now use the existing bounded glyph/component rasterizer, with exact overlap grouping via an x sweep; inline Markdown code uses measured shared-toolkit capsules with real padding/baselines and zoom reflow. make verify-rich-text-safety exited 0: nine selected tests plus affected strict Clippy, using exact a832bd6 producer pin and a private 6,039-byte MCP disclosure/Metal witness. Old-guard and no-capsule controls failed as required, exact source restored; make test-licenses and pinned one-commit signature verification exited 0. Indexed task/report bytes and committed source hashes match; captures/log hashes are in .bboxy/runs/codex-rich-text-safety.json. No merge, push, install, live input or backend restart.'
  commit: b0c78f2b2692397738c19a02e4f1961d6918ce72
  reported_by: seat:01M6KKME8YC7J9M3SZRA1J44AP
  op_id: 01M6KND3PPZW2JSCKJ324JJB0T
  verify_revision: 0
  report: '[[reports/2026-11-04-rich-text-safety-delivery.md]]'
provenance:
  digest: sha256:ee484218f4765d4b897925b40536c65a3bfe8cfe46d31b5d2c042433df1cfd93
  frontmatter_digest: sha256:b7d20299c3ffc054f50f90a4b01cf311db3a0ea4503d93cffe7dc2366c503d73
  signer: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
  sig: CW+FH7t+usYjmW0FFc8vZa/i1zUBB0OqfWlOO5W1ICNQ9huhEqKuDQF3EjMswbQwFZ22qfRdR2taDJlqn9urAA==
  log: .bboxy/log/2026-11-04.jsonl#121118
---
ROLE: EXECUTOR.

Fix the operator's live GUI crash FIRST, plus the persistent inline-backtick code artifact, in /Users/feral/mydev/bboxy450-term-ui-status, branch codex-text-rendering-20261104, clean signed base bd820ef3558f6e0f9dbbab78d7402ef8db12e793. You are a real bboxy450-harness worker on codex-acp; orient and fetch this signed task. Oracle owns all fetch/pull/merge/pin/publication/install work. No backend restart, live session effects, OS input or desktop capture.

Real reproduction evidence: after opening an embedded mcp.bboxy.knowledge_fetch tool card, the UI exited. Oracle-started GUI98770 log /Users/feral/mydev/bboxy450-term-codex-rich/target/recovery-restore/release/gui-reopen-efa21w_f/gui.log ends 'app error: chrome text line too large', 'cockpit UI detached; backend owners kept alive'. Live supervisor49475 and Logic49477 survived. Root cause boundary is crates/bbt-render/src/chrome/text.rs:386: shape_at raster canvas width>16384 or height>2048 returns that error. Native harness code/output now uses horizontally scrollable full literal text; a large MCP JSON/text line exceeds this limit despite a narrow viewport. Do not require the operator to click or restart. Reproduce in an owned private native fixture through expansion/rendering with a long unbroken MCP result, not just a string parser. Fix bounded rasterization/visible clipping or chunked painting so the real text remains accessible through scrolling/copy and cannot terminate the UI. Preserve exact Unicode/grapheme shaping, scale/phase, clipping, text extents and cache limits. Do not merely increase the canvas bound, drop all output, suppress all rendering errors or invent a second renderer.

The user also still sees a gray/backdrop artifact around inline `make verify-acp-controls` inside normal Markdown prose. Reproduce native inline-code layouts/paint at narrow/wide widths and scale/zoom, adjacent punctuation and wrapping, and correct background/padding/baseline/clipping so the small code capsule and text align without stray boxes. Use existing bbt-doc Markdown/highlighting and shared toolkit; preserve attribution. This covers rendering geometry only. Another worker owns harness card composition and Python/heredoc presentation; another will own attachments/composer. Do not edit their producer files or APIs.

Add make verify-rich-text-safety with only meaningful selected native shaping/paint/inline-code tests, a private real rich-pane expansion/render witness where possible, affected strict compile, and a before/after failure assertion. Warm worktree-local target, two Cargo jobs. No whole make test/parity/formal/runtime corpora, no vendor calls. Pure shaping/layout changes need no new stateful model campaign. Prove long-line tails remain readable/paintable under horizontal scroll and that unchanged normal text is still correct; retain small native captures under target. Raw logs outside Git; compact hashes/exits JSON only in .bboxy/runs.

Signed Boxy source and one append-only indexed report reports/2026-11-04-rich-text-safety-delivery.md with exact command/exits/limits and evidence for the literal acceptance strings. Copy indexed bytes to own report, signed commit with Report trailer, task_report. No broad staging/reset/stash/credential access, other worktrees, new workers, merges, push or install. Report a real blocker once with the exact decision needed; otherwise implement quietly. The operator explicitly wants speed and only needed checks.


Handoff clarification: copying this exact indexed task record into .bboxy/tasks/2026-11-04-rich-text-safety.md in your own worktree is explicitly authorized for the local report/task linkage gate. Preserve its indexed fields and signed bytes; do not edit task lifecycle or import unrelated records. A signed source checkpoint may precede the indexed report; commit the report afterward as a report-only checkpoint so metadata cannot hold up Oracle's build. Reuse existing exact-source green checks; do not rerun suites for this record-only change.


## Report from seat:01M6KKME8YC7J9M3SZRA1J44AP

Delivered signed Boxy commit b0c78f2b2692397738c19a02e4f1961d6918ce72 on codex-text-rendering-20261104; worktree clean. Full native line extents now use the existing bounded glyph/component rasterizer, with exact overlap grouping via an x sweep; inline Markdown code uses measured shared-toolkit capsules with real padding/baselines and zoom reflow. make verify-rich-text-safety exited 0: nine selected tests plus affected strict Clippy, using exact a832bd6 producer pin and a private 6,039-byte MCP disclosure/Metal witness. Old-guard and no-capsule controls failed as required, exact source restored; make test-licenses and pinned one-commit signature verification exited 0. Indexed task/report bytes and committed source hashes match; captures/log hashes are in .bboxy/runs/codex-rich-text-safety.json. No merge, push, install, live input or backend restart.

```json
{
  "op_id": "01M6KND3PPZW2JSCKJ324JJB0T",
  "commit": "b0c78f2b2692397738c19a02e4f1961d6918ce72",
  "report": "[[reports/2026-11-04-rich-text-safety-delivery.md]]",
  "verify_result": {
    "command": "make verify-rich-text-safety",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Expanding a large MCP result no longer exits the UI or exceeds bounded native text rasterization",
        "reference": "crates/bboxy450-term/tests/rich_text_safety.rs:95 — private production MCP tool disclosure renders full 6039-byte result; line-guard-control.log fails with chrome text line too large; final.log passes."
      },
      {
        "criterion": "Long text and Unicode content remain accessible with correct scrolling, clipping and copy semantics",
        "reference": "crates/bbt-render/tests/chrome_long_lines.rs:16 — native Unicode tail pixel equality at 4 scales × 4 phases, clipped horizontal origins and cache budgets; crates/bbt-doc/tests/inline_code.rs:44 exact positioned-run source bytes."
      },
      {
        "criterion": "Inline Markdown code has aligned text and bounded background without the reported stray artifact",
        "reference": "crates/bboxy450-term/tests/rich_text_safety.rs:120 — widths 240/640, zoom 90/100/160%, scales 1/2; isolated paint asserts no pixels outside capsule; PNGs and hashes retained."
      },
      {
        "criterion": "Selected native reproduction, geometry/paint checks and affected strict compilation pass without live input or backend restart",
        "reference": ".bboxy/runs/codex-rich-text-safety.json — exact source hashes, make verify-rich-text-safety exit 0, 9 selected tests, strict affected Clippy, make test-licenses exit 0; indexed report [[reports/2026-11-04-rich-text-safety-delivery]]."
      }
    ]
  }
}
```
