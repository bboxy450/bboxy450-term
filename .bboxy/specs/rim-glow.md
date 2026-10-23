---
id: 01M5MT43P3AHWH7F9G3EAP85DR
type: spec
title: "rim glow — the pane glow moves from the outside lane to an alive rim inside the border"
status: decided
links:
  - { to: "[[ux-prototype]]", rel: changes }
  - { to: "[[renderer]]", rel: changes }
  - { to: "[[gui-components]]", rel: changes }
  - { to: "[[2026-10-22-architecture-round-1]]", rel: decisions }
  - { to: "[[rim-never-covers-text]]", rel: invariant }
  - { to: "[[glow-never-moves-layout]]", rel: invariant }
created: 2026-10-23T16:39:50Z
updated: 2026-10-23T16:39:50Z
---
# Rim glow

Owner: Corvin Duskmantle <ARCHITECT>. Decided 2026-10-23 (decision #35). Operator, verbatim and in order, on the lab's glow-inside variants: **"I ACTUALLY LOVE RIM EFFECT"**, "lets go everywhere with rim effect, more alive gradient rim border, that is alive and moving color and pulsating", "8px alive rim is the thing we want", "go with it". The approved look is `lab/web/glow-inside.html?state=alive-rim` (Wren, 2026-10-23), which supersedes the plain 2 px variant this spec first recorded. The seat glow is an **alive rim** just inside the pane border with a bloom reaching up to 8 px in; the outside lane is retired; panes pack tighter.

Lens: the pane is the unit; its border is where identity and activity read. Assumes the lab variant's equation and table as measured on 2026-10-23. Breaks when a pane's content padding is smaller than the rim plus a text inset; the invariant below forbids that.

## 1. Semantics: the alive rim (parameters as built in the lab, CSS px; native matches them under strict parity)

- **Rim.** The band `[0, 2]` px inside the pane edge. Intensity `(.42 + .58·crest) · (.72 + .28·pulse)`, with `crest = max(a1², .8·a2²)`, `a1` a sine of wavelength `wl(170)` moving at `−.45·t`, `a2` of `wl(110)` moving at `+.30·t`: two waves in opposite directions, so the rim never reads as one thing circling. Wavelengths are **snapped to the perimeter** (`wl(λ) = P / round(P/λ)`, P the border perimeter including the corner arcs) so every wave closes without a seam.
- **Bloom.** From 2 px to **L** px inside, `exp(−(depth − 2) / (.3·L))`, weight `.3·crest·(.6 + .4·pulse)`. **L is the Settings glow value, 2 to 8, default 8**: "8px alive rim" means the bloom reaches 8 px in.
- **Colour.** The seat colour, hue-rotated along the perimeter by `.55·sin(2π(s/P − .12t)) + .3·sin(2π(s/wl(230) + .2t))` radians about the grey axis, mixed toward white by `.22·crest` on the rim. Waiting and errored rotate their state colours (amber, red) the same way.
- **Pulse.** `.5 − .5·cos(2π · time / (2600 / (.5 + spd)) ms)`: faster when busy.
- **Activity table** (amplitude / speed): idle `.32 / .35`; active `1 / 1`; waiting `.9 / .6` amber; errored `.65` red, still. Focused ×**1.3**. Unfocused panes keep the rim at full seat colour while content dims ([[ux-prototype]] dim).
- **Where it lives.** Inside the pane it covers the terminal's 8 px padding and the title strip's top and side edges, **never a text cell in the body** ([[rim-never-covers-text]]). "Everywhere with rim" means every pane-like surface: the pane and its strip now; other surfaces only when the operator asks for them by name. The sidebar row glow (`glow.row_*`) is unchanged by this spec.
- No blur pass, no particles, no outside band.

## 2. Gutters and edges

- The gutter between panes **no longer follows the glow lane**. `layout.gutter` becomes a fixed **2 px** (the divider), and `glow.lane`, `lane_min`, `lane_max`, `pane_radius` and the particle tokens retire in favour of `glow.rim = 2` (fixed), `glow.depth = 8`, `glow.depth_min = 2`, `glow.depth_max = 8`. Each pane pair gains 6 px at L = 8 (the lab gallery's measurement).
- Window edges: panes and the sidebar keep the existing 2 px inset (`sidebar.edge`); the stage inset that followed the lane becomes 2 px as well.

## 3. The Settings glow row

The Appearance row that read "Glow lane 2–8 px" **keeps its 2–8 px range and its default 8**, and now means **rim depth**: how far the bloom reaches inside the pane (`glow.depth`, L in §1). The 2 px rim itself is fixed. The row never changes layout: the gutter is fixed and the whole effect is drawn inside the pane's padding, so moving the slider repaints and nothing moves ([[glow-never-moves-layout]]). Constraint it must satisfy: every pane's content padding ≥ `glow.depth_max` (8 px), so the bloom at full depth reaches the first cell edge and never crosses it; the pane padding today is exactly 8 px, so it holds with no room to spare, and the invariant guards it.

## 4. What this supersedes, and what happens to the renderer

- **Superseded:** the outside silk lane as the pane glow: the `[glow] lane` model in `design/tokens.toml` and the Border glow spec it cites, the lab's lane pages (kept as history, never promoted), and the "gutter follows the glow lane" rule. Velvet's style-lane and silk phase (33d1f9b) stands as the silk equation and activity table; only its geometry changes.
- **`materials.wgsl` lane pass:** keep the silk equation, change the geometry: the band is the inside perimeter strip of the pane rect, parameterised by arc length round the rounded rectangle; the lane early-out becomes a strip early-out (pixels farther than `rim` inside the border, or outside it, exit immediately). No blur pass is involved in glow any more.
- **R4C cost:** the glow's pixel count drops from an 8 px outside band to a 2 px inside strip per pane; R4c's measured glow numbers are re-measured under the HUD after the rim lands, not carried over. **R12-3**, held on this question, is **retargeted** from a lane hoist to the rim pass: per-pane rim uniforms travel through the R12-2 arenas; same step, new name.
- **Parity:** Wren promotes the alive-rim variant into `glow.js` (pane, title strip, style), `pane.html` and the showcase shader, lab-only and gated until the native pass pairs with it, with the `glow.rim*` and `glow.depth*` tokens; the native rim and the lab's recapture land in **one** commit so the catalogue flips at once under strict geometry (0 mismatches, [[parity-geometry]]).

## 5. Order (the Oracle's briefs)

G0 Wren: promote `alive-rim` into the design system (pane, strip, style), tokens `glow.rim`, `glow.depth*`, retire the lane pages from the reference set, recapture. G1 native: rim pass geometry, gutter and inset tokens, Settings row, the two invariant tests, one commit with the recapture. G2: R12-3 as the rim hoist; R4C re-measured.

## 6. Acceptance, the spec signs off

1. Strict geometry parity 0 mismatches over the full catalogue after the flip; every pane case shows the rim and no lane.
2. [[rim-never-covers-text]] and [[glow-never-moves-layout]] pass and join the default invariant set in G1's commit.
3. Glow GPU time per frame under the HUD at 15 open panes: lower than the lane's R4c figure, number recorded here.
4. `sh scripts/ci-local.sh --parity` exit 0 in a clean worktree of the handed-off commit.
