---
type: report
title: Bounded native MCP text painting and semantic inline-code capsules
task: '[[tasks/2026-11-04-rich-text-safety]]'
commits: []
verify:
  command: make verify-rich-text-safety
  exit_code: 0
author: seat:01M6KKME8YC7J9M3SZRA1J44AP
created: 2026-11-04T16:08:36.724Z
id: 01M6KN3GD99QSPJH5VDNBE613S
updated: 2026-11-04T16:08:36.724Z
number: 93377961361006
created_by: seat:01M6KKME8YC7J9M3SZRA1J44AP
updated_by: seat:01M6KKME8YC7J9M3SZRA1J44AP
authorship:
  signer: seat:01M6KKME8YC7J9M3SZRA1J44AP
  kind: seat
  public_key: Z1mnWUzcr/UKeTmWT7DZT4dYe6UKknER5pCBprq8E80=
  process_generation: '1'
provenance:
  digest: sha256:59af97c9f9d9cea34145b1afc0b42f16397d785cc3fbed99b5331eef7a2a37da
  frontmatter_digest: sha256:1edd8f62fdbc2991a9c897731d9c50831f251ef9835f04f00106b86ac38d1817
  signer: seat:01M6KKME8YC7J9M3SZRA1J44AP
  sig: 1fBhO4erXxAQEqenO3321VmR1D5GhNnJMZ+/KOYkzhQ1d140UeE91hPJP0KUFpFUGXZkAgm60HGrM+mRTN+pAQ==
  log: .bboxy/log/2026-11-04.jsonl#120799
---
# Rich text safety delivery

Task: [[tasks/2026-11-04-rich-text-safety]]. Worktree: /Users/feral/mydev/bboxy450-term-ui-status; branch codex-text-rendering-20261104; base bd820ef3558f6e0f9dbbab78d7402ef8db12e793. Authenticated Executor: seat:01M6KKME8YC7J9M3SZRA1J44AP. Harness input pin: a832bd6ff4b31a36064508632c47840a58cfe56c. The signed delivery commit contains this indexed report and the source; task_report records its final SHA (commits is empty to avoid a self-referential hash).

## Implementation and literal acceptance evidence

1. **Expanding a large MCP result no longer exits the UI or exceeds bounded native text rasterization**
   - crates/bbt-render/src/chrome/text.rs:382 — shaping retains the complete logical line extent instead of applying a whole-line bitmap limit to the existing glyph/component renderer. Raster dimension arithmetic rejects non-finite or unrepresentable geometry; scale/phase, native runs, fallback fonts, baseline and quarter-pixel positioning remain in the shared CoreText path.
   - crates/bbt-render/src/chrome/text.rs:457 — the actual reference whole-line bitmap allocation still refuses canvases above 16,384 × 2,048. The production component allocation guard in text_glyphs.rs remains unchanged. No canvas bound increase, alternate renderer, swallowed render error or dropped output.
   - crates/bboxy450-term/tests/rich_text_safety.rs:95 — private-HOME production HarnessView fixture reduces native protocol envelopes for mcp.bboxy.knowledge_fetch, captures the collapsed tool, expands through capture_frame's production disclosure path, then paints through ChromeRenderer/Metal. The 6,039-byte unbroken JSON result and Unicode tail reach a full literal text primitive with no ellipsis. Captured pixels change on expansion.
   - Restoring the old shape_at width/height guard caused this same native expansion witness to fail with “chrome text line too large”: make exit 2, inner nextest exit 100. The control script asserts both nonzero exit and exact error, restores original bytes in finally, and verifies the restored source hash.

2. **Long text and Unicode content remain accessible with correct scrolling, clipping and copy semantics**
   - crates/bbt-render/tests/chrome_long_lines.rs:16 — a 4,096-character prefix plus combining accent, ZWJ emoji, Arabic, CJK and Latin tail exceeds the old limit. Horizontal origins reveal the tail under a bounded clip; visible tail pixels equal independently rendered native tail pixels at scales 1/1.5/2/4 and all four quarter-pixel phases. The bitmap API still rejects the whole line. Assertions retain zero whole-line masks, the existing 32 MiB coverage budget, 32,768 glyph-entry cap and 2 MiB line-plan cap.
   - crates/bbt-render/src/chrome/text_glyphs.rs:188 — an x-coordinate sweep replaces all-pairs overlap discovery for disjoint wide lines while retaining exact overlap membership and source/run paint order, including reordered bidi positions. The new unit witness compares 512 deterministic ordered/reversed bounded cases against independent flood-fill overlap components.
   - crates/bbt-doc/tests/inline_code.rs:44 — positioned code-run bytes concatenate exactly to the source (including spaces, combining accent and ZWJ emoji) at widths 64/120/240/640; wrapping begins at extended grapheme boundaries. Padding is geometry and does not add thin spaces to selectable text. Clipboard OS I/O is not exercised.

