# UX prototype spec — bboxy450-term sidebar and panes

Status: DECIDED by the operator in direct review, 2026-10-21. Milestones: `ux-prototype-v1` (b7538ed, first review) and `ux-prototype-v2` (the implementation starting point; the lab is the parity target for the renderer harness). Source of truth for look, interaction and purpose of every element in the lab showcase (`lab/web/showcase.html`). Reasoning per rule: [[ux-flow]]. Earlier Oracle picks: [[ux-pick]]. Vocabulary: [[vision]] (seat, tab, split, shadow). **Swarm** is the canonical unit in [[vision]], decided by the operator on 2026-10-21.

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
A swarm is a rounded body (radius 11, surface colour) in the finish language of the 2023 Range Rover rear: flush, nothing protrudes, no chevrons, no left bars, no hairlines.
- **Band**: a thin (22 px) gloss band set into the top of the body, in the swarm colour, with the swarm name in wide-tracked uppercase (10 px, .22 em), the seat count, and on the right a colour swatch.
- **Bumper**: a 6 px strip along the bottom of the body in the swarm colour (3 px when shut). No accent line anywhere.
- **Colour**: one per swarm, random from its key (36 hues × 2 lightness bands), user-changeable by clicking the swatch. Hover lightens the band's own colour; it never turns white or grey.
- **Open and shut**: every swarm's band is always visible. Clicking a band opens or shuts the block, nothing else. A shut block is a thin tab: band only, with the roll-up of its seats' state glyphs (worst first, max 6) and a waiting badge. Several blocks may be open. Blocks sit 8 px apart (retired-vocabulary token name: `layout.cohort_gap`).
- **Unfocused**: the swarm holding the focused seat is full contrast; every other swarm's band and rows desaturate and fade with the pane `[dim]` tokens, the body washes toward grey, the bumper keeps its colour.
- **Below the blocks**: a big body-coloured button `+ new swarm` (44 px). A swarm is a world: its own seats, tabs, shadows and stream. A new swarm starts with one zsh seat.

