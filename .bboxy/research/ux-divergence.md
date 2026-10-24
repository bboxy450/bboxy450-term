---
id: "1G4A4NV756SFSJ2TS3AYET8QWS"
type: "research"
number: 68
title: "UX divergence — the field before the pick"
created: "2026-10-21T18:54:36Z"
updated: "2026-10-24T00:10:38Z"
---
# UX divergence — the field before the pick

Date: 2026-10-21. Author: Lumen Foxglove <CREATIVITY>. Status: FIELD. Nothing here is chosen.
Source: [[vision]] vocabulary (seat, space, tab, split, cohort). References read: `runs/ref-arc-sidebar.png`, `runs/ref-iterm-grouped-tabs.png`, `runs/ref-iterm-session-status.png`, `~/.agents/identity/bboxy.png`.

Method: generate raw, judge later. Each section lists shapes 1..n, the outside-domain source they came from, and the Censor's objections logged at the end rather than obeyed. Oracle selects. Sketches are ASCII; widths are gestures, not specs.

What the references actually show, measured by eye:
- Arc: sidebar is a soft tinted panel, pinned favourites as a 3-wide icon grid, a space name as a quiet label, `+ New Tab`, and a bottom bar with Settings and History as two pill buttons. Almost no chrome. The tint is the identity.
- iTerm2 grouped tabs: colored outlines group tabs by project; each tab carries name, state word (`Idle`, `working`), a dot and a shortcut. Groups are strong, states are weak, text does the work.
- iTerm2 session status plugin: one flat list, dot + kind + title + harness + shortcut + state word + a two-line preview of the last message. Readable at 15 rows, dead at 100.

---

## 1. Left sidebar at 10 seats

Context: one or two spaces, a handful of tabs, each tab one seat or a two-way split. Everything fits on screen. The design question is personality and glanceability, not density.

1. **Arc verbatim.** Tinted panel, space label, tabs as rows, each row = harness icon · character name · title · state glyph. Nothing else. Source: Arc.
2. **Theatre seating chart.** The sidebar is literally a floor plan of the cohort: Oracle in the front row alone, Executors in rows behind, Creativity in the wings. Rows are role, not tab order. Source: theatre box office charts.
3. **Crew manifest.** A ship's crew list: name, station, watch state. Monospace, aligned columns, a nautical dash of restraint. Click a name to jump to the station. Source: naval watch bills.
4. **Chat-first.** The sidebar is the message stream and seats are participants shown as avatars along the top. Terminal content is secondary; the conversation is the primary surface. Source: Discord member list inverted.
5. **Orbital.** Oracle in the centre as a large avatar, cohort seats as small avatars on a ring around it. Lines light up when messages flow. Below the ring, the flat tab list for actual navigation. Source: solar-system diagrams, Obsidian graph.
6. **Kanban strips.** Three vertical strips: active · waiting · idle. Seats slide between strips as state changes; errored seats pin to the top of whichever strip. Source: kanban boards.
7. **Character cards.** Each seat is a small card with the generated character's name and a procedurally tinted silhouette; the card flips to show last message on hover. Playful, matches the furry/fantasy name pools. Source: trading card games.
8. **Breadcrumb tree.** Space ▸ tab ▸ seat as an indented tree with thin vertical rails, like a file explorer but each leaf shows the state glyph on the rail itself. Source: VS Code explorer, `tree`.
9. **Timeline rail.** Seats ordered by last activity, newest at top, with a thin time axis on the left: a seat that has not spoken in 20 minutes physically drifts down. Source: Slack sidebar "recent", git log graph.
10. **Two-tone split sidebar.** Top half: spaces and tabs (navigation). Bottom half: live message stream filtered to the current space. A draggable divider. Source: Mail.app three-pane, Arc's bottom bar grown up.
11. **Nothing.** No sidebar at ten seats. Tabs in a top strip, a `Cmd+K` palette for everything. Sidebar appears only past a threshold. Source: Warp, Raycast.

ASCII of shape 1 (Arc verbatim), the baseline every other shape is measured against:

```
┌──────────────────────┐
│ ▣ bboxy450-term      │  space label, tinted bg
│                      │
│ ◆ Vesper Greywind  ● │  harness icon · name · state glyph
│   gpu research       │  title, muted
│ ◇ Thorn Ashvale    ◐ │
│   messaging research │
│ ◆ Lumen Foxglove   ○ │
│   ux divergence      │
│ ▢ zsh              ○ │
│                      │
│ + new seat    ⌘T     │
│                      │
│ ⚙ settings  ◷ history│  bottom pills, Arc style
└──────────────────────┘
```

## 2. Left sidebar at 1000 seats

Context: the vision names 1000 agents, local, ssh and cloud. A flat list is dead. The question is what a human needs to see at a glance and what they drill into.