3. **Inline Markdown code has aligned text and bounded background without the reported stray artifact**
   - crates/bbt-doc/src/markdown/layout.rs:603 and :995 — Inline::Code gets a semantic measured capsule using the shared Box/Text/Flow toolkit, explicit horizontal padding and a hairline border. Its real text size and first-line baseline determine layout; shrinkable content reflows under host zoom. It no longer asks the harness's font-size heuristic to infer post-layout chip rectangles. The appearance carries adjacent attribution to bboxy450-harness gui/rich.rs.
   - crates/bbt-doc/src/markdown/layout.rs:1563 — table intrinsic widths include code font advances, padding and minimum grapheme widths.
   - crates/bboxy450-term/tests/rich_text_safety.rs:120 — production Markdown preparation/layout/paint of “Run `make verify-acp-controls`, then `é👩🏽‍💻許`!” covers widths 240/640, zoom 90/100/160%, and backing scales 1/2. Assertions check source text, viewport bounds, padding, first-line baseline and wrapped baselines. Isolated capsule pixels have zero alpha outside capsule bounds (allowing only the boundary pixel's antialias footprint). No stray path/backdrop or sampled material is allowed.
   - The inline broken control routes Inline::Code back through ordinary mono text: both focused geometry tests fail (nextest exit 100), including missing semantic capsules. Exact source bytes are restored before the final passing run.
   - Native PNGs were inspected: normal-width and narrow command capsules align with prose; 160% zoom reflows inside its capsule.

4. **Selected native reproduction, geometry/paint checks and affected strict compilation pass without live input or backend restart**
   - make verify-rich-text-safety — exit 0, nine selected tests pass: private native MCP/inline witness (1), long-line tail/clip/cache witness (1), selected native normal coverage/fallback/baseline/Unicode and overlap-sweep controls (5), inline/table geometry (2).
   - The target runs strict Clippy (-D warnings) on render lib/new integration test, doc lib/new integration test, and term lib/bin/new integration test. Worktree-local target; two Cargo jobs; sequential commands. Existing normal native line-mask/fallback/baseline comparisons pass.
   - make test-licenses — exit 0; six license fixtures, 34-crate policy/notice inventory, source-notice check and nine notice fixtures pass.
   - git diff --check — exit 0 before delivery.

## Retained evidence and limits

.bboxy/runs/codex-rich-text-safety.json contains source SHA-256s, actual command exits, log hashes, restored-control hashes and four capture hashes. Raw logs and PNGs stay under target/rich-text-safety/ outside Git:
- final.log and licenses.log — passing selected gate and policy checks.
- line-guard-control.log and inline-control.log — expected failure witnesses.
- expanded-mcp.png (640×420), inline-240-zoom100-scale1.png (240×420), inline-640-zoom100-scale1.png (640×420), inline-240-zoom160-scale2.png (480×840).

An initial sandbox run had no Metal adapter; graphics-enabled execution used an owned throwaway HOME with no window, desktop capture or OS input. A fixture-only missing bitmap registration was fixed by using the harness's existing compiled icon assets. An initial incomplete match for the new semantic code piece was fixed before the passing checks.

This is a private production rich-view expansion/layout/Metal witness, not a new supervisor/socket or installed-app lifecycle test. Existing native shaping and font fallback remain authoritative; the tests cover the stated finite sizes, scales, scripts and phases, not arbitrary fonts or unlimited input. Shaping retains existing cached metadata; bitmap and atlas budgets remain bounded. No full make test, parity/formal/runtime corpus, vendor call, backend restart, live-session action, producer/composer/API edit, merge, push or install was performed. Pure shaping/layout changes required no new stateful model campaign under this brief. Oracle owns integration, pinning, installation and publication.
