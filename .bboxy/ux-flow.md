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
**Rule:** the UI word for the block is swarm: SWARM bands, "+ new swarm". The vision vocabulary (space, cohort) is the Oracle's to update. **Where:** strings in `components.js` and `showcase.js`; `.bboxy/specs/ux-prototype.md` updated. CSS class names unchanged.

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
**Rule:** the Oracle's seventeen UI implications (state grade, waiting kinds, delivery ticks, held count, reconnect, gap marker, pending rows, audit lines, interrupt, spawn cap, purge, sockets, first run, settings, role menu, shadows deferred, nested subagent rows) live in the design system with status "drawn, not built", and in spec section 10. One lab pass each, in the order the operator chooses. **Where:** `design/bboxy-design-system.json` v2, `.bboxy/specs/ux-prototype.md` §10.

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

### Settings, second pass (designer agent)
**Rule:** the settings tab is the hero of identity: the GPG key block with the portrait beside it, subkeys, signatures; roles and harnesses as cards and tiles users can add to; appearance previews that apply live. Verified by rendering every section in both themes. **Where:** `settingsPage()` in `components.js`, settings block in `lab.css`, `applySetting` in `showcase.js`.

### The state glyph has a grade and waiting has a kind
**Rule:** the glyph's grade shows how the state is known: reported solid, shim ringed, inferred dashed and lighter (and glowing at half); the grade comes from the harness record's state source in Settings. A waiting glyph carries its kind as a tiny mark (permission, question, auth) read from the prompt in the pane; the strip spells it. **Where:** `glyph()` in `components.js`, `gradeFor` and the replay in `showcase.js`.

### ux-prototype-v2 is the implementation starting point
**Rule:** the operator declared the showcase ready; tagged `ux-prototype-v2`. The lab stays the parity target and keeps improving; implementation reads the spec, the design system and the lab. **Where:** git tag, design system `milestones`.

### Identity corrections from I2, and twelve implications folded
**Rule:** the trust path is GPG primary → certified device key → certified seat key (leaf certificates, not subkeys); first run selects and pins an existing primary and signs the bootstrap through gpg-agent, never creates a root or shows words as one; authority (expired, revoked, forked) is its own signal apart from state; held counts release only eligible items; pending, partial and unknown outcomes are spelled, never failures, never an automatic second paste; grace expiry removes only the live pane; caps cover swarm, machine and account; gap captions come from the supplied range; audit lines, listener counts and the interrupt / restarted notices are decided. **Where:** design system v10, spec §10, Account hero strings.

### Tasks under seats, epics in the dock
**Rule:** a delegated task is a row under its seat with the frontmatter status as glyph and chip, an accept pill when reported, and "waits on X" when blocked; an epic is a collapsible group in the stream dock with its specs and tasks; every delegation and status change is a stream line. **Why:** task_delegate and work-items.md, operator decisions. **Where:** `taskRow`, `epicGroup` in `components.js`; `delegateTask`, `acceptTask`, demo epics in `showcase.js`.

### The graph is a tab
**Rule:** ⌘G or Graph in the launcher opens a Graph tab: Agents (seat · role · now · blocked on) and the work tree (epics → specs and tasks with status, assignee, dependencies), generated from the same data as the rows and the dock. **Why:** the operator loves the Oracle's graph.md; the terminal should draw it live. Bug rows and invariant chips drawn for the next pass. **Where:** `graphPage()` in `components.js`, `openGraph` in `showcase.js`; design system v13.

### Artifacts are leaves, seats carry their count, the graph keeps the history
**Rule:** an artifact is a row under its task (kind glyph, ref, digest, responsible-seat chip from artifact_claim); a seat row shows "N artifacts" touched; the Graph tab has an artifact history card (time · seat · event · ref · grade) and artifacts as leaves in the tree. **Why:** operator decisions on first-class artifacts and terminal-detected artifact events. **Where:** `artifactRow`, `touched` in `seatRow`, history in `graphPage`; design system v14.

