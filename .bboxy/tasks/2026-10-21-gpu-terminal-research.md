---
id: "7CGBHGAGE74J62JJ08JPTJK7G3"
type: "task"
number: 61
title: "Task: GPU terminal emulators in Rust — research"
created: "2026-10-21T18:53:32Z"
updated: "2026-10-24T00:10:38Z"
---
# Task: GPU terminal emulators in Rust — research

Date: 2026-10-21. Owner: Oracle. Assignee: Executor (Codex). Status: ASSIGNED.

## Goal

Decide what bboxy450-term is built on. We need a GPU-rendered terminal core in Rust that we can embed and extend with tabs, groups, splits, a sidebar, and in-process messaging. Produce research pointers and a decision-ready comparison.

## Scope

Research only. No code in this repo beyond `.bboxy/`. Read source of the candidates; do not paraphrase READMEs.

Candidates (add any you find):
- `alacritty` + `alacritty_terminal` crate (VTE state machine, PTY, grid; renderer in the app crate)
- `wezterm` (`wezterm-term`, `wezterm-gui`, `termwiz`, mux with tabs/panes/splits, Lua config)
- `rio` (`rio-backend`, `sugarloaf` renderer on wgpu)
- `ghostty` (Zig, not Rust — note why it is excluded or what to steal: libghostty embedding model)
- `warp` (closed; note only the public architecture)
- Rendering layers: `wgpu`, `glyphon`, `cosmic-text`, `swash`, `fontdue`
- Terminal state: `vte` crate, `alacritty_terminal`, `wezterm-term`, `termwiz`
- Windowing: `winit`, `tao`
- Embedding/GUI shells: `iced`, `egui`, `gpui` (Zed), `xilem`, `floem`

## Questions to answer

1. Which crate gives a reusable **terminal model** (grid, scrollback, VTE parsing, PTY) with a stable API we can depend on, and which bundle it with their renderer?
2. Which **GPU text rendering** path is viable: glyph atlas on wgpu (`glyphon`/`sugarloaf`), or roll our own? Measure: ligatures, emoji, fallback fonts, HiDPI, subpixel.
3. **Multiplexing**: what does wezterm's mux give us for tabs/panes/splits, and can it be reused without the Lua layer?
4. **Embedding a GUI chrome** (sidebar, tab bar) alongside GPU terminal surfaces: which GUI toolkit shares a `wgpu` device and surface cleanly? Zed's `gpui` is the strongest reference for "editor-grade GPU UI in Rust" — assess its usability outside Zed.
4b. **Shaders in chrome.** The operator wants the whole UI GPU-rendered with custom shaders (glow, blur, animated state). For each GUI toolkit candidate: can we inject a custom wgpu render pass or shader into a widget, and does it share the wgpu device/surface with the terminal grid? Note `vello`, `iced` custom shader widgets, `egui` paint callbacks, `gpui` render primitives. Also: does any candidate offer a storybook-style component gallery pattern.
5. Licensing of each candidate (MIT/Apache/GPL). Flag anything that constrains us.
6. Maintenance health: last release, commit cadence, open-issue trend, bus factor.
7. macOS Metal path specifics (via wgpu), and Linux Wayland/X11 if cheap to note.

## Deliverables

- `.bboxy/research/gpu-terminals-rust.md` — per-candidate section: repo, license, version/date, crate layout, `file:line — mechanism` pointers for PTY, parser, grid, renderer, mux. Measured facts only.
- `.bboxy/specs/terminal-core.md` — a recommendation: which crates we build on, what we write ourselves, with the two strongest alternatives and why they lost.

## Acceptance

- Every candidate above has a section with license and last-release date.
- At least one `file:line` pointer per candidate for the parser and renderer entry points, read from a local clone.
- The spec names one primary stack and states the reversibility of that choice.
- Clones go under `.bboxy/runs/clones/` or `/tmp`; they are not committed.

## Verification

```sh
ls .bboxy/research/gpu-terminals-rust.md .bboxy/specs/terminal-core.md
grep -c 'License' .bboxy/research/gpu-terminals-rust.md
```

## Report back

On `#bboxy450-term`: files written, commands run with exit status, what was not checked.
