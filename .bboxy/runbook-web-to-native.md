---
id: "0CYVEEJVA1RJ4171D7WV0WEH6S"
type: "note"
number: 120
title: "Runbook — the web → native loop"
created: "2026-10-22T00:35:09Z"
updated: "2026-10-24T00:10:38Z"
---
# Runbook — the web → native loop

How a UI element goes from an idea to a pixel on Metal, with a harness looking at the pictures at every step. This is the whole system, end to end, as it runs today. Back to [[vision]] · specs [[ux-prototype]] [[gui-components]] [[renderer]] · graph [[graph]].

## 0. The idea in one paragraph

Design happens in a plain HTML lab that anyone can open in a browser. The design is written down as data, not prose: a design-system JSON that names every element and its states, and two token files that hold every colour, size and curve. The Rust build compiles those same token files in, so the native renderer and the lab cannot drift on values. A storybook binary, bboxybook, renders every component in every state, density and theme on the real wgpu renderer. A script captures the lab headless, renders the same matrix natively, diffs each pair, and writes an index worst-first. A harness reads the index and the images, fixes the worst, re-runs. The operator looks at checkpoints, not at prose. Iterate.

## 1. The pieces and who owns them

| Piece | Path | Owner | What it is |
|---|---|---|---|
| Web lab | `lab/web/*.html`, `components.js`, `showcase.js`, `lab.js`, `lab.css`, `fixtures.js` | Creativity (Wren) | One page per component with a state strip, density and theme toggles; `showcase.html` is the full-window mock; every state deep-linkable `?state=<slug>&theme=&density=` |
| Design system | `design/bboxy-design-system.json` | Creativity | Foundations, signals, elements, modules, each with states and a `status: drawn \| built`; the queue of work is `drawn, not built` |
| Tokens | `design/tokens.toml`, `design/style.json` | Creativity writes, everyone reads | Colours, radii, row heights per density, glyph sizes, glow, dim, motion; the Raycast material; `lab/web/gen-tokens.py` emits `tokens.js` for the lab |
| Rule log | `.bboxy/ux-flow.md`, `.bboxy/ux-pick.md`, `.bboxy/specs/ux-prototype.md` | Creativity + Oracle | Every operator rule with its why; the pick history; the current UX state spec, parity target tag `ux-prototype-v2` |
| ui-model | `crates/bbt-ui-model` | Executor | Compile-time tokens (`build.rs` reads both token files; a test asserts equality with `tokens.js` at every path), model types, tree algebra ported from the lab, our layout engine, draw-list atoms; zero runtime deps |
| Renderer | `crates/bbt-render` | Executor | wgpu on Metal, one device: cell passes, CoreText grayscale atlas, chrome pass (SDF rounded rects, hairlines, gloss, text runs), fx pass; `headless.rs` renders to PNG |
| bboxybook | `crates/bboxybook` | Executor | The native storybook: `gallery` (windowed, winit), `manifest`, `render --all`, `compare`; `fixtures/design-system-v22.json` pins the coverage horizon; `build.rs` bakes the lab SVG icons |
| Parity loop | `scripts/parity.sh`, `crates/bboxybook/capture.mjs`, `parity.rs` | Executor | Capture web → render native → diff → `target/parity/index.html` + `report.md` |
| CI | `.github/workflows/ci.yml` | Oracle | macOS runner; runs the loop when lab, design, ui-model, render, bboxybook or the script change; uploads the index as an artifact |
| Oracle | this file, [[graph]], briefs | Oracle | Decides, briefs, verifies by looking, calls checkpoints |

## 2. The loop, step by step

```
operator taste ──▶ Creativity builds it in the lab (one element per pass)
                      │  commit + push, one line on #bboxy450-term
                      ▼
          design-system.json status: drawn → built (web)
          tokens.toml / style.json hold every value
                      │
                      ▼  (same files, compiled in)
          bbt-ui-model build.rs ──▶ TOKENS / STYLE constants
                      │
                      ▼
          Executor ports the element: ui-model type + draw atoms + chrome module
          bboxybook gallery entry (guard test: every built design-system entry has one)
                      │
                      ▼
          scripts/parity.sh
            1. cargo build -p bboxybook
            2. bboxybook manifest  → requested.json   (every component × state × density × theme)
            3. node capture.mjs    → target/parity/web/**.png      (headless Chromium over the lab deep links)
            4. bboxybook render --all → target/parity/native/**.png (headless Metal)
            5. bboxybook compare   → diff/**.png, index.html (web | native | diff, worst first), report.md
                      │
                      ▼
          a harness reads report.md and the three PNGs of the worst triplet   ← the judging step
          fixes the Rust side (or flags a lab bug to Creativity)
          re-runs the loop                                                       ← iterate
                      │
                      ▼
          Oracle reviews index.html, opens it once for the operator at a checkpoint
          operator gives points → Creativity brief or Executor brief → top of the loop
```