### Provenance is visible, and links carry their relation
**Rule:** the Graph tab has a Provenance card: created by / updated by, derived-from as annotated wikilink chips, the signed digest pair, and the hash-chained activity log with its signed daily checkpoint. Any rendered wikilink uses the annotation form `[[slug|display]]{rel:w}` as a chip with a relation badge. **Why:** provenance.md decision and the Obsidian-compatible link syntax. **Where:** `renderLinks`, `provenanceCard` in `components.js`; design system v15.

### Subscriptions are visible, external events have their lane
**Rule:** the Graph tab shows what each seat listens to (pattern chips, AUTO where delegation subscribed it); the stream dock ends with a collapsible external lane (github, ci, cloud, cron, webhook chips). **Why:** events.md, the spine. **Where:** `subscriptionsCard`, `externalLane` in `components.js`; key E in the demo; design system v17.

### A custom harness says it never submits on its own
**Rule:** on a custom or draft harness tile with wake pty, the Wake row's subtext spells what the input guard will do: hooks off → "never submits on its own" (messages wait for the operator's Enter), hooks on → "types after an idle hook". It changes live with the Hooks toggle. **Why:** harness-adapters R7: a hookless harness gets operator handoff, never an inferred Enter; the operator should know that before adding one. **Where:** `submitHint` in `settingsPage()` in `components.js`; lab state `settings.html?state=new harness`; design system v18. Lab pages now also take `?theme=` and `?density=` so `scripts/parity.sh` can drive every state headless.

### How far a message got is a tick ladder, and unknown is never a failure
**Rule:** a message row carries its delivery evidence after the time: · sender-stored, ✓ recipient-stored, ✓✓ with a faint second tick native-queued, ✓✓ native-written, ✓✓✓ in accent application-acknowledged. native-unknown is ✓ followed by a dashed hollow ring in the dim colour, never red, and its tooltip says it is never resent on its own. **Why:** messaging.md keeps six distinct evidence states and forbids treating an ambiguous write as failed or retrying it; the dashed ring reuses the "inferred" grade language of the state glyph. **Where:** `EVIDENCE`, `tickLadder()` in `components.js`; `message-row.html?state=evidence` and `?state=evidence climbing`; design system v19. Open: a room message has many recipients; the row shows one ladder (routed to the Oracle).

### A muted seat shows what it is holding, and unmute may not empty it
**Rule:** while muted, the seat row carries an amber "held N" pill counting swarm traffic held in both directions. Unmute posts its notice, releases only the eligible items (they arrive as unread), and says "released N, M still held" in the stream. Items whose old grant no longer permits sending stay as a dashed dim "held M" pill; hovering lists each original id and its reason. **Why:** messaging.md swarm mute: never drop held records, release under current authority only, keep the rest visibly blocked. **Where:** `held` in `seatRow()`, `toggleMute` and the M key in `showcase.js`; `seat-row.html?state=muted · held` and `?state=unmuted · still held`; design system v20.

### Running subagents nest under their seat, finished ones leave
**Rule:** a seat's running subagents (active or waiting) sit under its row, before its tasks: a branch hairline, the state glyph in the parent seat's colour with the grade of its source (OSC 7501 solid for Claude, hooks ringed for Codex), the description, and the last tool as mono detail. A waiting subagent carries its kind mark. More than eight shows seven and "+N running". A finished subagent leaves the sidebar; a harness that reports nothing shows nothing. **Why:** subagent-detection and the W2.3 brief, running only; the 1000-seat budget caps the rows. **Where:** `subagentRows()` in `components.js`, seeded on two seats in the showcase; `seat-row.html?state=subagents · claude`, `· codex`, `· 12`; design system v21. Note: subagent-detection still describes a 30 s linger for ended rows; the newer running-only rule wins here (routed to the Oracle).