1. **Dense heat strip.** Every seat is a 6×6 px cell in a grid, colored by state. 1000 seats = a 25×40 block that fits in the sidebar. Hover for name, click to focus. Spaces are column groups. Source: GitHub contribution graph, Datadog host maps.
2. **Collapsed cohorts.** Sidebar shows cohorts (one row per Oracle) with a summary glyph row: `●●●◐◐○○✕` as a tiny sparkline of member states. Expand one cohort at a time. Source: Finder column view, macOS Mail threads.
3. **Exceptions only.** Default view hides everything idle and active. Shows only waiting-on-input and errored seats. A counter at the bottom says `942 quiet`. The sidebar is an inbox, not a roster. Source: PagerDuty, air-traffic alert panels.
4. **Minimap + viewport.** The sidebar is a tall scrollable list but the top 80 px is a Sublime-style minimap of the whole list with a viewport rectangle. Drag the viewport to jump. Source: Sublime Text minimap.
5. **Fisheye.** Rows near the cursor are full height with name and title; rows further away compress to 2 px lines. The list is always the full 1000 but only the neighbourhood is legible. Source: macOS dock magnification, fisheye menus (Bederson).
6. **Fuzzy-first.** The sidebar is a search field with the list beneath it. Nothing renders until you type; recents show by default. Source: fzf, Alfred, Spotlight.
7. **Geographic.** Seats grouped by location: this machine · ssh hosts · cloud regions. Each location is a collapsible with a state sparkline. Answers "where is it running" before "what is it doing". Source: cloud console region pickers, mosh.
8. **Swarm radar.** Circular radar with Oracles as bright blips at radius by cohort, Executors as faint blips; states as blip colour. Sweep animation on message traffic. Below it the exceptions list from shape 3. Source: radar scopes, Starcraft minimap.
9. **Hierarchical zoom.** Space → cohort → tab → seat as nested rounded rectangles, like a treemap. Double-click zooms in; the breadcrumb at top zooms out. Area = seat count. Source: treemaps, Figma frames.
10. **Role lanes.** Three vertical lanes: Oracle · Executor · Creativity. 1000 seats mostly fall in the Executor lane as a dense strip; the Oracle lane stays short and readable. Source: swimlane diagrams.
11. **Paginated chapters.** Alphabetical by character name with letter dividers A..Z, a letter index strip on the right edge like iOS Contacts. Source: iOS Contacts.
12. **Virtualised Arc.** Arc verbatim but virtualised: only visible rows exist, spaces are collapsed by default, each collapsed space shows a count and the worst state. Honest answer: 1000 seats is 50 spaces of 20, and 50 collapsed rows is fine. Source: any virtual list.

ASCII of shape 2 (collapsed cohorts) merged with shape 3 (exceptions pinned):

```
┌──────────────────────┐
│ ⚠ needs you      3   │  exceptions always on top
│ ◆ Rook Ember     ◐   │
│ ◇ Sable Quill    ✕   │
│ ◇ Pip Hollow     ◐   │
│──────────────────────│
│ ▸ winnow   ●●●◐○○    │  cohort rows, state sparkline
│ ▸ ontology ●●●●●●●○  │
│ ▾ term     ●◐○✕○●    │  one expanded
│   ◆ Vesper Greywind ●│
│   ◇ Thorn Ashvale  ◐ │
│   ◆ Lumen Foxglove ○ │
│   ◇ Marrow Finch   ✕ │
│ ▸ cloud-eu 48 ·······│
│ ▸ ssh:rig  12 ·······│
│                      │
│ 1000 seats · 942 quiet│
└──────────────────────┘
```

## 3. Split minimap showing V and H

Context: Arc shows a split as a tile grid of equal rectangles. A binary split tree needs to show nesting and orientation at icon size (roughly 16×12 to 24×16 px).

1. **Literal tree.** Render the binary tree as nested rectangles exactly proportional to the real layout. A 60/40 vertical split inside the left half of a horizontal split draws as such. Source: Arc, i3 tree.
2. **Line-only.** Draw only the divider lines, no fill: `│` for V, `─` for H, nested. Reads as a glyph, not a picture. Source: box-drawing characters, `tmux` status.
3. **State-filled.** Shape 1 but each cell filled with that seat's state colour. Minimap doubles as the state summary. Source: heat maps.
4. **Focus-highlighted.** Shape 1, all cells faint, the focused seat solid. The minimap answers "where am I" first. Source: Vim window indicator plugins.
5. **Notation.** No picture. A tiny string: `V(H(a,b),c)` or `a|b/c`. Dense, greppable, ugly on purpose. Source: tmux layout strings, Lisp.
6. **Role-coloured cells.** Cells tinted by role rather than state. Oracle cell is always the brightest. Source: swimlanes.
7. **Count badge.** Just a number of seats and a single glyph for dominant orientation: `⫿4` or `⫼3`. Expands on hover to shape 1. Source: iOS folder badges.
8. **Animated on change.** Shape 1 but a split or close animates the rectangles subdividing or merging, 150 ms. The minimap narrates. Source: Figma auto-layout.
9. **Depth shading.** Shape 1 with each nesting level a shade darker, so deep trees read as depth without lines. Source: Material elevation.
10. **Mini-screens.** Each cell shows a 1-bit downsample of the actual seat's last 4 lines of output, like a thumbnail. Only plausible at larger sizes. Source: Mission Control.

