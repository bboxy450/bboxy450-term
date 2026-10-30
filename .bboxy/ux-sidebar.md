---
id: "2QA4Z32N5QSEMMQ1CAX1HFNK3W"
type: "note"
number: 65
title: "UX: the left sidebar — operator constraints"
created: "2026-10-21T18:53:32Z"
updated: "2026-10-30T04:18:32Z"
---
# UX: the left sidebar — operator constraints

Owner: Vesper Greywind <ORACLE>. Source: operator critique of iTerm2 on 2026-10-21. Back to [[vision]]. Divergence field: [[ux-divergence]].

## Reference screenshots

| File | What it shows | Verdict |
|---|---|---|
| `runs/ref-iterm-grouped-tabs.png` | iTerm2 grouped tabs across the top: colored group pill, tabs inside, state text (Idle / working) and ⌘N shortcut per tab | **Good.** This is the grouping model. Move it to the left. |
| `runs/ref-arc-sidebar.png` | Arc browser sidebar: split tiles as a mini-grid, space label, New Tab, Settings/History at the bottom | **Good.** Riff on the split mini-grid; add vertical and horizontal split shapes. |
| `runs/ref-iterm-session-status.png` | iTerm2 harness plugin "Session Status" panel on the right: list of sessions with state and last message | **Not bad, wrong side, too busy.** We want it on the left, simpler, more elegant. |
| `runs/ref-iterm-full-window-bad.png` | Full iTerm2 window: top grouped tabs, seats with splits, Session Status on the right | **Seats and splits good. Top bar ok. Right panel bad.** It reorders, and the rows are text that does not map to what the operator sees inside a seat. |

## Constraints (DECIDED)

0. **Own look, every platform.** macOS first, Linux and mobile follow. The look and feel is unified across all of them and transcends OS styles. No native-toolkit chrome, no Aqua, no GTK, no Material. Our own creation. Design for touch and small screens as well as a 1000-seat desktop.

1. **One left-side element** combines what iTerm puts in the top tab bar and the right Session Status panel. Nothing on the right.
2. **Stable order.** Rows never reorder on state change. A seat is where you left it. State is shown in place (glyph, color), never by moving the row.
3. **Visual identity maps to the seat.** What a row shows must be recognizable inside the seat: the seat's generated character name, its harness icon, its color. The same name and color appear in the seat's title strip. An operator glances at a row and knows which pane it is without reading.
4. **Hierarchy is spatial.** Space → tabs → split tree, drawn as a mini-grid like Arc's split tiles, extended with vertical and horizontal split shapes. The mini-grid is the map of the tab.
5. **Icons over text.** Harness icon, state glyph, role mark. Text is the seat name and a short title, nothing else by default. Last-output subtitle is opt-in.
6. **Shortcuts visible.** ⌘N per tab like iTerm's pill.
7. **Simpler than the iTerm plugin.** If a row needs two lines of explanation it is wrong.
8a. **Every pixel is precious** (operator, 2026-10-21). Minimise space between panes: gutters of a few pixels at most, no padding inside the window edge, the glow lives in the gutter and over the pane edge rather than taking layout room, title strips stay 22 px, the sidebar has a compact density and collapses fully. Terminal cells are the content; chrome earns its pixels or goes.
8. Must still read at 1000 seats: collapse by space, then by tab, with state rolled up (a space shows how many seats are waiting).

## Open

- **Gap:** color assignment per seat: hash of name, per-space palette, or operator-picked.
- **Gap:** what the seat's in-pane title strip looks like so the mapping to the sidebar row is instant.

## Utility accordions (operator, 2026-10-29)

Order: swarms, + new swarm, SSH Hosts, Cloud, Tasks, Chat last. Tasks and Chat share the swarm card chrome and independent saved folding. Closing them leaves the focused stage, running harnesses and in-memory draft intact. Existing per-section scrolling/resizing remains available when expanded.

Utility identities use fixed jewel tones from `color.section`: Hosts teal (machines), Cloud azure (remote destinations), Tasks violet (work), Chat rose (conversation). Native and web cards share the same token values in light and dark themes. Utility colors do not inherit the selected swarm's identity or encode its activity; status marks retain their own meaning.