## 3. Commands

```sh
# the whole loop
scripts/parity.sh                      # writes target/parity/{web,native,diff,index.html,report.md}
open target/parity/index.html

# pieces
python3 lab/web/gen-tokens.py          # tokens.toml + style.json → lab/web/tokens.js (+ bundles lab/web/logs/*.txt)
cargo test -p bbt-ui-model --locked    # token parity at every path, model algebra, layout vs the v2 scene fixture
cargo run -p bboxybook -- gallery      # windowed storybook on Metal
cargo run -p bboxybook -- gallery --smoke
cargo run -p bboxybook -- render --all --out target/bboxybook
cargo run -p bboxybook -- showcase --scene 15
cargo test -p bboxybook --locked       # every preview renders on the GPU; built-entry guard; report regression

# lab only
open lab/web/index.html                # page list and demo keys
open "lab/web/seat-row.html?state=subagents-claude&theme=light&density=cozy"
```

Environment: `PARITY_OUT` overrides `target/parity`. Capture needs a local Chromium; without it the report marks references unproven and the script exits non-zero after rendering.

## 4. Reading the report

`report.md` header: cases, how many need review, the tolerance policy (8 per channel, 0.1% changed-pixel budget). Then a table sorted worst-first: component/state/density/theme, changed-pixel percentage, mean error, links to the three images. Rules of reading:

- A score ranks; it never accepts. Acceptance is a harness or the operator looking at the three images.
- Compare like with like: if web and native rendered different fixture data, the diff is meaningless; fix the fixture first (W3.2c step 0 makes bboxybook consume `lab/web/fixtures.js`).
- Blank native terminal bodies are allowed by design; a missing frame, strip or glyph is not.
- Goldens for the renderer's own tests (`bbt-render`, `tests/golden`) update only with `UPDATE_GOLDENS=1` and the diff attached to the commit; parity references are the lab, not goldens.

## 5. Adding a component

1. Creativity: draw it in the lab page with every state, add the design-system entry (`status: drawn`), tokens for any new value, push, one line on the channel.
2. Oracle: if it implies a new seat state, signal type or spec field, the spec changes first (one line in [[ux-pick]] or the spec), then the lab flips the entry to `built` on the web side.
3. Executor: ui-model type + atoms, chrome module, bboxybook gallery entry; the guard test fails until the entry exists.
4. Run the loop; iterate until the triplet reads the same to a harness; Oracle checkpoint.

## 6. Cadence and roles

- One element per pass. No premature optimisation. 120 fps is later.
- Creativity commits its own lab passes, often; the Oracle commits `.bboxy/` by path; nobody uses `git add -A` in the shared worktree.
- Checkpoints are rare and deliberate: bboxybook demos, each parity index, and the operator's single review when the lab is complete. The Oracle opens the page once and waits.
- Harnesses look at images. Every improvement loop ends with a harness reading PNGs, not a number.

## 7. Where it is today (2026-10-21)

Lab: 23 pages, design system v37, passes 1–11 of the 19-item queue plus six new pages and the diagrams page built. ui-model: 28 tests, zero deps, scene-15 layout equal to the v2 blob. Renderer: text on Metal, 48-variant smoke matrix, CI green. bboxybook: 56 previews, 1,664 triplets, first checkpoint accepted with 1,406 over tolerance, dominated by fixture mismatch; W3.2c aligns fixtures and re-runs. Next: input and focus, launcher, signals→fx, then the document modal and the Mermaid engine ([[document-viewer]]), transparency ([[renderer]] §1b).

Links: [[vision]] [[ux-prototype]] [[gui-components]] [[renderer]] [[graph]] [[plan]] [[document-viewer]]