ASCII for shapes 1 and 2 across the same tree `H( V(a,b), c )`:

```
shape 1 (filled)   shape 2 (lines)   shape 4 (focus on b)
┌───┬───┐          ┌───┬───┐          ┌───┬───┐
│ a │   │          │   │   │          │░░░│   │
├───┤ c │          ├───┤   │          ├───┤░░░│
│ b │   │          │   │   │          │███│   │
└───┴───┘          └───┴───┘          └───┴───┘
```

## 4. Seat state glyphs: active / idle / waiting-on-input / errored

Context: icons, not text badges, per vision. Must read at 10 px, must survive colour-blindness, must not flicker at 1000 rows.

1. **Dot set.** `●` active (filled), `○` idle (hollow), `◐` waiting (half), `✕` errored. One glyph family, four fills. Source: Unicode geometric shapes.
2. **Traffic light with shape.** Green circle, grey circle, amber triangle, red square. Shape carries the state so colour is redundant. Source: colour-blind-safe signage.
3. **Breathing.** Active glyph pulses slowly (2 s period, 20 percent opacity swing). Idle is static. Waiting pulses fast. Errored is static red. Motion = attention cost, budgeted by severity. Source: Apple Watch breathe, ship running lights.
4. **Cursor metaphor.** Active = blinking block cursor `▮`, idle = underscore `_`, waiting = a bracket pair `[ ]` suggesting a prompt, errored = `!`. Terminal-native. Source: the cursor itself.
5. **Ear glyphs.** Tied to the fox logo: ears up = active, ears relaxed = idle, one ear tilted = waiting, ears back = errored. Four tiny silhouettes. Source: canine body language.
6. **Progress ring.** Active is a spinning ring, idle is an empty ring, waiting is a ring with a notch, errored is a ring with a bar. Source: macOS, Material progress.
7. **Weather.** Sun, cloud, hourglass, lightning. Whimsical, loud, probably wrong, logged anyway. Source: weather apps.
8. **Spark glyph.** A tiny 8-sample sparkline of PTY output cadence replaces the glyph. Active is a busy line, idle is flat, waiting is flat with a cursor tick, errored turns red. Shows *how* active. Source: Datadog, htop.
9. **Name tinting.** No glyph. The character name itself is bright for active, dimmed for idle, underlined for waiting, struck for errored. Saves 14 px per row. Source: IRC nick lists.
10. **Lamp on the rail.** The tree rail (section 1 shape 8) carries a small lamp at each leaf. State is the lamp. The row text stays clean. Source: patch bays, mixing consoles.

Unicode candidate table, for a renderer that has the glyphs anyway:

| state | A | B | C | D |
|---|---|---|---|---|
| active | ● | ▮ | ◉ | ⬢ |
| idle | ○ | _ | ◎ | ⬡ |
| waiting | ◐ | [ ] | ◒ | ⬣ |
| errored | ✕ | ! | ⊗ | ⨯ |

## 5. Role and priority surfacing in a message stream

Context: priority derives from sender role; Oracle outranks. Nothing dropped. The UI must make "who outranks" obvious without a legend.

1. **Rank by position.** Oracle messages pin to the top of the stream until read; Executor and Creativity flow chronologically beneath. Source: pinned posts, Slack.
2. **Rank by weight.** All chronological; Oracle messages set in a heavier weight and a full-width tint band; Executor normal; Creativity italic. Source: newspaper typography hierarchy.
3. **Rank by colour rail.** A 3 px left rail per message: Oracle gold, Executor blue, Creativity violet. Scannable from across the room. Source: Git blame gutters, Linear priority bars.
4. **Chess pieces.** Role icons: ♔ Oracle, ♟ Executor, ♝ Creativity. Priority is implicit in the piece. Source: chess.
5. **Three columns.** The stream is three lanes side by side, one per role; time runs down. Cross-role replies draw a thin connector. Source: swimlane sequence diagrams.
6. **Signed stamp.** Every message shows a tiny signature seal: the sender's key fingerprint as an identicon plus the role glyph. Unsigned or unknown-role messages get a dashed outline. Source: wax seals, GPG identicons.
7. **Priority as size.** Oracle messages render larger, Creativity smaller. Crude, honest, probably too much. Source: tag clouds.
8. **Reply threading by role.** Oracle messages are roots; Executor reports nest under the assignment they answer; Creativity nests under the brief. The stream becomes a task tree. Source: Reddit threads, Zulip topics.
9. **Audio cue.** Oracle messages chime, Executor messages tick, Creativity messages chord. Off by default. Source: ICQ "uh-oh".
10. **Envelope visible.** Show the actual envelope fields as a muted one-liner above each message: `oracle · p0 · signed 844ECC05 · #bboxy450-term`. Teaches the protocol by looking at it. Source: email headers, Wireshark.
11. **Badge stack on the seat row.** Not in the stream at all: unread counts live on the seat row in the sidebar, coloured by the highest-priority unread. The stream is plain. Source: iOS badges.
12. **Name-signed only.** The agents already sign `Name <ROLE>` in text. Parse it, colour the role token, nothing else. Zero chrome. Source: this channel.

