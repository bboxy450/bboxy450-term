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