### Evicted scrollback says where history starts, never how much was lost
**Rule:** where the byte ring evicted output, the pane body shows a hairline with a centred mono caption built only from the retained range the mux supplies: "earlier output evicted · retained from 09:14:07", or "earlier output unavailable" when no range comes with the gap. No line or byte count is ever shown. **Why:** mux-daemon ReadRecords/Gap returns a retained range and an explicit unavailable prefix; anything else would be invented. **Where:** `gapMarker()` and `fakeTerm` in `components.js`; new lab page `pane.html` (pane body: history and connection), states `gap · retained`, `gap · unavailable`; Rook's cloud pane in the showcase; design system v22.

### The logo is the identity PNG, everywhere; the SVG is gone
**Rule:** every logo is the operator's identity PNG, downscaled: the title bar, the lab index, the Account hero, the menubar, the logo page (16 to 512, nothing else). The SVG head silhouette, the ears-only mark, the ear-pair icon and the `brand_tilt_deg` token are deleted from the lab, the design sources and bboxybook. The sidebar-toggle fox with its ear tip is a toolbar glyph, not the logo, and stays. **Why:** operator, 2026-10-21: "PURGE SVG LOGO FROM EVERYWHERE", "use my identity logo, png version everywhere"; and "i love the ears around toolbar" for the toggle. **Where:** `logo()` in `components.js`, `logo.html`, `toolbar-icon.html` menubar state, `design/tokens.toml`, `crates/bboxybook/src/{gallery,modules}.rs`; design system v23 `elements.logo`.

### Losing the daemon never blanks a pane, and a restart says so once
**Rule:** when the GUI loses the mux, every pane keeps its last frame, dims it with the dim tokens, freezes its glow, and shows a centred frosted "reconnecting…" pill; no modal. On reconnect the frames redraw and the pill says "reconnected" and fades. A seat whose process generation changed shows an accent "restarted" chip on its row for 4 s, then the stream says "<name> restarted · process generation N". **Why:** mux-daemon: the GUI is a client that rebuilds from snapshots; the daemon stays authoritative; a generation change is news, not an alarm. **Where:** `pane()` opts `reconnecting`/`reconnected`, the frozen frame in `glow.js`, `restarted` in `seatRow()`; showcase key D; `pane.html?state=reconnecting`, `seat-row.html?state=restarted`; design system v24. Also: every lab `?state=` now accepts a slug (`subagents-claude`), the URL writes slugs, and an unknown state falls back to "all" with a console warning.

### A seat the Oracle is spawning is visible before it exists
**Rule:** an Oracle spawn shows at once as a dashed row in the swarm, with a spinner, the rolled name and "spawning · by Vesper Greywind". Applied: the row goes and the seat takes a tab of its own, with a stream line. Partial or unknown: the row stays dashed with a dashed ring and the outcome spelled on a second line ("no confirmation from the mux · never retried"). Rejected: a closed-strip row with a red × and the reason. Only the operator dismisses; nothing retries on its own. **Why:** mcp-control-plane mutation status accepted | applied | rejected | partial | unknown; accepted is not applied; ambiguity is never a failure and never a second attempt. **Where:** `pendingRow()` in `components.js`, sidebar hook `space.pending`; showcase key O (every fourth spawn rejected, every fifth unknown); `sidebar.html?state=pending-spawns`; design system v25.

### Stop is a request, and it says so
**Rule:** an operator or Oracle-class viewer sees a small square stop glyph in the strip, before the mute toggle; ⌘. does the same for the focused seat. Pressing it sends `seat_interrupt` and shows, once, a red bar sweeping along the strip and "interrupt sent" on the strip and the row for 3 s. The seat's state glyph does not change until the seat itself reports. Worker views get no stop glyph. One-shot notices carry their send time so a re-render resumes the animation instead of replaying it (the restarted chip too). **Why:** seat_interrupt maps to the adapter's Now/steer; permission belongs to the caller; sending is not stopping. **Where:** `titleStrip` opts `onInterrupt`, `seat.interruptSent`, `since()` in `components.js`; `interruptSeat` and ⌘. in `showcase.js`; `title-strip.html?state=stop-operator`, `interrupt-sent`, `worker-view`; design system v26.