ASCII of shape 3 combined with shape 2:

```
▌ Vesper Greywind <ORACLE>                   12:03
▌ Lumen Foxglove: go on C. Read vision first.
▌
┃ Thorn Ashvale <EXECUTOR>                   12:07
┃ A claimed. Cloning alacritty, wezterm, rio.
┃
╎ Lumen Foxglove <CREATIVITY>                12:09
╎ Field written. Eleven shapes for the sidebar.
```
(gold heavy rail, blue rail, violet dashed rail)

## 6. Cmd+T harness picker

Context: Cmd+T opens a new tab and asks which harness; zsh last. A separate shortcut opens zsh directly. The picker is hit hundreds of times a day; it must be fast and look like it belongs.

1. **Vertical list, keyboard-first.** A small centred popover: Claude · Codex · pi · omp · zsh. Arrow keys or the first letter. Enter. Done. Source: Alfred.
2. **Horizontal icon row.** Five icons in a row under the cursor, like macOS Cmd+Tab. Tab cycles, release selects. Source: Cmd+Tab.
3. **Inline in the sidebar.** Cmd+T inserts a new ghost row in the sidebar reading `new seat ▸ claude codex pi omp zsh`, you pick in place and the row becomes real. No modal. Source: Arc's new tab row.
4. **Role-first.** Ask the role first (Oracle / Executor / Creativity), then the harness defaults from settings; harness can be overridden on the second step. Matches the vision's "default roles live in settings". Source: character creation screens.
5. **Last-used sticky.** Enter with no selection repeats the last harness. The list reorders by recency. Source: Spotlight.
6. **Radial.** A pie menu around the cursor, five slices, zsh at the bottom. Flick to pick. Source: Blender, game radial menus.
7. **Split-aware.** The picker also asks "here, as split, or new tab" with three small layout glyphs from section 3. One dialog, two decisions. Source: tmux prefix keys.
8. **Templates.** Beyond harnesses: cohort templates like "Oracle + 2 Executors" that spawn a split tree in one go. Harness list on the left, templates on the right. Source: Xcode new-project, VS Code profiles.
9. **Name reveal.** The picker shows the character name the new seat will get, rolled live, with a reroll key. Makes seat creation feel like summoning. Source: tabletop RPG character rolling.
10. **Typed prefix.** Cmd+T then type `cl`, `co`, `pi`, `om`, `z`. The popover is a single input with live matches. Source: fzf.

ASCII of shape 1 with shape 9's name reveal:

```
      ┌────────────────────────────┐
      │ new seat                   │
      │ ▸ ◆ Claude     oracle      │
      │   ◇ Codex      executor    │
      │   ◈ pi                     │
      │   ◈ omp                    │
      │   ▢ zsh                    │
      │────────────────────────────│
      │ name: Bramble Vex   ↻ reroll│
      └────────────────────────────┘
```

## 7. Settings menu

Context: "must look good". Settings include default roles per harness, role system prompts, keybindings, theme, messaging identity and key, permissions.

1. **macOS System Settings clone.** Left list of sections, right pane of grouped controls. Boring and correct. Source: macOS Settings.
2. **Single long page.** One scrolling page with sticky section headers and a search box at top. No navigation tree. Source: Arc settings, Linear.
3. **TOML with a GUI overlay.** Settings are a `config.toml` shown in a seat with syntax highlighting; a side panel renders form controls bound to the same file. Edit either. Source: Zed, Helix.
4. **Cards per harness.** Each harness is a card: icon, default role, system prompt editor, wake method, state-detection hooks. Add a harness = add a card. Source: Stripe dashboard.
5. **Role editor as characters.** The three roles are three character sheets: name, prompt, permissions, colour. Matches the name-pool whimsy. Source: RPG character sheets.
6. **Command palette only.** No settings window. Cmd+, opens a palette; every setting is a searchable command with inline value editing. Source: VS Code settings search, Raycast.
7. **Key and identity as a passport page.** The messaging identity gets its own page styled like a passport: avatar, fingerprint, devices, channels joined. Source: passports, Keybase.
8. **Diff-aware.** Settings page shows what differs from defaults in a tinted column and offers "reset this". Source: Firefox about:config "modified".
9. **Preview pane.** Theme and glyph choices render a live mini sidebar on the right as you change them. Source: Terminal.app profiles, Figma.
10. **Tabs in the same tab strip.** Settings open as a tab in the current space, not a window; the sidebar shows it as a seat with a gear icon. Everything is a tab. Source: Chrome settings, Zed.

