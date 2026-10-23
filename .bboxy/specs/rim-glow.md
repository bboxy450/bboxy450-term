---
id: 01M5MT43P3AHWH7F9G3EAP85DR
type: spec
title: "rim glow — the pane glow moves from the outside lane to a 2 px rim inside the border"
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

Owner: Corvin Duskmantle <ARCHITECT>. Decided 2026-10-23 (decision #35). Operator, verbatim, on the lab's glow-inside variant 4 (`lab/web/glow-inside.html?state=rim`, main ab0412c): **"I ACTUALLY LOVE RIM EFFECT."** This spec turns that into the design: the seat glow is a rim just inside the pane border, the outside lane is retired, and panes pack tighter.

Lens: the pane is the unit; its border is where identity and activity read. Assumes the lab variant's equation and table as measured on 2026-10-23. Breaks when a pane's content padding is smaller than the rim plus a text inset; the invariant below forbids that.

## 1. Semantics: an inside rim

- **Where.** A band `glow.rim` px wide (default **2**) immediately inside the pane border, drawn in the pane's padding band between the border and the text cells. It never covers a text cell ([[rim-never-covers-text]]).
- **What.** The silk **crest term only**: `crest = .5 + .5·sin(2π·(s/λ − t·.35))` with wavelength λ = 150 px **snapped to the perimeter** (λ adjusted so an integer number of crests closes round the pane: `λ' = P / round(P/150)`, P the border perimeter including the corner arcs), so crests slide around the rim without a seam. No blur, no particles, no outside band.
- **Activity table** (amplitude / speed, from the lab): idle `.32 / .35`; active `1 / 1`; waiting `.9 / .6` in amber with the breath; errored `.65` red, still. Focused multiplies amplitude by **1.3**. Unfocused panes keep the rim at full seat colour while content dims ([[ux-prototype]] dim).
- **Colour.** The seat colour, as before; the sidebar row glow (`glow.row_*`) is unchanged by this spec.

## 2. Gutters and edges

- The gutter between panes **no longer follows the glow lane**. `layout.gutter` becomes a fixed **2 px** (the divider), and `glow.lane`, `lane_min`, `lane_max`, `pane_radius` and the particle tokens retire. Each pane gains the 6 px per side the lane used to reserve.
- Window edges: panes and the sidebar keep the existing 2 px inset (`sidebar.edge`); the stage inset that followed the lane becomes 2 px as well.

## 3. The Settings glow row

The Appearance row that read "Glow lane 2–8 px" becomes **"Glow rim 1–4 px"**, default 2 (`glow.rim_min = 1`, `glow.rim_max = 4`, `glow.rim = 2`). It controls the rim width only. It never changes layout: the gutter is fixed and the rim is drawn inside the pane's padding, so moving the slider repaints and nothing moves ([[glow-never-moves-layout]]). Constraint it must satisfy: every pane's content padding ≥ `rim_max + 2` px (a 2 px inset between rim and the first cell column or row); the smallest pane padding today is 8 px, so it holds with room.

## 4. What this supersedes, and what happens to the renderer

- **Superseded:** the outside silk lane as the pane glow: the `[glow] lane` model in `design/tokens.toml` and the Border glow spec it cites, the lab's lane pages (kept as history, never promoted), and the "gutter follows the glow lane" rule. Velvet's style-lane and silk phase (33d1f9b) stands as the silk equation and activity table; only its geometry changes.
- **`materials.wgsl` lane pass:** keep the silk equation, change the geometry: the band is the inside perimeter strip of the pane rect, parameterised by arc length round the rounded rectangle; the lane early-out becomes a strip early-out (pixels farther than `rim` inside the border, or outside it, exit immediately). No blur pass is involved in glow any more.
- **R4C cost:** the glow's pixel count drops from an 8 px outside band to a 2 px inside strip per pane; R4c's measured glow numbers are re-measured under the HUD after the rim lands, not carried over. **R12-3**, held on this question, is **retargeted** from a lane hoist to the rim pass: per-pane rim uniforms travel through the R12-2 arenas; same step, new name.
- **Parity:** Wren promotes the rim variant into the design system and `glow.rim` tokens; the native rim and the lab's recapture land in **one** commit so the catalogue flips at once under strict geometry (0 mismatches, [[parity-geometry]]).

## 5. Order (the Oracle's briefs)

G0 Wren: promote variant 4 into the design system, tokens `glow.rim*`, retire the lane pages from the reference set, recapture. G1 native: rim pass geometry, gutter and inset tokens, Settings row, the two invariant tests, one commit with the recapture. G2: R12-3 as the rim hoist; R4C re-measured.

## 6. Acceptance, the spec signs off

1. Strict geometry parity 0 mismatches over the full catalogue after the flip; every pane case shows the rim and no lane.
2. [[rim-never-covers-text]] and [[glow-never-moves-layout]] pass and join the default invariant set in G1's commit.
3. Glow GPU time per frame under the HUD at 15 open panes: lower than the lane's R4c figure, number recorded here.
4. `sh scripts/ci-local.sh --parity` exit 0 in a clean worktree of the handed-off commit.
