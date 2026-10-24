---
id: 01M5MT43P3AHWH7F9G3EAP85DR
number: 154
type: spec
title: "rim glow — the pane glow is an alive rim centred on the pane border, half out, half in"
status: decided
links:
  - { to: "[[d35-alive-rim]]", rel: decision }
  - { to: "[[d39-suite-is-the-product]]", rel: decision }
  - { to: "[[d42-focus-swell]]", rel: decision }
  - { to: "[[d43-kit-alive-borders]]", rel: decision }
  - { to: "[[d44-waiting-attention]]", rel: decision }
  - { to: "[[ux-prototype]]", rel: changes }
  - { to: "[[renderer]]", rel: changes }
  - { to: "[[gui-components]]", rel: changes }
  - { to: "[[2026-10-22-architecture-round-1]]", rel: decisions }
  - { to: "[[rim-never-covers-text]]", rel: invariant }
  - { to: "[[glow-never-moves-layout]]", rel: invariant }
created: 2026-10-23T16:39:50Z
updated: 2026-10-24T00:24:57Z
---
# Rim glow

Owner: Corvin Duskmantle <ARCHITECT>. Decided 2026-10-23 (decision [[d35-alive-rim|#35]]). Operator, verbatim and in order, on the lab's glow-inside variants: **"I ACTUALLY LOVE RIM EFFECT"**, "lets go everywhere with rim effect, more alive gradient rim border, that is alive and moving color and pulsating", "8px alive rim is the thing we want", "go with it". The approved look is `lab/web/glow-inside.html?state=alive-rim` (Wren, 2026-10-23), which supersedes the plain 2 px variant this spec first recorded. The seat glow is an **alive rim** just inside the pane border with a bloom reaching up to 8 px in; the outside lane is retired; panes pack tighter.

Lens: the pane is the unit; its border is where identity and activity read. Assumes the lab variant's equation and table as measured on 2026-10-23. Breaks when a pane's content padding is smaller than the rim plus a text inset; the invariant below forbids that.

## 1. Semantics: the alive rim (parameters as built in the lab, CSS px; native matches them under strict parity)

- **Rim.** The band `[−R/2, +R/2]` px **across the pane border**, half outside and half inside, **R = `glow.rim`, the Settings "Rim size", 2 to 8, default 4** (operator, 2026-10-23, verbatim and in order: "2px can come 1px on the inside, 1px on the outside so border feels more alive", "rim size configurable from 2 to 8 px, half of size on each side!", "do half, half, maybe? for balance", "half / half / gap grows with the rim, outer halves meet this is the way to go!", "solidify and commit and make sure it gets implemented in real native ui"; earlier the same day "2extra pixels" had moved a 2 px inside rim to 4 inside, superseded within the hour). The shaders read R as a uniform, never a literal, and the rim quad grows by R/2 outside the pane rect. Intensity `(.42 + .58·crest) · (.72 + .28·pulse)`, with `crest = max(a1², .8·a2²)`, `a1` a sine of wavelength `wl(170)` moving at `−.45·t`, `a2` of `wl(110)` moving at `+.30·t`: two waves in opposite directions, so the rim never reads as one thing circling. Wavelengths are **snapped to the perimeter** (`wl(λ) = P / round(P/λ)`, P the border perimeter including the corner arcs) so every wave closes without a seam.
- **Bloom.** From the rim's inner edge, R/2, to **L = `glow.depth` = 8** px inside the border, `exp(−(depth − R/2) / (.3·L))`, weight `.3·crest·(.6 + .4·pulse)`. **L is fixed at 8** (the pane padding); it stopped being the Settings value when the slider became rim size, and `glow.depth_min` and `glow.depth_max` retire once native stops reading them.
- **Colour.** The seat colour, hue-rotated along the perimeter by `.55·sin(2π(s/P − .12t)) + .3·sin(2π(s/wl(230) + .2t))` radians about the grey axis, mixed toward white by `.22·crest` on the rim. Waiting and errored rotate their state colours (amber, red) the same way.
- **Pulse.** `.5 − .5·cos(2π · time / (2600 / (.5 + spd)) ms)`: faster when busy.
- **Activity table** (amplitude / speed): idle `.32 / .35`; active `1 / 1`; waiting `.9 / .6` amber; errored `.65` red, still. Focused ×**1.3**. Unfocused panes keep the rim at full seat colour while content dims ([[ux-prototype]] dim).
- **Where it lives.** The inner half and the bloom cover the terminal's 8 px padding and the title strip's top and side edges, **never a text cell in the body** ([[rim-never-covers-text]]); the outer half lies in the gutter, where it meets the neighbour's outer half exactly, or in the stage inset. "Everywhere with rim" means every pane-like surface: the pane and its strip now; other surfaces only when the operator asks for them by name. The sidebar row glow (`glow.row_*`) is unchanged by this spec.
- No blur pass, no particles; the only paint outside a pane rect is its rim's outer half, R/2.

## 2. Gutters and edges

- The gutter between panes **is the rim size**: `layout.gutter = glow.rim` (R), so the outer halves of two neighbouring rims meet exactly and never overlap (operator: "gap grows with the rim, outer halves meet"). Tokens: `glow.rim = 4`, `glow.rim_min = 2`, `glow.rim_max = 8`, `glow.depth = 8` fixed; `glow.lane*`, `pane_radius` and the particle tokens are retired, `glow.depth_min` and `glow.depth_max` retiring. Measured in the lab showcase (Wren, wren/rim-4): R = 2 gives inset 1 and gap 2; R = 4 inset 2 and gap 4; R = 8 inset 4 and gap 8. **The slider is a layout setting by design**, applied through relayout ([[rim-size-is-layout]]), never by paint; the fixed 2 px gutter of the first ruling yields to it.
- Window edges: the stage inset is **R/2**, one outer half, so a pane at the window edge shows its whole rim; the sidebar keeps `sidebar.edge` 2 px (it has no rim; its row glow is unchanged).

## 3. The Settings glow row

The Appearance row becomes **"Rim size", 2–8 px, default 4** (`glow.rim`, R in §1). The rim is R/2 outside and R/2 inside the border, the gutter is R and the stage inset R/2, so **moving the slider re-lays out the stage**: gutter and inset only, through the normal relayout path, never as paint; pane padding, strips and cell grids never change ([[rim-size-is-layout]]). Depth is fixed at 8 and is no longer a setting; changing nothing but depth still moves nothing ([[glow-never-moves-layout]]). **The cell edge rule (settled 2026-10-23):** inside the border the glow band is the half-open strip `[0, min(8, padding))`, **clipped to the pane's content padding**, so no pixel of the text cell grid ever receives glow; at L = 8 inside the 8 px padding the bloom's last pixel sits against the first cell edge at about `exp(−2.5) ≈ 8 %` of the bloom weight, which is a fade to the edge, not over it. The padding does **not** grow (the operator wants tighter panes; the only layout that follows the rim is the gutter and the inset, which hold no content); the invariant measures the **whole band**, outer half, inner half and bloom, not the rim core alone (a core-only check would permit bloom over text), and the outer half touches no surface's content because gutter and inset hold none. A surface with padding smaller than the depth shows a shallower bloom, never glow over cells.

## 4. What this supersedes, and what happens to the renderer

- **Superseded:** the outside silk lane as the pane glow: the `[glow] lane` model in `design/tokens.toml` and the Border glow spec it cites, the lab's lane pages (kept as history, never promoted), and the "gutter follows the glow lane" rule. Velvet's style-lane and silk phase (33d1f9b) stands as the silk equation and activity table; only its geometry changes.
- **`materials.wgsl` lane pass:** keep the silk equation, change the geometry: the band is the perimeter strip **centred on the pane border**, R/2 each side, parameterised by arc length round the rounded rectangle; the quad grows by R/2 outside the rect; the early-out exits pixels farther than R/2 outside the border or farther than the depth inside it. Shader shape as the lab has it: `rim = smoothstep(−out, −out + .6, dep) · (1 − smoothstep(in − .4, in + .4, dep))`, bloom from `in` to depth, with `in = out = R/2`. No blur pass is involved in glow any more.
- **R4C cost:** the glow's pixel count drops from an 8 px outside band to an R px strip centred on the border plus the bloom; R4c's measured glow numbers are re-measured under the HUD after the rim lands, not carried over. **R12-3**, held on this question, is **retargeted** from a lane hoist to the rim pass: per-pane rim uniforms travel through the R12-2 arenas; same step, new name.
- **Parity:** Wren promotes the alive-rim variant into `glow.js` (pane, title strip, style), `pane.html` and the showcase shader, lab-only and gated until the native pass pairs with it, with the `glow.rim*` and `glow.depth*` tokens; the native rim and the lab's recapture land in **one** commit so the catalogue flips at once under strict geometry (0 mismatches, [[parity-geometry]]).

## 5. Order (the Oracle's briefs)

G0 Wren: promote `alive-rim` into the design system (pane, strip, style), tokens `glow.rim`, `glow.depth*`, retire the lane pages from the reference set, recapture. G1 native: rim pass geometry, gutter and inset tokens, Settings row, the two invariant tests, one commit with the recapture (landed, f403bd9, as the inside rim). G2: R12-3 as the rim hoist; R4C re-measured. **G3 native (the operator's "make sure it gets implemented in real native ui"):** the centred rim with `in = out = R/2`, `layout.gutter = R` and the stage inset `R/2` from the setting through relayout, the Settings row as "Rim size" 2–8 default 4, depth fixed at 8, the `rim-size-is-layout` test and the outer-half extension of `rim_never_covers_text`, in one commit with Wren's recapture (wren/rim-4) under strict geometry.

## 6. Acceptance, the spec signs off

1. Strict geometry parity 0 mismatches over the full catalogue after the flip; every pane case shows the rim and no lane.
2. [[rim-never-covers-text]] and [[glow-never-moves-layout]] pass and join the default invariant set in G1's commit.
3. Glow GPU time per frame under the HUD at 15 open panes: lower than the lane's R4c figure, number recorded here.
4. `sh scripts/ci-local.sh --parity` exit 0 in a clean worktree of the handed-off commit.
5. Rim size 2, 4 and 8 give gutter 2, 4, 8 and inset 1, 2, 4 in native and lab alike, 0 geometry mismatches; [[rim-size-is-layout]] passes and joins the default set in G3's commit.

## 7. Focus swell (operator, 2026-10-23, decision [[d42-focus-swell|#42]])

Verbatim, from Wren's "focus swell" demo at 10/20/30/40 px: "can we change focus circle effert to just using silk border glow, and pulsing it to say 40pxs and then back, you know? without changing spacing between blocks, as an indication of me focusing it", then "40px swell is fucking perfect!".

- **What.** On focus, the focused block's silk swells **40 px past its rim** and settles back over **0.9 s** with a sine envelope (`reach(t) = 40 · sin(π · t / 0.9)` px, t in seconds from focus), same colour and crest as the rim, weight falling to zero at the reach. It **replaces the locate ripple** everywhere the ripple fired (pane and row, click and keyboard focus; the ux-flow locate rules are superseded and Wren rewrites them with the reference).
- **Paint, never layout.** The swell is drawn in the glow pass over the gutter and over neighbouring blocks; nothing moves ([[glow-never-moves-layout]], [[rim-size-is-layout]]). It is zero at rest, so the steady-state band and [[rim-never-covers-text]] are untouched.
- **Legibility.** The focused block's own text is never covered (the swell lies outside its rim). A neighbour's text under the swell stays readable. **Acceptance from Wren's reference (df6d58e, 2026-10-23):** over a neighbour's nearest text cell (12 px away at rim size 4) the weight is at most **0.48** (mean 0.36, trough 0.13) for at most 0.9 s; the curve is `weight = env · (.6·e^(−2.8u)·(.45 + .55·crest) + .18·e^(−((u − c0)/.22)²))` with `u = (d − R/2)/reach`, `reach = 40·sin(πt/900 ms)`, `env = sin(πt/900 ms)`, the weight fixed and not scaled by activity; the native pass matches it under strict parity.
- **Tokens.** `glow.swell = 40` (px past the rim), `glow.swell_ms = 900`; both read by the shader as uniforms.

## 8. Alive borders are a kit capability (operator, 2026-10-23, decision [[d43-kit-alive-borders|#43]])

Verbatim: "and same swell and same border effect as on terminal blocks should be on the left, each block in our UI GPUGUI system should have ability to define this type of dynamic border and border swell and such! thats important, our blocks should be sexy out of the box!"

- **Where it lives.** The rim and the swell are a **style property of any `bbt-ui` block**, not pane code: `Style.rim { size, outer, rim_alpha, swell, attention }` on the block's style (`Style.border` already exists as the hairline stroke, `crates/bbt-ui/src/style.rs:299`, and stays what it is; Oracle wording fix, 2026-10-23), with the same `materials.wgsl` rim pass run over the block's rect; the "alive" inputs (activity, seat colour, focus, pulse phase) arrive per block through the R12-2 arenas as uniforms. Shader in `bbt-render`, style types in `bbt-ui-model` (shared with the lab), defaults in `bbt-ui`; nothing in the app ([[layering]], decision [[d39-suite-is-the-product|#39]]). **What it replaces:** today `Pane` sets the rim as a style override, `style.overrides.glow = Some(ui::Glow { … })` (`crates/bbt-ui/src/modules/pane.rs:398`), and only panes have one; that override becomes `Style.rim` on every block, filled per block kind from the tokens, so `Pane` stops writing it and the rim pass reads `rim` off every block rect. `ui::Glow` as a pane-only override is retired in G5 (Seraph renames it).
- **Defaults.** Panes and their strips, sidebar seat rows, tabs and cards carry the rim and the swell out of the box; a block opts out by style, never by code.
- **Layout rule for blocks that are not panes.** Only the pane gutter and the stage inset follow the rim size ([[rim-size-is-layout]]). Every other block draws its outer half into the spacing it already has (the lab's `--rim-space`, 0 for edge-to-edge rows) and is clipped there; its inner half and bloom stay inside its own padding, so a block with less padding than the bloom shows a shallower bloom, never glow over its content. Idle seat rows carry no rim at rest (`glow.row_idle`) and still take the swell; cards keep their gap and fit the outer half (Wren's reference df6d58e).
- **Order.** Lands after G3 as **G5**: the capability, the defaults on rows, tabs and cards, Wren's references for each, one paired commit under strict geometry.

## 9. Attention

A seat waiting for the operator (question, permission, auth) auto-swells and pulses strongly until handled, and fires a desktop notification whose click focuses it: [[attention]] (decision [[d44-waiting-attention|#44]]).