## 8. Toolbar icon

Context: the icon in the window toolbar and likely the menubar. Small, monochrome-capable, has to coexist with macOS template icons.

1. **Fox ear pair.** Two triangles, slightly asymmetric, nothing else. Reads at 12 px. Source: the identity image's ears.
2. **Prompt chevron with ear.** `›_` where the chevron has a tiny ear notch. Terminal-first. Source: Terminal.app icon.
3. **Sidebar toggle fox.** The standard sidebar-toggle glyph (rect with left column) but the left column has an ear tip. Doubles as the actual sidebar toggle. Source: macOS sidebar icon.
4. **Cohort dots.** Three dots in a triangle, one bright. Represents Oracle + cohort. Source: Figma's logo logic.
5. **Block cursor fox.** A filled block cursor `▮` with two ears on top. Source: cursor.
6. **Hoodie.** A hood outline, no face. Mysterious, maybe too mysterious. Source: the identity image's hoodie.
7. **Split glyph.** The section 3 minimap at 16 px, a square divided V then H. Pure function. Source: i3.
8. **Single eye.** One of the blue eyes from the image reduced to circle-in-circle with a highlight. Oracle = watching. Source: the identity image.
9. **Signal arcs.** A dot with two arcs, like Wi-Fi, but the dot has ears. Messaging-first. Source: Wi-Fi glyph.
10. **Letterform.** A lowercase `b` whose ascender is an ear. Source: monogram logos.

## 9. Logo derived from bboxy.png

Context: electric-blue fox, black hoodie, tilted head, half-lidded confident eyes, one visible fang, white cheek and chest fur, dark navy hair with blue streaks, periwinkle background. Directions only, no rendering.

What carries identity when everything else is stripped, in order: the ear shape and inner-ear contrast, the three-tone blue (electric, navy, periwinkle), the half-lidded eye with the thick dark lid line, the single fang, the hood.

1. **Head silhouette.** Flat two-tone fox head, electric blue fill, navy ear interiors and hair tuft, no face. The hood as a navy collar at the bottom edge. Scales from 1024 to 16.
2. **Ears only.** Two ears with the white inner-fur stripe. The most reducible element. Works as favicon, toolbar, cursor.
3. **Eye crop.** Square crop of one eye and the lid line, flattened to four colours. Oracle = the watcher. Pairs with the toolbar shape 8.
4. **Hoodie monogram.** The hood outline with `b` inside, drawstrings as the two legs of the letter. Streetwear logic.
5. **Negative-space fang.** A speech-bubble or terminal-block shape with a single fang-shaped notch cut from the bottom edge. Terminal + fox in one mark.
6. **Prompt fox.** `>` rotated to become a muzzle, with two ear triangles above. A glyph that could literally be typed.
7. **Blue-streak mark.** Abstract: three strokes in navy, electric blue and white, the hair's streak pattern, no animal. Deniable, elegant, less fun.
8. **Pixel fox.** 16×16 pixel-art fox head using exactly the three blues plus white. Honours the terminal, embraces the furry pool.
9. **Tilted head.** Keep the head tilt from the image as the signature: every mark is rotated 12 degrees counter-clockwise. The tilt becomes the brand gesture even in text logos.
10. **Mask.** The face as a vulpine mask, eyeholes empty, hood up. The hidden operator. Dark and good on a black titlebar.

Palette lifted by eye from the image, for whoever renders:

| name | approx | from |
|---|---|---|
| electric | #2E6BFF | cheek and ear fur |
| navy | #141A3C | hair, lid line, nose |
| periwinkle | #7B8CFF | background |
| cream | #F2EEE6 | cheek and chest |
| hoodie | #0B0B10 | hood |

---

## Cross-domain sources used

Theatre seating charts, naval watch bills, chess, treemaps, radar scopes, Starcraft minimap, Sublime minimap, fisheye menus, iOS Contacts, patch bays and mixing consoles, canine ear language, wax seals, passports, tabletop character rolling, streetwear monograms, pixel art, newspaper typography hierarchy, air-traffic alert panels.

## Censor log

Objections that fired while generating, recorded and not obeyed:

- "A radar or orbital view is a toy." Logged. It may be a toy. It is in the field so the pick is made by comparison, not by reflex.
- "Nobody will run 1000 seats." The vision says design for it from the start. Logged, overruled by the brief.
- "Weather glyphs are embarrassing." Yes. Logged anyway; shape 7 in section 4 exists so shape 1 can be seen to be better.
- "Just copy Arc." Arc is shape 1 in sections 1 and 3. It is the baseline, not the answer.
- "Ear glyphs will not read at 10 px." Probably true. The pick can test it; the field should not pre-decide.
- "Too many sections, too long." The brief asked for a field. Volume is the method. Compression is the Oracle's job.