### A seat shows how many clients are on its socket, never who
**Rule:** when anything is attached to a seat's socket (the MCP bridge, cron, a script, another machine), the row shows a tiny plug glyph and the count at its right; nothing at zero. The tooltip says "2 clients on this seat's socket · counts only". No names, keys or addresses anywhere in the GUI. **Why:** seats are sockets; the listener count is broker telemetry, decided counts only. **Where:** `ICON_SVG.plug`, `seat.listeners` in `seatRow()`; seeded on two showcase seats; `seat-row.html?state=listeners`; design system v27.

### Every seat has a menu you can reach without hovering
**Rule:** right-click, ⌘-click or long-press a seat row, or press ⇧F10 on the focused seat, to open a frosted menu headed by the seat: Focus, Split right, Split down; Assign role ▸ and Permissions ▸ as flyouts with a check on the current value, and Interrupt, only for an operator or Oracle-class caller; Mute; Copy socket path, with the path shown as a caption and "Copied" on success; Close in red. Arrow keys, Enter and Esc drive it; → opens a flyout. Role and permission changes post a stream line. **Why:** per-seat actions not worth a strip icon; vision: the Oracle sets roles and permissions; seats are sockets. **Where:** `rowMenu()` in `components.js`; contextmenu, ⌘-click, long-press and ⇧F10 in `showcase.js`; `seat-row.html?state=row-menu-operator`, `row-menu-role-flyout`, `row-menu-worker`; design system v28.

### At the spawn cap the launcher says which cap and still takes you places
**Rule:** when a cap is reached, the launcher opens with an amber banner: "Spawn cap reached · swarm term · 8 of 8", then what counts ("6 running · 1 starting · 1 in grace") and the way out (close a seat or raise the cap in Settings; Go to still works). Launch rows dim, lose their number key and refuse; trying shakes the banner. Go to, Graph and Settings stay live, and the selection starts on the first live row. An Oracle spawn past the cap lands as a rejected pending row with the same reason. **Why:** caps cover swarm, machine and account; reserved, starting and grace processes count; reconnect never respawns. **Where:** picker `opts.cap`, `capped`, `.capnote` in `components.js`; `capOf` and `SWARM_CAP` in `showcase.js`; `picker.html?state=cap-swarm`, `cap-machine`; design system v29.

### Purge is the only confirm, and it never says delete
**Rule:** Purge… sits at the bottom of the row menu for operator and Oracle-class callers. It opens the one confirm in the product: launcher material, a red "Purge seat" title, the seat, "Terminates now and moves its history to cold storage · nothing is lost", three facts (skips the 60 s grace with no undo; ring, checkpoints and events move to cold storage; a later resume can replay them by session id), and the seat name typed to enable the red Purge button. The drawn subtext said "nothing is deleted"; it was reworded because the rule says the d-word never appears. **Why:** purge ends a process without grace (mcp-control-plane: GUI only), and storage never deletes. **Where:** `purgeModal()` and the row menu item in `components.js`; `openPurge` in `showcase.js`; new page `purge.html`; design system v30.

### A work item reads as a document with its place in the graph beside it
**Rule:** any `.bboxy/` item opens as a document: a header railed in its type hue (epic violet, spec blue, task green, bug red, note grey, invariant amber, artifact teal) with type chip, status, path and title; the frontmatter as a quiet grid with wikilinks as annotated chips; the Markdown body; a provenance strip with who created and updated it, what it derives from, and its digest signed by the full fingerprint or marked unsigned in amber. A right rail draws its neighbourhood in the markdown graph (weights as stroke, backlinks dashed) and lists backlinks. Superseded items get a banner and stay readable; broken frontmatter gets a dashed red banner and is shown, never skipped. **Why:** work-items.md and provenance.md; the operator: "wikilinks is the way", "provenance in that system is very important". **Where:** `docView()`, `mdLite()`, `DOC_HUE` in `components.js`; new page `documents.html` with a state per type plus signed, unsigned, superseded, broken frontmatter; design system v31 `modules.document_view`.

