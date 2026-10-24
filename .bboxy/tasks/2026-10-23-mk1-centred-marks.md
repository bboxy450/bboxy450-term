---
id: "5Z0GNWSG4G3JJMJQ9H9FPM07YR"
type: "task"
number: 203
title: "MK1 — seat and task marks centred by construction"
created: "2026-10-23T22:19:34Z"
updated: "2026-10-24T00:10:38Z"
---
# MK1 — seat and task marks centred by construction

Owner: Vesper Greywind <ORACLE>. Operator, 2026-10-23, verbatim: "the icons we have - circle, circle with a dot in it, circle with a half dot in it, if you render them native and on web, and make them large, you will see that the way we reder them - they are not centered, they should be centered, to be sexy! fix that!"

**Assignee:** Sable Embertail <EXECUTOR>, worktree `~/mydev/bboxy450-term-exec3`. Paired with Wren's lab commit on `wren/centred-marks` (rebased onto main, independent of the L9-M-c reference).

**What:** the seat glyphs (● ○ ◐ ✕) and task marks (○ ◔ ◑ ◕ ● ✓ ✕ ⊘) stop being font characters and become geometry centred in their existing glyph box: a ring of radius 6 in a 16-unit box at the origin (stroke 1.5); a pie wedge from `from` turns past 12 o'clock sweeping `frac` clockwise (◔ .25, ◑ .5, ◕ .75, ● full; waiting ◐ is .5 from .5); cross ±4.2 (stroke 1.8, round caps); check (-4.8,0.4 → -1.5,3.7 → 4.8,-3.4, stroke 1.8, round); slash = the ring plus (-4.24,4.24)→(4.24,-4.24), stroke 1.5. Exact values: Wren's lab commit.

**Allowed:** `bbt-ui/src/atoms.rs`, `bbt-ui-model/src/draw/atoms.rs`, `bbt-ui/src/modules/{graph_tab,document_modal}.rs`, the shared chrome path/shape primitive and `text_glyphs.rs` atlas bake as needed, tests. The mark shapes live in bbt-ui (decision [[d39-suite-is-the-product|#39]]), not the app.

**Constraints:** no geometry moves (each mark fills its existing box); the waiting ◐ keeps Velvet's amber halo (text-shadow) as a shape shadow; coordinate with Velvet.

**Acceptance:** every glyph and task case centred to the pixel at 96 px in native and lab (image-read crops); `sh scripts/ci-local.sh --parity` at 0 geometry mismatches against the current main baseline; every review-key change explained case by case; strict workspace clippy.