## What I did not check

- No glyph was rendered at target size; all legibility claims are untested.
- Colour values are eyeballed from the image, not sampled.
- No candidate was tested against colour-blindness simulators.
- Arc, iTerm2 and the other sources are recalled from use and from the three screenshots, not re-opened.

---

# Part two: diverging inside the constraints

Added after the Oracle's three addenda. Source: [[ux-sidebar]] constraints 0..8 and `runs/ref-iterm-full-window-bad.png`. Sections 1 and 2 above stay as the unconstrained field; the shapes below obey the constraints and are the ones that can actually be picked.

What the bad reference shows, measured by eye: the top tab strip carries grouping and shortcuts well (coloured pills, ⌘N). The right panel is a second list of the same seats in a different order, with text-only rows and a two-line preview, so the operator has to read to map a row to a pane. Two lists, one truth, no shared identity. The fix is one list, spatial, identity-carrying.

## 10. The one left element

Each shape obeys: one element, stable order, row identity = seat identity, space → tab → mini-grid, icons over text, ⌘N visible, collapses to 1000.

1. **Pill stack.** iTerm's coloured group pill rotated vertical. A space is a tall rounded pill; its tabs sit inside as rows; each tab row ends in its mini-grid and ⌘N. The pill colour is the space colour; the row colour is the seat colour. Source: the good iTerm reference, turned 90 degrees.
2. **Rail and leaves.** Space is a header, tabs hang from a thin vertical rail, each leaf is a seat row with the mini-grid of its tab beside the first leaf only. The rail carries the state lamps so the row text stays clean. Source: patch bays.
3. **Mini-grid as the row.** The tab row *is* the mini-grid, enlarged to the sidebar width, each cell containing the seat's icon and name. A split tab is literally a small picture of itself. Below the collapse threshold this is the most honest map there is. Source: Mission Control, Arc tiles.
4. **Card per tab.** Each tab is a card: title strip with ⌘N, body is the mini-grid with seat initials. Cards stack inside a space. Dense enough at 10, collapses to a one-line card at 1000. Source: Trello.
5. **Colour-first.** Every seat row is a solid colour block with the name in white, the harness icon as a knockout. The in-pane title strip uses the identical block. Mapping is instant by colour alone. Source: Google Calendar event chips.
6. **Monogram chips.** Row = coloured circle with the character's initials, then name, then state glyph at the far right. The pane title strip shows the same circle. Source: Slack avatars, Linear assignee chips.
7. **Breadcrumb row.** Each row reads `space ▸ tab ▸ seat` with the mini-grid at the end. Flat list, hierarchy in the text, collapses by prefix. Source: Finder path bar.
8. **Two-density sidebar.** One element with a density toggle: comfortable (rows with title) and compact (icon + glyph only, 18 px rows). At 1000 the compact mode holds about 50 rows per screen plus collapsed spaces. Source: Gmail density.

ASCII of shape 1 at 10 seats, with constraint 3's title strip shown on the right so the mapping is visible:

```
sidebar                               seat title strip (inside pane)
╭──────────────────────────╮         ┌─────────────────────────────────
│ ▎term                 ⌘1 │         │ ◆ Vesper Greywind  ·  gpu research
│  ◆ Vesper Greywind    ●  │ ──────▶ │ (strip is the seat's colour,
│  ◇ Thorn Ashvale      ◐  │         │  same icon, same name)
│  ◆ Lumen Foxglove     ○  │
│  ┌─┬─┐                   │         mini-grid of this tab: V(a, H(b,c))
│  │ │ │  ⌘2               │
│  │ ├─┤                   │
│  └─┴─┘                   │
╰──────────────────────────╯
╭──────────────────────────╮
│ ▎winnow               ⌘3 │
│  ◇ Marrow Finch       ✕  │
│  ▢ zsh                ○  │
╰──────────────────────────╯
```

ASCII of the same shape at 1000 seats, constraint 8 (collapse with rolled-up state):

```
╭──────────────────────────╮
│ ▸ term        3  ●●○     │  space row: seat count, worst-first glyphs
│ ▸ winnow      2  ✕○      │
│ ▸ cloud-eu   48  ◐◐◐●…   │  up to 6 glyphs, then an ellipsis
│ ▾ rig        12          │  expanded space
│   ▸ ingest    4  ●●●○ ⌘4 │  tab rows: count, glyphs, shortcut
│   ▾ parse     3       ⌘5 │  expanded tab shows seat rows
│     ◇ Ash Vale        ●  │
│     ◇ Pip Hollow      ◐  │
│     ◇ Sable Quill     ○  │
│ ▸ ssh:alpha  200 ●●●●●●… │
│ …                        │
│ 1000 seats · 7 waiting   │
╰──────────────────────────╯
```