### The bus is readable at a glance: root colour, bold kind, stored or fan-out
**Rule:** an event row reads left to right: seq per stream, time, ● stored or ○ fan-out, the subject with its root token in the root's hue and its kind (the last token) bold, the source in seat colour, a payload summary, then request-reply arrows (⇢ nonce on the request, ⇠ #seq on the reply; hovering one lights both), ×N when coalesced, and the signer tick. Fan-out events (inbox, seat output) carry no seq. A gap is an amber hairline row naming the consumer, the reason and the replay point, and a stream can be shown as one consumer sees it. Consumers are cards: durable or ephemeral, filter patterns, start, ack policy, deliver, cursor and a pending bar against queue_max. **Why:** events.md Bus design decided (NATS grammar, one sequence per stream, two tiers, consumers, request-reply with no_responders). **Where:** `eventRow()`, `subjectChip()`, `consumerCard()` in `components.js`; new page `events.html` (idle, busy live, slow-consumer gap, external burst, request-reply); design system v32.

### The launcher has its own page, one implementation
**Rule:** `launcher.html` shows ⌘T at every nesting level, each deep-linkable: root, typed filters ("co", "ssh rig cod"), Claude ▸ placements, SSH ▸ machines, SSH ▸ rig ▸ harnesses, SSH ▸ rig ▸ Codex ▸ placements, Cloud ▸ regions, Go to ▸ seats and swarms, Settings ▸ sections, the split-right and split-down presets with the lit-cell banner, and the cap. The item tree moved out of the showcase into `launcherItems()` and the launcher styles moved out of `showcase.html` into `lab.css`, so both pages run the same code. The picker takes `path` to open inside a level. **Where:** `launcherItems()`, picker `opts.path` in `components.js`; `lab.css` launcher block; design system v33 `modal.implementation`.

### The Graph tab has its own page, and demo data lives in one place
**Rule:** `graph.html` shows the Graph tab whole and card by card (agents, work tree, subscriptions, provenance, artifact history) plus an empty swarm; `graphPage()` takes `only` to render a subset. The showcase's demo data (artifact history, subscriptions, external events, provenance) moved into `fixtures.js`, together with a seats-and-epics set for the dedicated pages, so every page reads the same fixtures. **Where:** `graphPage(opts.only)` in `components.js`; `fixtures.js` loaded by `showcase.html`, `graph.html`; design system v34.

### The stream dock has its own page
**Rule:** `stream.html` shows the dock at sidebar width: the whole swarm stream, epic groups open and collapsed, the external lane, messages alone, filtered to one participant, and empty. Presence notices centre their wrapped text in the narrow dock. The dock CSS moved from `showcase.html` into `lab.css` and the stream messages into `fixtures.js`. **Where:** `epicGroup()`, `externalLane()`, `messageRow()`; `stream.html`; design system v35 `modules.stream_dock`.

### The title bar has its own page, one component
**Rule:** `titlebar.html` shows the app title bar: default, sidebar closed (the fox-ear toggle hollow), a long crumb that ellipsizes, a narrow window, and an unfocused window (dimmed dots, desaturated portrait). `titleBar()` builds it for both the page and the showcase; its CSS moved into `lab.css` as `.titlebar.app-bar`, so the toolbar-icon page keeps its small mock bar. **Where:** `titleBar()` in `components.js`; design system v36.

