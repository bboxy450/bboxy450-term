# UX flow — rules and thinking, recorded as we go

Owner: operator, written down by Lumen Foxglove <CREATIVITY> during direct review in the lab. Each entry: the rule, why, where it lives. Newest at the bottom. Oracle folds decisions into [[ux-pick]]; this file keeps the reasoning. Lab: `lab/web/showcase.html`. Tokens: `design/tokens.toml`.

## 2026-10-21, direct review session (ICQ muted for focus)

### No demo legend in the window
**Rule:** nothing in the product chrome explains demo keys. **Why:** it overlapped seats and told the user nothing. **Where:** keys stay live; the list sits on `lab/web/index.html`.

### Unfocused panes grey out hard
**Rule:** unfocused pane content and strip desaturate and fade, background takes a grey wash; border and glow keep the seat colour. **Why:** iTerm2's inactive-pane read, pushed 20% further by the operator; identity must survive on the border. **Where:** `[dim]` in tokens (saturate 0.10, opacity 0.45, wash 0.28), `lab.css` `.pane.unfocused`.

### Finish language: the 2023 Range Rover rear
**Rule:** flush surfaces, hairline gaps, one thin gloss band with wide-tracked small caps set into the body, nothing protrudes, no chevrons, no left colour bars. **Why:** the operator's reference; "chevron on the left is overused". **Where:** `lab.css` finish block.

### The cohort block is the car
**Rule:** a cohort (space) is a rounded body. Its band is set into the top. Its seats sit on the body with no hairlines. The bottom is a bumper. **Why:** first pass put bands and hairlines everywhere and read as noise; the shape of the car around the block is the point, not its details. **Where:** `.space`, `.space-head`, `.space-body`.

### Cohort colour: band and bumper, no line
**Rule:** each cohort has one colour, random from its key, user-changeable (swatch at the right of the band). The band and the bumper carry it. No accent line anywhere. **Why:** a line under the band plus a bumper read as "line and chevron" again. Colour as mass, not as stroke. **Where:** `--space` from `seatColor(space)`, `.space { border-bottom: 6px solid var(--space) }`, swatch cycles `space.color`.

### Sidebar glow is a separate, quieter budget
**Rule:** rows glow with radius 4, 30% of pane alpha, particles only on active rows (4), idle rows do not glow. Panes keep the full effect. **Why:** 20 px rows with overlapping halos mix colours into dirt. **Where:** `[glow] row_*` tokens, `uMul` per-rect multiplier in `showcase.js`.

### One set of window controls
**Rule:** the title bar owns the three dots. The sidebar header has none. **Why:** two sets top-left is weird. **Where:** `components.js` sidebar header; showcase hides the header entirely since the title bar names the app.

### Every pane has a macOS-style close
**Rule:** a red dot at the left of the pane strip kills the seat: the split collapses into its sibling, the row leaves the sidebar, focus moves to the first pane. It greys with the strip when unfocused, like macOS. **Where:** `.strip .close`, `closeSeat()`.

### Bottom-left row is Arc's
**Rule:** archive box · one dot per cohort (current filled in its colour) · plus. Dots switch cohorts. **Why:** the operator's Arc reference. The seat-count band was dropped. **Where:** `.sb-foot`, `onSwitchSpace`.

### "+ new seat ⌘T" lives inside the active cohort
**Rule:** the new-seat row is the last row of the open cohort, like Arc's New Tab inside the space. **Where:** `.tab-row.newseat`.

### Logo is the portrait
**Rule:** the identity PNG (`~/.agents/identity/bboxy.png`, lab copy at 512 px) is the mark: circle in chrome, rounded square as a tile. **Why:** the SVG silhouette was meh. **Where:** `C.logo()`, `lab/web/bboxy.png`.

### A cohort is a world
**Rule:** a cohort has its own seats, agents, participants and messaging. Switching cohorts switches the sidebar body, the stream and the panes. **Why:** the dots are not sections of one list; each is a different set. **Where:** `space()` in `showcase.js`, per-space `participants` and `stream`.

### Cohorts read as tabs
**Rule:** every cohort band shows in the sidebar; only the active one opens to its seats; clicking a shut band makes it active. A shut band is a thin tab with its roll-up glyphs and waiting count. **Why:** the first screen should still show both cohorts; switching should feel like tabs. **Where:** `.space.closed`, `st.onSwitch`.

### A big body-coloured button creates a cohort
**Rule:** under the cohort blocks, a full-width rounded button "+ new cohort" in the body colour. The footer plus does the same. **Where:** `.new-cohort`, `newSpace()` (a new cohort starts with one zsh seat).