### 3.2 Inside an open block
Rows at cozy density (24 px, 12.5 px text, 14 px icons: compact's footprint with macOS proportions), flush on the body, no separators. Selection is an inset rounded highlight. Section labels (Shadows, new seat) are sentence case, 11 px semibold grey.
- **Tab row**: caret-less name, roll-up glyphs when collapsed, ⌘N, and the split **mini-grid**: the tab's split tree drawn as a tiny grid with real ratios, focused cell in accent, waiting and errored cells tinted. A single-seat tab shows no grid.
- **Seat row**: a where-mark first when the seat is remote (cloud glyph for cloud, server glyph for ssh, nothing for local) · harness icon in a small coloured tile · character name in the seat colour · role mark (♔ oracle, ♟ executor, ♝ creativity) · title (hidden at compact density) · ⌘N · state glyph. Rows never reorder. The focused row has a quiet gloss band and no bar.
- **`+ new seat ⌘T`**: last seat row of every open swarm, like Arc's New Tab inside the space. It never moves with focus.
- **Closed strip**: a seat in grace after close (see 5.3): struck name, grey icon, countdown, `undo ⌘Z`.
- **SHADOWS**: a group of other people's harnesses in this swarm's channels (published remote participants). Grey name, desaturated icon, state glyph as published, grey breathing glow when active. No pane, no logs. Clicking one filters the stream to that participant; it never focuses or ripples.

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
- Tab / Shift+Tab: next or previous pane. Arrow keys: the nearest pane in that direction (⌘⌥ arrows while a field has focus, plain arrows otherwise). Keyboard moves fire locate on both sides.
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
"+ new seat" or ⌘T opens a centred Spotlight-style launcher over the blurred window: a large search field, rows grouped under section labels (Harnesses, Shells; later Browser, Remote) each with icon, name and a kind tag, prefix filter, ↑↓ and Enter, Escape or a click outside closes. The footer shows the rolled character name with a reroll and the key hints. Enter launches the seat as a new tab in that swarm, focuses and locates it. Launchables are data rows. Rows nest: SSH opens to a machine list (from ~/.ssh/config), Cloud to a region list, each machine or region to the harnesses, each harness to its placements, so "ssh rig codex" types straight through and the seat is born remote on that host; each harness row opens (→ or Enter on the row's chevron) to its placements New tab / Split right / Split down; a "Go to" row opens to every seat and swarm, Enter on a seat focuses it with locate, on a swarm opens it and jumps. ← or Backspace on an empty field climbs out; a crumb chip shows the path. Typing flattens the nest: "goto nyx" or "codex split" filter straight into the sublist. Enter on a harness row launches with the default placement. Number keys pick the nth visible row. All of this holds in every launcher, including the ⌘D presets.
- **Matching** is tiered: label and alias prefix; facets (seats carry harness, where, host, role, state, swarm, muted, so "codex", "ssh", "cloud", "oracle", "waiting" filter Go to); subtitle substring; fuzzy subsequence on the label; and, in the real app only, semantic matching with the Jev encoder (ICQ's local pocket-encoder MiniLM, 384-d, no network): rows embedded once, the query per keystroke, cosine merged under the lexical tiers as "Related". Words are ANDed; a word naming a parent descends instead. Jev also orders: with an empty field the rows closest to the context (focused seat, swarm, recent picks, recent messages) form the Suggestions section; with a query it breaks ties inside a tier; plain counts, not Jev, order placements by habit; nothing reorders within 300 ms of a keystroke or re-numbers rows reachable by number keys. Full detail in the design system's modal.matching.

### 5.5 Splits
Two ways, same result.
- **Launcher keys**: ⌘T opens the launcher for a new tab; inside it ↵ new tab, ⌘↵ split right of the focused pane, ⌘⇧↵ split below. ⌘D and ⌘⇧D (iTerm) open the launcher already preset to split right or below of the focused seat: the placeholder names the placement, ↵ launches there, ⌥↵ makes a tab instead. The footer pill names the active actions. A banner above the panel says what will happen: for ⌘D / ⌘⇧D it draws the real pane layout large with the new cell breathing in accent ("Split right · of <seat>"), for ⌘T it draws the swarm's tab strip with a new lit tab ("New tab · in <swarm>"); behind the modal the target half of the focused pane breathes in the same tint.
- **After launch**: the new pane is focused, the locate ring fires at its centre, its cursor blinks, and the pointer warps to the pane centre (native only; the lab cannot move the OS pointer).
- **Drag and drop**: drag a seat row from the sidebar, or a pane by its strip, onto another pane. The hovered half of the target (left, right, top, bottom) tints with the selection tint; dropping splits there at 50/50. A dragged seat moves: its old split collapses into the sibling. Phantoms cannot be dragged.
Either way the placed seat gets focus and the locate signal. Ratios are then adjusted by dragging the gutter (section 2).

### 5.6 Settings
⌘, or the Settings row in the launcher opens settings as a tab in the current swarm: a pane with a strip, a left nav with search, and cards in the card material. Sections: Account as a hero around the GPG key (portrait beside the full 40-hex fingerprint grouped in 4s, ed25519 · signing, unlocked · gpg-agent cache, uid, copy fingerprint, trust path root key → device subkeys → seat keys, root held by gpg-agent · never exported, Show paper recovery with its identity-only caption, Link Google for cloud drawn as a link), device subkey cards with revoke (local; peers learn on next contact) and an enroll card that produces a signed request when the primary is elsewhere, a Signatures card (commits on, messages via the seat's leaf key on, seats sign commits off and assumed); Cloud providers as tiles with a connect toggle; SSH machines as rows from `~/.ssh/config`; Roles as cards per role with a built-in chip, default system prompt, may-send-to and an editable priority class (changing class re-issues the role's seat grants), plus a + new role card (name, colour, mark, prompt, send, class); Harnesses as tiles with default role, wake, state source and hooks, plus a + new harness tile (name, command, colour, icon; runs through the PTY adapter); Appearance with theme thumbnails, density previews at real row heights, gutter, glow and subtitle, all applying live; Shortcuts. Read-mostly from `config.toml`: a changed row shows the accent dot and a reset. Opens on Account. Closing the tab needs no grace.

### 5.7 Mute
⌘M or the speaker icon in the pane strip toggles mute on the focused seat. A muted seat works one to one with the operator: its row stays in place with an amber muted mark and a dimmed name; its messages to the swarm are held; the swarm stream shows a centred presence notice on mute and again on unmute. No other signal changes.

### 5.8 Messages
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

## 7. Style and tokens

`design/bboxy-design-system.json` is the structured design system: foundations, signals, elements, modules (the launcher modal as its own module with its full keyboard and nesting rules), interactions and gaps. It references the two value files below.

`design/style.json` is the visual style: the Raycast material (frosted panels, hairlines, one radius scale, one type scale, selection as a tint, tiles, badges, pills, one motion curve) with the swarm band and bumper layered on top. Every surface in the showcase is drawn from its values through CSS variables.

Tokens (single source: `design/tokens.toml`)
Colour (dark and light sets, state and role colours, seat colour hash), density (touch / cozy / comfortable / compact), radius, `layout.gutter`, the retired-vocabulary token name `layout.cohort_gap`, sidebar widths, strip height, `[glow]` pane and row budgets, `[dim]`, motion. The web lab consumes `tokens.js`, generated by `lab/web/gen-tokens.py`; the Rust build will include the same file.

## 8. Demo-only (not product)
Keys 1-4 (state of the focused seat), M (message arrives), P (shadow speaks), the log replay with its state following, `?scene=15`, `?gutter=`. No legend in the window; the list lives on the lab index.

## 9. Gaps
- Remote owned seats versus shadows: owned seats over ssh look like local seats; not shown in the lab.
- The toolbar icon exists as a lab page only.

## 10. From the specs (drawn, not built)

Seventeen implications the other specs place on the UI, recorded by the Oracle on 2026-10-21 and drawn into `design/bboxy-design-system.json` (status "drawn, not built"). Each is one lab pass.

| # | element or signal | source | drawing |
|---|---|---|---|
| 1 | state grade | harness-adapters §3 | BUILT: reported solid, shim ringed, inferred dashed at 72%; inferred glows at half; grade from the harness record's state source |
| 2 | waiting kinds | OSC 7501 | BUILT: ◐ with a tiny kind mark (⚿ permission, ? question, ⚷ auth); the strip title spells it |
| 3 | delivery evidence | messaging | tick ladder on message rows; unknown is a hollow mark, never a failure |
| 4 | held count | mute | muted row shows "held N" |
| 5 | reconnect | mux state streams | stale frame dimmed under a "reconnecting…" pill; "restarted" chip on generation change |
| 6 | scrollback gap | ring eviction | hairline with "… N lines not retained" |
| 7 | pending row | MCP mutation status | dashed row with spinner until applied; rejected shows the reason |
| 8 | audit lines | Oracle audit log | centred stream lines: Oracle spawned / closed / interrupted X |
| 9 | interrupt | seat_interrupt | stop glyph in the strip (Oracle/operator), red bar sweep |
| 10 | spawn cap | per swarm | launch rows dim, banner says cap reached; Go to still works |
| 11 | purge | GUI only | the one confirm modal, typed seat name |
| 12 | sockets | seats are sockets | row menu "copy socket path"; listener count on the row |
| 13 | first run | identity v0.1 | root key + 24-word paper recovery shown once; enrollment and Google deferred |
| 14 | settings | config.toml | settings tab read-mostly |
| 15 | role and permissions | per seat | row menu, Oracle/operator |
| 16 | shadows | global messaging | module kept, deferred |
| 17 | nested subagent rows | 7501 / hooks | optional per harness |