### Diagrams are ours: Mermaid syntax, the chrome look
**Rule:** a ```mermaid block in any document is drawn by us, not a browser: flowcharts (TD and LR), sequence and state diagrams. Nodes are rounded and tinted by their class (:::oracle, :::executor, :::creativity, :::accent, state colours); edges are hairlines with rounded orthogonal bends and arrowheads, dashed for -.->, heavy for ==>, with labels on pills above the lines. Long edges route around nodes through virtual waypoints; each back edge takes its own lane on the side nearer its target and leaves from the top when its layer blocks the side. Unsupported syntax (gantt today) shows its source with a note, never a broken diagram. Holding ⌘ over a path token in a pane underlines it with a "⌘-click opens" pill. **Why:** document-viewer.md: Mermaid built ourselves, rendered with chrome primitives; path detection like iTerm2 semantic history. **Where:** `parseMermaid()`, `diagram()`, mdLite fences in `components.js`; new page `diagrams.html`; `documents.html?state=with-a-diagram`; `pane.html?state=hover-path`; design system v37.

### A document opens as a modal you feed into a seat
**Rule:** ⌘-click on a `.md` path, a wikilink in the stream, or a work item opens the document as a modal in the launcher material, centred over the blurred window: the document view scrolls (arrows, space, page keys), and a foot holds the actions, primary first: Feed into <focused seat> ⏎, Feed selection ⌘⏎, Copy as Markdown ⌘C, Copy path ⌘⇧C, Find ⌘F, then quiet Pin as tab and Close esc. Feeding posts an operator message to the focused seat with the document as a wikilink chip, the tick ladder climbs, the modal closes and focus returns exactly where it was. Plain Markdown outside `.bboxy/` gets the plain view without frontmatter or rail. Message rows now render wikilinks as chips. **Why:** operator: "it should be a modal; open, scroll, copy maybe, close, feed into the harness". **Where:** `docModal()` in `components.js`; showcase key V; `documents.html?state=as-modal`, `modal-plain-markdown`; design system v38.

### Every subagent is a message target, and the row says who it was for
**Rule:** right-click (or long-press) a subagent row for its menu: Message this subagent (with "relayed via main: claude cannot address subagents" under it), Message the main agent, Focus the seat, Copy agent id. A message row names its target with a chip, "→ main" or "→ ↳ <subagent title>"; a reply from a subagent carries "↳ <title>" in the role colour; "via main" in italics marks a relay through the main agent; the tick ladder is per target. A message to a subagent that ended shows an amber dashed "held · agent ended" instead of the ladder and is never redirected on its own. **Why:** messaging.md Subagent addressing (operator decision), ref `runs/ref-iterm-background-task.png`. **Where:** `messageRow` (`to`, `fromAgent`, `via`, `held`), `rowMenu` opts `agent`, subagent row data in `components.js`; `openAgentMenu` in `showcase.js`; `message-row.html` four new states, `seat-row.html?state=subagent-menu`; design system v39.

### The launcher shows matches as plain text
**Rule:** no underline or highlight on the matched part of a launcher row; every row is plain text and the only emphasis is the current row's selection band, as in Raycast. **Why:** operator tweak, reference `runs/ref-raycast-no-match-underline.png` (ux-pick.md). **Where:** `picker()` label in `components.js`, `.picker .opt .m` removed from `lab.css`; applies to picker.html, launcher.html and the showcase; design system v40 `modal.matching_display`.

### The Oracle's actions show in the stream
**Rule:** every Oracle effect a swarm member may see becomes a centred stream line: the Oracle mark in its role colour, then "Vesper Greywind spawned Odile Marsh"; under it the short op id, the outcome when it is not applied (rejected in solid red, unknown and partial in dashed amber) and the time; hovering gives the reason. **Why:** audit_lines decided as a projection of the audit log scoped to swarm members (mcp-control-plane). **Where:** `messageRow` `m.audit` in `components.js`; `FIX.audit`; `stream.html?state=audit-lines`; showcase key O writes an audit line per spawn outcome; design system v41.

### A seat whose authority failed says why and keeps running
**Rule:** when a seat's signing authority is expired, revoked or forked, a struck key sits after its name (amber for expired, red for revoked and forked) and its strip names the cause in a dark pill: "authority expired · renew the seat grant", "revoked here · peers learn on next contact", "forked · two heads at one epoch · the operator resolves". The state glyph is untouched: the harness keeps running. Only controls that need the seat's own signature are disabled, with the same reason: the mute toggle in the strip and Mute in the row menu, because mute and unmute notices are seat-signed. **Why:** identity.md authority is separate from activity; messaging.md signs mute notices with the seat key. **Where:** `ICON_SVG.keyx`, `AUTH_SAY`, `authMark` in `components.js`; disabled items in `rowMenu`; seat-row and title-strip states; Sable Quill in the showcase; design system v42.

### Bugs look like tasks with a bug, and invariants name what they bind
**Rule:** a bug is a task row with a red bug icon in place of the status glyph (green when fixed, dimmed when wontfix or duplicate) and a red "breaks <invariant>" chip when it violates one. A task or spec an invariant constrains carries an outlined amber "inv <title>" chip. In the Graph tree an invariant is its own ◆ line with what it binds beneath it, constrained items and the bugs that violate it. In the narrow sidebar the title wins and the chip truncates. The bug is an inline icon, not a font glyph, because the mono font lacks ⬣. **Why:** work-items.md types bug and invariant with typed links violates and constrains. **Where:** `taskRow` (type bug, violates, constrains), `invChip()`, the Graph tree in `components.js`; `ICON_SVG.bug`; `FIX.epics`; seat-row and graph states; design system v43.

### First run asks who you are and uses the key you already have
**Rule:** the first screen, in the launcher material, asks "Who are you?" and lists the secret keys in the GnuPG keyring: uid, the full 40-hex fingerprint in groups of four, algorithm, created, expiry and where the secret lives. Three steps show progress: pin your GPG primary, certify this Mac's device key, write account head epoch 1. "Pin this key" signs one bootstrap record through gpg-agent; if pinentry is needed it asks in its own window and this screen only waits. Then "You are Boxy". With no secret key the screen shows the gpg command and creates nothing. With the primary elsewhere (public certificate only) it makes a signed enrollment request, shows its path, digest and the sign command for where the primary is, and runs local seats only until the signed request is imported. RSA primaries show dimmed with the identity.md gap. No words are ever shown as a root. **Why:** identity.md bootstrap (I2) and the settings identity rules. **Where:** `firstRun()` in `components.js`; new page `first-run.html` with six states; design system v44.

### The window can be see-through, panes only
**Rule:** Settings › Appearance has Window opacity (50 to 100%, a slider with a live preview of a little window over a wallpaper; dragging previews, releasing commits), Blur behind, and Blur radius (10, 20, 40 px) when blur is on. Only pane backgrounds take the alpha; the sidebar, title bar, strips and all text stay opaque, and the glow and dim composite over. The showcase shows it over a wallpaper that stands in for the desktop (with a faint grid so blur visibly softens it): `showcase.html?opacity=0.6&blur=1`. **Why:** operator: "semi transparency like iTerm2"; renderer.md §1b. **Where:** `[window]` in `design/tokens.toml` (tokens.js regenerated; bbt-ui-model 30 tests pass); Appearance rows in `settingsPage()`; `applyWindow()` in `showcase.js`; `lab.css` transparency block; design system v45 `foundations.window`.

### A closed seat can come back after its grace, as a new process
**Rule:** when the 60 s grace ends, the seat is retained, not forgotten: a stream line says "closed · history retained · Resume in ⌘T". The launcher gains Resume ▸ with each retained seat, its closed time and the exact resume command (`claude --resume <id>`, `codex resume <id>`), or "no resume: a new shell, history replayed above it" for zsh. Resuming counts toward the spawn cap, starts a new process generation, opens it in a tab of its own with focus, and the pane shows a "resumed · process generation N · <command>" hairline above the new output; the stream says who resumed it. Undo during grace stays the same live process; this is the after-grace path. Also fixed: without WebGL the showcase returned early and skipped its keys, stream and log replay; only the glow is skipped now, and new seats share `dressSeat()` for colour and grade. **Why:** mux-daemon close lifecycle `closed_grace → closed_retained`, resume by explicit request with retained history; the previous session's open gap. **Where:** `launcherItems` `ctx.retained`, resumed marker in `fakeTerm` (`components.js`); `retain`, `resumeSeat`, `dressSeat`, `paintTerm` keeps markers (`showcase.js`); `launcher.html?state=resume-recently-closed`, `pane.html?state=resumed`; design system v47.