### Standing rules carried over from the Oracle's table
Signal semantics: locate ripple only on click or keyboard focus (pane ripple from the pane centre, row ripple from the row's left edge); glow = state; focus = brightness; unread = dot; hover = highlight only; no timers in demos, keys change one thing at a time. Every pixel is precious: 2 px gutters, no outer padding, glow paints over edges and takes no layout space.

### Hover on a cohort band stays in its colour
**Rule:** hovering a band lightens its own colour; it never turns white or grey. **Why:** a white band on hover lost the cohort identity. **Where:** `.space-head:hover` gradient in `lab.css`.

### The group of other people's harnesses is called "shadows"
**Rule:** the sidebar group label reads SHADOWS, not participants. **Why:** they are messages from the external world, present but not yours; the word matches the grey and the Dark Souls ghost the idea came from. **Where:** `components.js` space pill; the spec term stays "phantom seats" in [[ux-pick]].

### Open/shut and focus are two different actions
**Rule:** clicking a cohort band only opens or shuts that block. Clicking a seat only focuses it; the stage follows the focused seat into whichever cohort it lives in, and no block opens or shuts because of it. A footer dot is a jump: it opens that cohort and focuses its first seat. **Why:** coupling them made a focus change collapse the cohort the operator was looking at. **Where:** `open` set + `toggleSpace` / `jumpSpace` in `showcase.js`; `onToggleSpace` in `components.js`.

### Breathing room between cohort blocks
**Rule:** cohort blocks sit 8 px apart, independent of row density. **Where:** `layout.cohort_gap` token, `.space` and `.new-cohort` margins.

### Hover on the left never changes the view; hover on a pane focuses it
**Rule:** hovering a sidebar row only highlights it; focus from the sidebar is click only. Hovering a pane for 150 ms focuses it (brightness only, no ring), and typing blocks that for a second. **Why:** dwell on the sidebar walked the operator through views while reading. Dwell on panes is the normal focus-follows-mouse feel and stays, as in [[ux-pick]]. **Where:** `armHover` bound to panes only in `showcase.js`.

### No space dots in the footer
**Rule:** the footer is archive on the left, plus on the right, nothing between. **Why:** Arc's dots exist because Arc hides the other spaces; here every cohort is a visible band, so dots only repeated the bands and multiplied as cohorts were created. **Where:** `components.js` sidebar footer.

### Everything resizes by drag
**Rule:** the sidebar's right edge drags between 160 and 520 px and the width persists. Every split gutter is a divider: drag it to change the ratio (10% to 90%); the split tree keeps the ratio so the sidebar mini-grid follows. The 2 px gutter has an 8 px grab zone; nothing visible is added. **Where:** `.divider`, `.sb-resize` in `showcase.html` and `showcase.js`.

### Landing on a new screen still shows the locate ring
**Rule:** when a seat click switches the panes to another tab or cohort, the locate ripple fires after the switch, on the row and at the centre of the pane that appeared. **Why:** the operator wants the circle to say what they are looking at, especially when the view just changed. **Where:** `locate()` in `showcase.js`, called after `render()` on a stage switch.

### Locate is directional
**Rule:** clicking a row rings its pane (circle from the pane centre). Clicking a pane bounces its row: a quick scale and a border in the seat colour around the row item, no ring. Keyboard focus does both. Nothing locates itself. **Why:** the circle should point to the thing you did not click. **Where:** `locate(key, where)` and `.seat-row.bounce` in the showcase.

### Closing a seat is reversible (operator decision, also in vision)
**Rule:** close puts the seat in a 60 s grace state: the pane leaves the split, the row becomes a collapsed closed strip in its cohort (struck name, countdown, undo). Undo or ⌘Z restores it in place with its split position and refocuses it. After the window the seat is gone. No confirm dialog; the undo window is the confirmation. **Where:** `closeSeat` / `undoClose` in `showcase.js`, `.closed-strip` in `components.js`; the mux and MCP specs carry the process side.

### Not XFCE: macOS proportions at the same footprint
**Rule:** the showcase runs a `cozy` density: 24 px rows, 12.5 px text, 14 px icons, inset rounded selection, sentence-case semibold section labels, band 24 px with softer tracking, bumper 4 px. Compact (20 px) stays for the 1000-seat reading. **Why:** 20 px rows with 10 px tracked labels and a thick bumper read like XFCE. **Where:** `[density.cozy]` in tokens, macOS block in `lab.css`.

### Cohorts are called swarms
**Rule:** the UI word for the block is swarm: SWARM bands, "+ new swarm". The vision vocabulary (space, cohort) is the Oracle's to update. **Where:** strings in `components.js` and `showcase.js`; `.kb/specs/ux-prototype.md` updated. CSS class names unchanged.

### Nothing in the sidebar moves on focus
**Rule:** the "+ new seat ⌘T" row is in every open swarm, not only the focused one. **Why:** a row that appears where focus is makes the whole list jiggle on every focus change. **Where:** `components.js` space pill.

### Unfocused swarms dim like unfocused panes
**Rule:** the swarm holding the focused seat is full contrast; every other swarm's band and rows desaturate and fade with the same `[dim]` tokens as panes, the body washes toward grey, the bumper keeps its colour. Hover lifts an unfocused swarm a little. **Why:** the same reading left and right says what is focused at a glance. **Where:** `.space.unfocused` in `lab.css`, class set in `components.js`.

### The sidebar has its own ground
**Rule:** the sidebar background is a token pair: lighter than the window in dark mode (#1A1E3C under #12142A blocks), darker than the window in light mode (#E6E4F5 under white blocks), so swarm blocks contrast with their ground in both themes. **Where:** `color.*.sidebar` tokens, `--sidebar-bg`.

### Swarms are named in place
**Rule:** double-click a swarm band to rename it; Enter or blur commits, Escape cancels. A new swarm is born as `swarm N` with the name field already open. **Where:** `spaceHeader` in `components.js`, `editName` in `showcase.js`.

### One icon for the theme
**Rule:** the title bar has one theme icon: sun in dark mode, moon in light mode, click flips. No auto label, no three-state cycle; the OS preference only sets the start. **Why:** it is a demo, simplify. **Where:** `themeBtn` in `showcase.js`.

### No sidebar footer
**Rule:** the sidebar has no bottom row. No archive, no plus. The big "+ new swarm" button under the blocks is the one way to create a swarm. **Where:** `components.js` sidebar; the stream dock now sits at the very bottom.

### Launching a seat is a Spotlight / Raycast moment
**Rule:** "+ new seat" or ⌘T opens a centred launcher over a blurred window: big search field, rows grouped under small section labels (Harnesses, Shells; later Browser, Remote) with an icon, a name and a kind tag on the right, prefix filter, ↑↓ and Enter, Escape closes, the rolled character name with a reroll and the key hints in the footer. Enter launches the seat as a new tab in that swarm, focuses it and locates it. The list is data so new launchables (browser tab, vnc, ssh) are rows, not new UI. **Why:** operator asked for Spotlight; Raycast is the reference for the row anatomy. **Where:** `C.LAUNCH` and `picker()` in `components.js`, `openSpotlight()` in `showcase.js`.

### The visual style is a file, and it is Raycast's material
**Rule:** `design/style.json` holds the style: one frosted material (translucent, blurred, hairline-edged), one radius scale (panel 20, card 12, row 10, tile 8, badge 6), one type scale, one spacing grid, selection as a foreground tint, icons in coloured tiles, numbers in badges, kinds in muted text on the right, shadows only under floating things, one motion curve. The generator feeds it into `tokens.js` and `lab.js` exposes it as CSS variables; the showcase stylesheet uses nothing else. Applied to the title bar, sidebar ground, swarm cards (band and bumper kept), seat rows (tile · name · title · badge · glyph), new-seat and closed strips, the new-swarm button, the stream dock and the launcher. **Why:** the operator pointed at Raycast as the look; a file keeps every surface on the same values. **Where:** `design/style.json`, `lab/web/showcase.html` stylesheet, `components.js` seat-row tile.

### Splits: from the launcher by key, or by dragging onto a pane
**Rule:** in the launcher, ↵ opens the seat as a new tab, ⌘↵ splits to the right of the focused pane, ⌘⇧↵ splits below it; the action pill names all three. By drag: pull a seat row from the sidebar or a pane by its strip onto another pane; the hovered half (left, right, top, bottom) tints, dropping splits there at 50/50. A dragged seat is moved: its old split collapses. The new or moved seat gets focus and the locate signal. **Why:** the operator asked where splits live; keyboard in the launcher and drag on the panes are the two natural places. **Where:** `placeSeat()` and the drag code in `showcase.js`, pill in `picker()`.

### ⌘D and ⌘⇧D split like iTerm, then ask which harness
**Rule:** ⌘D opens the launcher preset to "split right of <focused seat>", ⌘⇧D to "split below"; the placeholder names the placement, ↵ launches the chosen harness there, ⌥↵ opens a tab instead, and the pill shows the preset actions. ⌘T remains "new tab", with ⌘↵ / ⌘⇧↵ as the split variants. **Why:** iTerm muscle memory. **Where:** `openSpotlight(sp, preset)` in `showcase.js`, `actions` in `picker()`.

### ⌘T also goes places
**Rule:** the ⌘T launcher lists, under "Go to", every seat (name, swarm · title, harness tile) and every swarm (name, seat count, swarm colour tile). Typing filters launchables and destinations together; ↵ on a seat focuses it with the locate signal, ↵ on a swarm opens it and jumps to its first seat. ⌘D / ⌘⇧D presets stay launch-only. **Where:** `openSpotlight` items in `showcase.js`; `matches()` in `picker()` matches label prefix, any word, or the subtitle.

### Mute is a mark on the row and a toggle on the strip
**Rule:** ⌘M, or the speaker icon in the pane strip, mutes the focused seat: it works 1:1 with the operator, its row stays where it is with an amber muted mark and a dimmed name, and the swarm stream gets a centred presence notice ("<name> muted · 1:1 with the operator"); unmute posts the matching notice. Nothing moves, nothing glows differently. **Why:** the operator's vision decision, lifted into the sidebar like `icq mute`. **Where:** `toggleMute` in `showcase.js`, `.mutemark`, `.strip .mute`, `.msg.presence`.

### You see the split before it happens, and the new pane is ready to type
**Rule:** while the ⌘D / ⌘⇧D launcher is open, the half of the focused pane where the new seat will land breathes in the accent tint behind the modal, and the launcher's input row shows a two-cell split diagram with the new cell lit. After launch the new pane is focused, the locate ring fires at its centre, and its cursor blinks; the native app also warps the pointer to the pane centre (the lab cannot move the OS pointer). **Why:** the operator wanted splitting to be unmistakable in the modal and the new seat typeable at once. **Where:** `.pane.will-*`, `.split-preview`, `.pane.focused .cur` in `showcase.html`; `openSpotlight` in `showcase.js`.

### The launcher wears a banner that says what will happen
**Rule:** above the launcher panel sits a banner, same material, large. For ⌘D / ⌘⇧D it draws the real pane layout with the new cell breathing in accent and reads "Split right / Split down · of <seat>". For ⌘T it draws the swarm's tab strip with a new lit tab at the end and reads "New tab · in <swarm>". The small in-row diagram is gone; the banner is the indication. **Why:** the operator wanted splitting in their face, with the space above the modal used. **Where:** `.spot-banner`, `.tab-strip` in `showcase.html`; `openSpotlight` in `showcase.js`.

### The launcher nests, and typing flattens the nest
**Rule:** rows can hold sublists. Every harness row opens to its placements (New tab, Split right, Split down); a "Go to" row opens to every seat and swarm. → or Enter on a parent dives in (a crumb chip shows the path), ← or Backspace on an empty field climbs out, Escape climbs before it closes. Enter on a harness row still launches with the default placement. Typing flattens: "goto nyx", "go nyx", "codex split" filter straight into the sublist. Number keys pick the nth visible row. Works in every launcher, ⌘D presets included. **Why:** the operator wants "goto <name>" to work anywhere and some options to carry nested choices. **Where:** `picker()` in `components.js` (children, stack, scope), items in `openSpotlight`.

### The design system is a file: `design/bboxy-design-system.json`
**Rule:** foundations (material, radius, type, spacing, colour roles, motion, density, theme), signals, elements (tile, badge, kbd, glyph, role mark, swatch, hairline, pill, crumb, chevron, band, bumper, close dot, mute toggle, muted mark, mini-grid, ghost, drop zone, divider, cursor), modules (title bar, sidebar, swarm card, seat row, tab row, new-seat row, closed strip, shadows, new-swarm button, stream dock, pane, title strip, banner, glow canvas) and the modal as its own module with anatomy, rows, keyboard, typing and rules, plus interactions and gaps. Values point at `tokens.toml` and `style.json`; this file is the structure. **Where:** `design/bboxy-design-system.json`.

### The specs' implications are drawn before they are built
**Rule:** the Oracle's seventeen UI implications (state grade, waiting kinds, delivery ticks, held count, reconnect, gap marker, pending rows, audit lines, interrupt, spawn cap, purge, sockets, first run, settings, role menu, shadows deferred, nested subagent rows) live in the design system with status "drawn, not built", and in spec section 10. One lab pass each, in the order the operator chooses. **Where:** `design/bboxy-design-system.json` v2, `.kb/specs/ux-prototype.md` §10.

### Where a seat runs is a prefix, and the launcher knows remote
**Rule:** cloud seats get a cloud glyph and ssh seats a server glyph before the harness tile on rows and strips; local seats get nothing. In the launcher, SSH and Cloud are parent rows under Remote: → or typing opens the harnesses, one more level opens the placements; "ssh codex" and "cloud claude" type straight through. The launched seat carries where and host. **Where:** `.where` in `components.js`, `remoteRows` in `showcase.js`.

### SSH picks the machine first
**Rule:** SSH ▸ machine ▸ harness ▸ placement, Cloud ▸ region ▸ harness ▸ placement. Machines come from `~/.ssh/config` in the real app. "ssh rig codex" types through all three levels. **Where:** `HOSTS`, `remoteRow` in `showcase.js`.

### Typing matches meaning, in tiers
**Rule:** Go to filters by facet as well as name: harness, where, host, role, state, swarm, muted ("goto codex", "goto ssh", "goto cloud", "goto waiting"); words are ANDed; fuzzy subsequence catches "nyxc"; in the real app a fifth tier uses the Jev encoder (ICQ's local pocket-encoder MiniLM, 384-d, no network) to add "Related" rows by meaning under the lexical hits. Not in the web demo. **Why:** "codex" under Go to returned nothing; the operator wants smart matching and the encoder is already on the machine. **Where:** `wordHit`/`tier` in `picker()`, `tags` on go-to items, design system v3 `modal.matching` and `modal.matching.jev`.

### Jev orders as well as matches
**Rule:** with an empty field the launcher's Suggestions are the three rows closest to the context vector (focused seat, swarm, last picks, last messages); with a query Jev breaks ties inside a tier; habit counts order placements; nothing reorders within 300 ms of a keystroke or re-numbers reachable rows. **Where:** design system v4 `modal.matching.jev.ordering`.

### Settings are a tab, in the same material
**Rule:** ⌘, or Settings in the launcher opens a settings tab in the current swarm: left nav with search, cards. Account carries sign-in (Google, deferred to cloud), the root key, devices; Cloud carries providers; Machines carries ssh hosts; Roles carry default system prompts and send permissions; Harnesses, Appearance, Shortcuts follow. Read-mostly from config.toml, changed rows dotted with reset. **Why:** the operator asked for settings with login, cloud, ssh machines, roles and their default prompts. **Where:** `settingsPage()` in `components.js`, `openSettings` in `showcase.js`, `.settings` in `lab.css`.

### Arrows walk the panes
**Rule:** arrow keys move focus to the nearest pane in that direction, chosen by alignment first and distance second; ⌘⌥ arrows while a field has focus (iTerm), plain arrows otherwise. Keyboard moves locate on both sides. **Where:** `neighbour()` in `showcase.js`.

### Identity is the GPG key
**Rule:** the operator's existing Ed25519 GPG key is the identity: fingerprint and uid are the hero of Account, devices are subkeys with revoke, seats sign under the roster, commits and messages are signed, Google is a secondary link for cloud. **Why:** the operator loves identity based on the GPG key; lean in. **Where:** Account hero and Signatures card in `settingsPage()` (designer pass); identity spec alignment requested from the Oracle.

### What the Account screen must not promise (Oracle, GPG root)
**Rule:** nine constraints, now in the design system under settings_tab.identity_rules: no create/export of the root; enrollment handles "primary elsewhere" with a signed request; unlock is pinentry, never our password field; "revoked here, peers learn on next contact"; recovery restores identity only; full 40-hex fingerprint, never a short id; "signed by Boxy" means the chain verifies; Google is a link, not a sign-in; per-seat commit signing switchable, default off. **Where:** `design/bboxy-design-system.json` v6; the Account hero follows them.

### Users define their own roles
**Rule:** Settings › Roles has a "+ new role" card: name, colour, mark, default system prompt, may-send-to, and a priority class (oracle class / worker class) because delivery order and badging follow the class, not the name. Built-ins are editable, not deletable, chipped "built-in". Harness default-role selects list custom roles. **Where:** `settingsPage()` Roles (designer pass); class semantics routed to the Oracle for messaging/MCP.

### Users add their own harnesses
**Rule:** Settings › Harnesses has a "+ new harness" tile: name, command, tile colour, icon, default role, wake method (default pty), state source (default output cadence), hooks. Built-ins are chipped "built-in". A user-added harness runs through the generic PTY adapter and shows in the launcher. **Where:** `settingsPage()` Harnesses (designer pass); adapter semantics routed to the Oracle.

### Appearance settings apply live
**Rule:** density, theme and gutter chosen in Settings › Appearance apply to the window at once; the density control previews its four row heights before you pick. **Why:** the operator wants density as a real setting, not a lab toggle. **Where:** `settingsPage({ onChange })` → showcase applies tokens and re-renders.
