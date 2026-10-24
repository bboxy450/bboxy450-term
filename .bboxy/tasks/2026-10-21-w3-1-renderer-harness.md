---
id: "3FJ48X4G47CF0J4J6GGAMP6YKC"
type: "task"
number: 100
title: "W3.1 — bbt-render: headless harness, smoke test, first text"
created: "2026-10-21T21:11:08Z"
updated: "2026-10-24T00:10:38Z"
---
# W3.1 — bbt-render: headless harness, smoke test, first text

Owner: Vesper Greywind <ORACLE>. Assignee: first free Executor after W1.1 starts (does not depend on W1.1 for steps 1–2). Status: OPEN. Spec: [[renderer]] §1–§4, order §5 steps 1–2. Plan: [[plan]] W3.

Worktree `~/mydev/bboxy450-term`. Allowed: `crates/bbt-render/**`, root `Cargo.lock`, `.github/workflows/ci.yml` (only to add the render artifact path if missing), `.bboxy/runs/w3-1-*.log`.

1. `wgpu 30.0.1`, Metal. `device.rs` creates an adapter/device without a surface. `headless.rs`: `render_to_rgba(scene, w, h) -> Vec<u8>` via an offscreen texture and buffer readback; `png` write helper to `target/render/<name>.png`.
2. Smoke test: render a solid-colour quad and a hard-coded 80×24 grid of coloured cell backgrounds (`cells.bg` pass, full-screen fragment reading a rows×cols colour buffer). Assert pixel colours at known cells. `cargo test -p bbt-render` green locally.
3. `cells.text`: glyph atlas (grayscale R8, grows ×2), CoreText rasterization via `objc2-core-text` for ASCII in one font (Menlo), instanced draw with the 32-byte instance layout from terminal-core D3. Smoke test renders "bboxy450-term" and asserts ink in the expected cells and none elsewhere.
4. Variant generator skeleton: `tests/variants.rs` enumerates font×size×scale×theme (values from renderer §4.3) and renders the smoke scene for each to `target/render/smoke/<variant>.png`. No goldens yet; the images are the deliverable for the operator to look at.
5. CI: confirm the macOS runner renders headless Metal; if not, record the error in `.bboxy/runs/w3-1-ci.log` and gate the tests behind an env var.

Verify: `cargo test -p bbt-render` green; `ls target/render/smoke | wc -l` ≥ 48. Report: commit, image count, one PNG path for the operator, what is not covered.
