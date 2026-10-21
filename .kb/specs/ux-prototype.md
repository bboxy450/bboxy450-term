# UX prototype spec — bboxy450-term sidebar and panes

Status: DECIDED by the operator in direct review, 2026-10-21. Milestone tag `ux-prototype-v1` (commit b7538ed). Source of truth for look, interaction and purpose of every element in the lab showcase (`lab/web/showcase.html`). Reasoning per rule: [[ux-flow]]. Earlier Oracle picks: [[ux-pick]]. Vocabulary: [[vision]] (seat, tab, split, shadow). **Swarm** is the UI word for what vision calls a space; the operator renamed it from swarm on 2026-10-21.

Written from the code, not from memory: `showcase.js`, `components.js`, `lab.css`, `design/tokens.toml`.

---

## 1. Purpose

The window is a cockpit for swarms of agents. The left sidebar is the map; the right stage is the view. Every element exists to answer one of four questions at a glance: which swarm, which seat, what state, where am I.

## 2. Window

- **One canvas, our chrome.** No native toolkit widgets. Dark theme by default, light theme real (not inverted), theme starts from the OS (`prefers-color-scheme`); one title-bar icon (sun in dark, moon in light) flips it, persisted.
- **Title bar** (36 px): three window dots on the left (the only window controls anywhere), sidebar-toggle icon, the identity portrait as the mark (18 px, circle-cropped `bboxy.png`), app name, a crumb `· swarm · seat` naming what is focused, the theme icon and the settings gear on the right.
- **Every pixel is precious.** No outer padding. Panes sit 2 px apart (`layout.gutter`, 4 is the alternative). The glow paints over gutters and edges and takes no layout space.
- **Everything resizes by drag.** The sidebar's right edge drags between 160 and 520 px and persists. Every gutter between panes is a divider with an 8 px grab zone; dragging changes the split ratio (10% to 90%), the split tree keeps it, the sidebar mini-grid follows on release.

## 3. Sidebar: the map

The sidebar sits on its own ground (`color.*.sidebar`): lighter than the window in dark mode, darker in light mode, so swarm blocks contrast with it in both themes.

### 3.1 Swarm block (the car)
- **Name**: double-click the band to rename in place; Enter or blur commits, Escape cancels. A new swarm is born as `swarm N` with its name field already open.
A swarm (space) is a rounded body (radius 11, surface colour) in the finish language of the 2023 Range Rover rear: flush, nothing protrudes, no chevrons, no left bars, no hairlines.
- **Band**: a thin (22 px) gloss band set into the top of the body, in the swarm colour, with the swarm name in wide-tracked uppercase (10 px, .22 em), the seat count, and on the right a colour swatch.
- **Bumper**: a 6 px strip along the bottom of the body in the swarm colour (3 px when shut). No accent line anywhere.
- **Colour**: one per swarm, random from its key (36 hues × 2 lightness bands), user-changeable by clicking the swatch. Hover lightens the band's own colour; it never turns white or grey.
- **Open and shut**: every swarm's band is always visible. Clicking a band opens or shuts the block, nothing else. A shut block is a thin tab: band only, with the roll-up of its seats' state glyphs (worst first, max 6) and a waiting badge. Several blocks may be open. Blocks sit 8 px apart (`layout.cohort_gap`).
- **Unfocused**: the swarm holding the focused seat is full contrast; every other swarm's band and rows desaturate and fade with the pane `[dim]` tokens, the body washes toward grey, the bumper keeps its colour.
- **Below the blocks**: a big body-coloured button `+ new swarm` (44 px). A swarm is a world: its own seats, tabs, shadows and stream. A new swarm starts with one zsh seat.