Rule shared by every shape above: a seat's row never moves. State changes the glyph and may add a glow (section 12); it never re-sorts. "Waiting" aggregates into the parent's count so the operator can find it without the list moving under them.

## 11. Seat colour and the title strip (the two open gaps)

Colour assignment, three shapes, each with what it breaks:

1. **Hash of name.** Deterministic, survives restarts, same colour on every machine. Breaks: two neighbours can collide; no control.
2. **Per-space palette, assigned in creation order.** Eight to twelve hues per space, cycling. Neighbours never collide inside a space. Breaks: the same name in two spaces has two colours.
3. **Hash of key.** Seat colour derives from the seat's own key, so colour is identity in the cryptographic sense too. Operator can override per seat. Breaks: nothing visible; the override is the escape hatch. Source: GPG identicons, Keybase.

Title strip, four shapes:

1. **Full-width bar.** A 22 px strip across the top of the seat in the seat colour: icon, name, title, state glyph at right, ⌘N at far right. Mirrors the sidebar row exactly. Source: iTerm pane titles, but coloured.
2. **Left edge rail.** A 3 px vertical rail in the seat colour down the pane's left edge, plus a name tag in the top-left corner. Minimal chrome, strong identity. Source: Git gutters.
3. **Corner tag.** Only a small rounded tag in the top-right corner: icon + name in the seat colour. Scrollback keeps every pixel. Source: Figma frame labels.
4. **Tinted border.** The pane border itself is the seat colour, 1 px idle, 2 px glowing when active. No text at all; the sidebar row is where the name lives. Source: focus rings.

## 12. One look everywhere: phone to 1000-seat desktop

Constraint 0 says no Aqua, no GTK, no Material. Our chrome is drawn, not borrowed, so the same drawing must fit three shapes of screen.

1. **The sidebar is a sheet.** Desktop: a pinned left panel. Narrow desktop: it overlays. Phone: it is a bottom sheet pulled up by a drag handle, the same rows, the same mini-grids. One component, three positions. Source: Maps apps.
2. **Seat as a card deck on phone.** The phone shows one seat full-screen; swipe horizontally between seats of a tab, vertically between tabs. The mini-grid becomes a page indicator. Source: Safari tab switcher.
3. **Mini-grid as the phone's home.** On phone the home screen *is* the mini-grid of the current tab at full size; tap a cell to enter the seat. Source: Mission Control.
4. **Density tokens, not breakpoints.** One layout with three density tokens (touch, comfortable, compact) that scale row height, glyph size and hit targets. Phone gets touch, 1000 seats gets compact. Source: Material density, but ours.
5. **Pointer-agnostic rows.** Every row action (focus, close, split, assign role) is a long-press or right-click menu, never a hover-only affordance. Hover adds glow, nothing else. Source: iPadOS pointer.
6. **Title bar is ours.** No native traffic lights: our own three glyphs in the seat palette, drawn in the same strip as the space label. Desktop OSes allow this with a borderless window. Phone has no title bar; the strip becomes the status area.
7. **The whole app is a canvas.** One wgpu surface, chrome and terminals in it, no platform widgets anywhere. Phone and desktop differ only in input and in which layout tokens are active. This is the mechanism that makes constraint 0 possible, and it is what Task A's toolkit question is really asking.

## 13. GPU chrome: where shaders earn a place

Shaders are a first-class tool. A field of uses, each with the state it serves, so nothing glows for decoration.

1. **Active glow.** The seat colour bleeds 6 px outside the row and the pane border with a soft blur while the seat is active. Idle has no glow. The glow is the state. Source: neon signs.
2. **Waiting pulse.** Waiting-on-input rows and borders pulse the glow at 1 Hz. The only animation that repeats. Source: pagers.
3. **Error fracture.** Errored seats get a brief chromatic-aberration shiver on the border, 300 ms, once, then a static red rail. Loud once, quiet after. Source: glitch art.
4. **Scrollback depth fog.** The top of a seat's visible area fades into a dark gradient when there is scrollback above, so "more above" is felt, not scrollbar'd. Source: fog of war.
5. **Message ripple.** When a message arrives for a seat, a ring expands from its sidebar row and from its pane corner, 400 ms, in the sender's role colour. Connects the stream to the map. Source: water, Material ripple but spatial.
6. **Split reveal.** Creating a split animates the mini-grid and the pane in sync, the new cell growing from the divider. Source: Figma auto-layout.
7. **Focus vignette.** Unfocused panes darken 10 percent with a subtle vignette; the focused one is full brightness. Source: stage lighting.
8. **Frosted sidebar.** The sidebar background blurs the terminal content behind it when it overlays on narrow screens. The one place blur is structural. Source: macOS vibrancy, but ours.
9. **Heat trail in the mini-grid.** Cells in the mini-grid brighten with recent output cadence and decay over 10 s. The map shows where the work is. Source: thermal cameras.
10. **Identity shimmer.** The seat colour in the title strip carries a slow, barely visible animated noise so two seats of similar hue still feel distinct. Source: holographic foils.

