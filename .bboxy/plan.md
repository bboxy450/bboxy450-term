---
id: "0BQSDW8NSVSWEAN2X17DCEZSY4"
type: "note"
number: 93
title: "Implementation plan — v0.1"
created: "2026-10-21T20:08:43Z"
updated: "2026-10-24T00:10:38Z"
---
# Implementation plan — v0.1

Owner: Vesper Greywind <ORACLE>. Date: 2026-10-21. Status: DRAFT until the operator answers the open decisions at the bottom. Back to [[vision]]. Specs: [[terminal-core]] [[mux-daemon]] [[harness-adapters]] [[subagent-detection]] [[messaging]] [[mcp-control-plane]] [[ux-prototype]] [[identity]] (pending). Research index: `research/`.

## Priority order (operator, 2026-10-21)

**Cockpit first (operator, 2026-10-22, [[vision]] Cockpit first):** a working local terminal with harnesses, messaging, tasks, provenance and events, in the sexy UI; ssh and cloud are dummies in the UI until the cockpit works. Epic [[2026-10-22-c-cockpit]]. Earlier order, still true within that: rendering first. Then local messaging, MCP, harness visibility. Cloud designed in from the start, built after rendering. Private cross-instance messaging after cloud. Global messaging last. W2.4 is local-only in v0.1; W4 moves behind cloud.

## Shape

One Cargo workspace, weavie-managed once Nix is installed ([[monorepo-weavie]]). One binary, two modes. Six crates:

| Crate | Owns | Spec |
|---|---|---|
| `term-core` | forked alacritty_terminal + vte parser, viewport grid, checkpoint export/import, OSC 7501/7502 pass-through | [[terminal-core]] |
| `mux` | supervisor + logic processes, seats, PTYs (portable-pty), byte ring, harness adapters, messaging, MCP server, state streams | [[mux-daemon]] [[harness-adapters]] [[messaging]] [[mcp-control-plane]] |
| `ui-model` | seat/cohort/tab/split state enums and layout tree, no rendering deps; shared with the lab | [[ux-prototype]] |
| `renderer` | wgpu device, cell instancing, text pipeline per ghostty, chrome primitives, glow/ripple shaders | [[text-rendering]], renderer architecture (Oracle, next) |
| `bbt-ui` | the wgpu component library: Box, Text, Button, Table, List, Input, materials as shaders, Style from tokens | [[gui-components]] §2b |
| `bbt-doc` | native Markdown viewer and our own Mermaid engine; semantic-history path detection | [[document-viewer]] |
| `app` | winit window, borderless title bar, GUI client of the mux, settings | [[ux-prototype]] at tag `ux-prototype-v2` (operator: the implementation starting point), `design/bboxy-design-system.json`, `design/style.json`, `design/tokens.toml`, `lab/web` |
| `lab` | Rust component gallery on the real renderer, tokens from `design/tokens.toml` | [[ux-prototype]] §7 |

## Workstreams and first assignments

Each line is one Executor assignment with its own brief. Order within a stream is sequential; streams run in parallel.

**W1 term-core (TTY/VTE).**
1. Vendor alacritty HEAD `term/`, `grid/`, `index`, `cell`, `vte` parser into `crates/term-core`; cut tty, event_loop, vi_mode, search, selection; build green with the alacritty test suite that still applies. Verify: `cargo test -p term-core`.
2. Viewport-only mode (`history_size = 0`) as the daemon default; expose parser-state checkpoint export/import; unknown-OSC pass-through hook for 7501/7502. Verify: round-trip test, SGR survives a checkpoint restore (Nyx's red-text probe as the fixture).

**W2 mux (backend).**
1. Supervisor: spawn via portable-pty, hold masters, single poller over raw fds, byte ring with checksummed records and committed boundary, lease protocol over a unix socket. Verify: SIGKILL the logic process, seat output continues; SIGKILL the supervisor test child, document SIGHUP.
2. Logic: seat registry with durable seat id ↔ harness session mapping, term-core per seat, state-stream protocol (Snapshot/Delta/Applied) over unix IPC. Verify: a headless client attaches, kills, re-attaches, screen identical.
3. Adapters: Claude and Codex first (7501 probe answer, sessions file, app-server events, hooks shim with BBOXY_SEAT_ID), zsh with ZDOTDIR overlay; pi/omp after. Verify: live capture fixtures from task H.
4. Messaging on quinn: local-only first (same machine, unix + loopback QUIC), envelope with signed role, receiver priority queue, mute/unmute notices. Verify: two seats exchange, Oracle message dequeues first.
5. MCP server with the catalogue, stdio bridge per seat. Verify: Claude seat lists seats and reads a screen through it.

**W3 renderer + app (frontend).**
1. Renderer skeleton: wgpu on Metal, one Device shared, cell instancing with the lifted layout, grayscale atlas via CoreText-backed rasterization, WGSL. Verify: renders a term-core grid at 120 Hz vsync in a winit window.
2. Chrome primitives: rounded rects, hairlines, text runs, the cohort block, pane strips; tokens from `design/tokens.toml` and the visual style from `design/style.json` (Raycast material: frosted panels, hairlines, radius and type scales, one motion curve) via `include_str!`. Verify: `lab` shows the sidebar at 10 and 15 seats matching the web lab screenshots.
3. Glow and locate shaders: border particle glow with the four readings, directional locate ripple, dim for unfocused. Verify: side by side with the web showcase.
4a. Document tab: ⌘-click a `.md` path or wikilink opens it rendered natively; Mermaid flowcharts first ([[document-viewer]]).
4. App: borderless window, sidebar/stage layout from `ui-model`, drag-resize gutters, ⌘T picker, close dot with grace strip and ⌘Z, GUI client of the state stream. Verify: full loop, Claude seat live in a pane with state and subagent rows.

**W4 identity + remote.** After [[identity]] lands: root key custody, device enrollment, Google binding; then ssh remote seats (supervisor bootstrap over ssh, direct QUIC to public VMs). Punch and relay in v0.2.

## Dependencies

W2.2 needs W1.1. W3.1 needs W1.1. W3.4 needs W2.2. W2.3 needs task H captures (Rook). W4 needs Nyx's identity spec. Everything else is parallel.

## Defaults applied 2026-10-21 (operator said start building; overridable)

- Crate names `bbt-core`, `bbt-mux`, `bbt-render`, `bbt-ui-model`, `bboxybook`, app `bboxy450-term`: all free on crates.io, workspace scaffolded.
- Plain cargo now; weavie + Nix when Nix is installed.
- Renderer architecture and the automated render-test harness: [[renderer]]. The harness is step one of the renderer.
- Priority cuts assumed: Claude, Codex, zsh in v0.1; single daemon process speaking the supervisor protocol internally; local root key only; config.toml read-mostly; undo-close and mute kept; Rust lab from week one; byte ring in.

## Open decisions blocking assignment

- **Vocabulary:** space vs cohort (see [[ux-pick]] header). Blocks `ui-model` naming.
- **Crate names** on crates.io (`bbt-*` prefix proposed). Blocks `Cargo.toml`.
- **Nix install** before W1.1 or not. Blocks the weavie layout.
- **Executor MCP tools** (minimal set proposed). Blocks W2.5 catalogue scope only.
- **Name pools** per cohort setting. Blocks nothing yet.
- **Remote seats in v0.1** (ssh for public VMs proposed). Blocks W4 scope only.