### 3.2 Inside an open block
Rows at cozy density (24 px, 12.5 px text, 14 px icons: compact's footprint with macOS proportions), flush on the body, no separators. Selection is an inset rounded highlight. Section labels (Shadows, new seat) are sentence case, 11 px semibold grey.
- **Tab row**: caret-less name, roll-up glyphs when collapsed, ⌘N, and the split **mini-grid**: the tab's split tree drawn as a tiny grid with real ratios, focused cell in accent, waiting and errored cells tinted. A single-seat tab shows no grid.
- **Seat row**: harness icon · character name in the seat colour · role mark (♔ oracle, ♟ executor, ♝ creativity) · title (hidden at compact density) · ⌘N · state glyph. Rows never reorder. The focused row has a quiet gloss band and no bar.
- **`+ new seat ⌘T`**: last seat row of every open swarm, like Arc's New Tab inside the space. It never moves with focus.
- **Closed strip**: a seat in grace after close (see 5.3): struck name, grey icon, countdown, `undo ⌘Z`.
- **SHADOWS**: a group of other people's harnesses in this swarm's channels (spec term: phantom seats). Grey name, desaturated icon, state glyph as published, grey breathing glow when active. No pane, no logs. Clicking one filters the stream to that participant; it never focuses or ripples.

### 3.3 Stream dock
Above the footer, hidden until the first message. Shows the last four messages of the current swarm: role rail (gold oracle, blue executor, dashed violet creativity, grey unsigned), parsed `<ROLE>` token, text, time, unread dot. A foreign Oracle gets no priority chip. Filter chip `from <name> × all` when a shadow is selected.

### 3.4 Footer
None. The big `+ new swarm` button is the only creator; the stream dock is the bottom-most element.

### 3.5 Sidebar glow budget
Rows carry the state glow at a quieter budget than panes so halos on 20 px rows never mix: radius 4, 30% of pane alpha, 4 particles on active rows only, idle rows do not glow (`glow.row_*`).

## 4. Stage: the view

The stage shows the tab that holds the focused seat, as a binary split tree with uneven ratios. Panes have square corners and sit 2 px apart.

### 4.1 Pane
- **Strip** (22 px, seat colour): macOS-style red close dot on the left, harness icon, name, role mark, title, ⌘N, state glyph. Names never wrap.
- **Border**: 1 px in the seat colour, 2 px when active.
- **Body**: terminal text, bottom-anchored, cursor last.
- **Unfocused**: strip and content desaturate (0.10) and fade (0.45), background washes 28% toward grey; the border and the glow keep the seat colour so identity survives (`[dim]`). Focused is full contrast. iTerm2's inactive read, pushed 20% further.

### 4.2 Border glow (one shader, four readings)
Rendered on a full-window WebGL2 canvas above the DOM with premultiplied alpha; particles advect along each border.
| state | glow |
|---|---|
| idle | very mild, static |
| active | breathing (1.8 s), many particles |
| waiting | amber, pulsing at 1 Hz |
| errored | red, static, few particles |
Focus brightens every state (×1.6). On a light ground the halo multiplies the background toward the seat colour instead of adding light. Pane glow radius 14 px.

## 5. Interactions

### 5.1 Focus
- Click a row or a pane: focus. If the seat lives in another tab or swarm, the stage switches to it; no block opens or shuts because of it.
- Hover a pane for 150 ms: focus (brightness only), blocked for one second after typing. Hover a sidebar row: highlight only, never focus.
- Tab / Shift+Tab: next or previous pane.
- Open/shut of a swarm and focus are two separate actions; neither triggers the other.

### 5.2 Locate (directional)
The ripple says where the thing you did not click is.
- Click a row → a ring expands from the centre of its pane, in the seat colour.
- Click a pane → its row bounces: a 520 ms scale-and-border pulse in the seat colour, no ring.
- Keyboard focus → both. A stage switch fires locate after the new screen renders. Clicking the already-focused row still locates. An off-screen row scrolls into view inside the sidebar, never the window.
- Never on hover, timers, messages or state changes.

### 5.3 Close (reversible)
The red dot closes the seat into a 60 s grace state: the pane leaves the split (sibling takes its place), the row becomes a closed strip with undo and countdown, focus moves to the first remaining pane. `undo` or ⌘Z restores it in its exact split position and refocuses it. After the window the seat is gone. No confirm dialog; the undo window is the confirmation. The process keeps running during grace (mux spec).

### 5.4 Launcher
"+ new seat" or ⌘T opens a centred Spotlight-style launcher over the blurred window: a large search field, rows grouped under section labels (Harnesses, Shells; later Browser, Remote) each with icon, name and a kind tag, prefix filter, ↑↓ and Enter, Escape or a click outside closes. The footer shows the rolled character name with a reroll and the key hints. Enter launches the seat as a new tab in that swarm, focuses and locates it. Launchables are data rows.

### 5.5 Messages
Unread shows as a dot on the seat row in the highest-priority unread role colour; focusing or clicking the seat clears it. A shadow's message pulses its row once (700 ms) and lands in the stream dock. No ripple for messages.

## 6. Signal semantics (one meaning per signal)
| signal | means | fires on | never on |
|---|---|---|---|
| border glow | state | continuously | anything else |
| focus brightness | keyboard focus is here | focus | hover highlight |
| state glyph ● ○ ◐ ✕ | state | state change | anything else |
| locate ring / row bounce | "that one, over there" | click, keyboard | hover, timers, messages |
| unread dot | a message is unread | arrival | anything else |
| hover | pointer is here | pointer | no focus on rows, no ripple |

## 7. Tokens (single source: `design/tokens.toml`)
Colour (dark and light sets, state and role colours, seat colour hash), density (touch / comfortable / compact), radius, `layout.gutter`, `layout.cohort_gap`, sidebar widths, strip height, `[glow]` pane and row budgets, `[dim]`, motion. The web lab consumes `tokens.js`, generated by `lab/web/gen-tokens.py`; the Rust build will include the same file.

## 8. Demo-only (not product)
Keys 1-4 (state of the focused seat), M (message arrives), P (shadow speaks), the log replay with its state following, `?scene=15`, `?gutter=`. No legend in the window; the list lives on the lab index.

## 9. Gaps
- Mute / unmute of a seat with presence notices (operator decision in [[vision]]) has no UI yet.
- Remote owned seats versus shadows: owned seats over ssh look like local seats; not shown in the lab.
- Settings as a tab and the toolbar icon exist as lab pages but are not wired into the showcase.
