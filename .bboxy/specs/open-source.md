---
id: "450CWGNJJ7DM4AMABKSN19ZQME"
type: "spec"
number: 126
title: "Open-source libraries — bboxy450-*"
created: "2026-10-22T17:03:48Z"
updated: "2026-10-24T15:28:01Z"
links:
  - { to: "[[d39-suite-is-the-product]]", rel: decision }
---
# Open-source libraries — bboxy450-*

Owner: Vesper Greywind <ORACLE>. 2026-10-22. Status: DECIDED. Operator: "our wgpu GUI library, cross-platform rendering library and async IO library should be designed so we open-source them independently of the term app, all named with bboxy450 prefixes; publish to crates and GitHub one at a time when ready; document them so rustdoc gives agents nice docs."

## 1. The crates

The suite is the product and the terminal is its first app (decision [[d39-suite-is-the-product|#39]], operator 2026-10-23): every crate below is a reusable part, the app is glue, and later apps built on the same kit connect through `bbt-proto`, the bus and messaging.

| Public crate | From | What it is | Depends on |
|---|---|---|---|
| `bboxy450-rt` | `bbt-rt` | IO runtime that keeps the UI thread free: one tokio runtime, coalesced UI wake, bounded UI queues, streamed background commands, hitch meter | tokio |
| `bboxy450-ui-core` | generic part of `bbt-ui-model` | tokens and style records, layout engine (size, solve, split), draw list, input events; no GPU, no IO | none |
| `bboxy450-render` | `bbt-render` | wgpu renderer: device, passes, chrome (rects, shadows, materials, text, icons, paths, groups), cell grid, headless capture | `bboxy450-ui-core`, wgpu |
| `bboxy450-ui` | L3–L6 of `bbt-ui` | component library: Style, materials, atoms, Box/Text/Stack/Flow, widgets, overlays | `bboxy450-ui-core` |
| `bboxy450-book` | `bboxybook` (later) | native storybook and web↔native parity loop | the three above |
| `bboxy450-harness-wake-up` | `bbt-mux/src/adapters` ([[harness-kits]] §2) | how to tap into a harness: launch wired in, observe state, deliver natively, say whether it can be woken, fall back under a guard | `bboxy450-term-core`, tokio, tokio-tungstenite, mio |
| `bboxy450-harness-host` | `bbt-supervisor` ([[harness-kits]] §3) | run programs in PTYs, leases, capture rings, checkpoints, resume, input receipts | `bboxy450-term-core`, portable-pty, mio |

Stays private, inside bboxy450-term: the app, `bbt-core`, `bbt-mux`, and a new `bbt-term-ui` holding everything domain-specific: seats, swarms, tabs, sidebar layout, L7 modules (SeatRow, SwarmCard, Pane, StreamDock, …), seat kinds and colours.

**Boundary rule:** a public crate never depends on a private one, never names seats/swarms/harnesses, and builds with `cargo package` from its own directory (no reads outside the crate). CI checks the first two with `cargo tree` and a grep for domain words.

**Design data:** the bboxy design (tokens, style records) ships inside `bboxy450-ui-core` as the default theme, as data files in the crate; the API takes a theme so others can bring their own.

## 2. Docs standard (agent-readable)

Every public crate:
- `#![doc = include_str!("../README.md")]`, so GitHub, crates.io and docs.rs show one text.
- `#![deny(missing_docs)]`, `#![warn(rustdoc::broken_intra_doc_links)]`.
- The crate doc has these sections, in order: **What it is** (two sentences), **Mental model** (the 3–6 nouns and how they relate), **Invariants** (what always holds, e.g. "nothing here blocks the calling thread"), **Entry points** (in the order an agent should call them), **Minimal example** (a compiling doctest), **Recipes** (task → snippet), **Pitfalls**, **Platform support**, **Feature flags**.
- Every public item has a one-line summary, plus `# Errors`, `# Panics` and `# Examples` wherever they apply. Examples are doctests that run.
- One `examples/` program per main use.

## 3. Release

**Velocity first (operator, 2026-10-22):** releasing must never slow the build. No release cycle runs inside the build loop; the Oracle publishes a crate only when it reaches a natural plateau where its API has stopped moving. Until then, keep the public/private boundary (§1) and the docs standard (§2) in mind while building, so a release is a small step when it comes.

One crate at a time, when its docs meet §2, its tests pass and its API is one we are willing to keep. Version 0.1.0.
- Public repo `github.com/bboxy450/<crate>`. The monorepo stays private; each release is one signed snapshot commit of the crate directory plus LICENSE-MIT and LICENSE-APACHE (license `MIT OR Apache-2.0`, except `bbt-core`, Apache-2.0 as an alacritty fork; the per-crate map and file placement are [[licensing]]).
- The signing, publishing and announcing order is the one in `~/.agents/AGENTS.md`: `cargo package`, detached `gpg` signature of the `.crate`, signed tag, `cargo publish` as bboxy450, push tags, `gh release` with the `.crate` and `.asc`, announce on `#bboxy450-crates`.
- The Oracle runs the publish; an Executor prepares the crate.

Order: `bboxy450-rt` (ready now, after rename and docs) → `bboxy450-ui-core` → `bboxy450-render` → `bboxy450-ui` → `bboxy450-book`. The split of ui-model and ui waits until the W3.2d L7 modules finish (7.10), so it doesn't fight in-flight module work. Briefs: [[2026-10-22-o-open-source]].

Links: [[runtime]] [[renderer]] [[gui-components]] [[vision]]