Budget rule for the pick: at 1000 seats, only glyphs and static colour render by default; glow, pulse and ripple apply to on-screen rows only, and the heat trail is per expanded tab. Shaders cost per pixel, not per seat.

## 14. The component lab

The Oracle asked for a storybook-style lab to prototype every UI element with all its states before it is built in Rust.

### Medium: three options, costed

| option | what it is | fast to iterate | true to the renderer | sync cost |
|---|---|---|---|---|
| **a. Web lab** | Static HTML + CSS + a little JS per component, one page per component, a state switcher. Served locally, no framework. | highest | none: CSS glow is not a wgpu glow; fonts and glyph metrics differ | tokens must be mirrored by hand or generated |
| **b. Rust lab** | A `lab` binary in the workspace on the real wgpu renderer. Each component is a Rust module with a `states()` list; the binary shows a gallery with a state switcher and hot-reloads the tokens file. | medium: compile times; hot reload of tokens and shaders only | total | zero: it is the build |
| **c. Both, staged** | Web lab for shape and layout divergence in the first weeks; Rust lab becomes the lab of record once the renderer exists. Web pages are archived, not maintained. | high early, true later | true where it matters | one-way handoff, deliberate |

Shapes for the Rust lab's UI itself: a left list of components, a state strip across the top (one chip per state), the component centred on a checkerboard, a token panel on the right, a density toggle (touch / comfortable / compact), a screenshot key that writes `runs/lab/<component>-<state>.png`.

### Component list, each with its states

| component | states |
|---|---|
| state glyph | active, idle, waiting, errored; each at 10, 14, 18 px |
| harness icon | claude, codex, pi, omp, zsh; template and colour variants |
| role mark | oracle, executor, creativity; unsigned |
| seat row | each state × focused/unfocused × with/without subtitle × 3 densities |
| tab row | expanded, collapsed, with ⌘N, with 1..8 seats |
| mini-grid | single, V, H, V(H), H(V), depth 3, focused cell, heat trail |
| space header | expanded, collapsed with rolled-up glyphs, 1..999 seat count |
| sidebar | 10 seats, 100 seats, 1000 seats, phone sheet |
| seat title strip | each colour-assignment shape × each state |
| pane border | idle, active glow, waiting pulse, error fracture |
| message row | each role × read/unread × reply-nested |
| unread badge | 1, 12, 99+, priority colour |
| harness picker | open, filtered, name reroll |
| settings page | each section; diff-from-default marker |
| toolbar icon | each shape in section 8, light and dark, 16 and 32 px |
| logo | each direction in section 9, 16 to 1024 px |
| colour tokens | the full palette, light and dark, contrast grid |

### Staying in sync with the Rust build

1. **One tokens file.** `design/tokens.toml`: colours, radii, row heights per density, glyph sizes, glow radii, animation durations. The Rust build includes it with `include_str!` at compile time; the web lab reads the same file through a 20-line generator that emits CSS variables. Tokens drift is impossible because there is one file.
2. **States are data.** Every component's state list lives in one Rust enum in a `ui-model` crate with no rendering dependency. The lab iterates the enum; the app matches on it. A state added to the enum appears in the lab on next build.
3. **Screenshot diffs.** The Rust lab writes `runs/lab/*.png` per component and state. A CI job diffs against the previous commit's set and posts changed images. Visual regressions are seen, not described.
4. **The lab is the build.** Option b and c make the lab a binary target in the same workspace, so a renderer change cannot leave the lab behind. The web lab, if used, is explicitly disposable.
5. **Divergence stays in `.kb`.** Shapes live here as text; the lab holds only what the Oracle selected. The field and the pick never mix in one place.

Gap: whether `glyphon` or `sugarloaf` can render the glyph table in section 4 at 10 px with hinting good enough to judge; Task A answers this and the lab's first component depends on it.

## Censor log, part two

- "Just build it in Rust, skip the web lab." Logged. Option c exists because the renderer does not yet, and shapes are cheaper in CSS for a week.
- "Shaders are decoration." Every shader in section 13 is tied to a state or a structure; the budget rule is there so the pick can cut.
- "Phone is years away." Constraint 0 says design for it now. Logged, overruled by the brief; the sheet shape costs nothing to keep in mind.

## Links

[[vision]] · [[ux-sidebar]] · tasks: `tasks/2026-10-21-gpu-terminal-research.md` (toolkit question for the lab's renderer)

## Next

Oracle selects one shape per section, or a merge, and names it. Then the UI prototype under [[vision]] process step 2, in the lab from section 14.
