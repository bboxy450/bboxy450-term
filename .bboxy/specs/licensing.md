---
id: 01M5MSBWYSH6HK73HKZ8E9ZYV4
number: 151
type: spec
title: "licensing — which crate carries which license, and why"
status: decided
links:
  - { to: "[[d8-open-source]]", rel: decision }
  - { to: "[[d34-licensing]]", rel: decision }
  - { to: "[[d36-apache-code-placement]]", rel: decision }
  - { to: "[[d50-licence-when-mit-when-apache]]", rel: decision }
  - { to: "[[d51-frontmatter-is-the-gui-kit]]", rel: decision }
  - { to: "[[d52-brand-and-face]]", rel: decision }
  - { to: "[[open-source]]", rel: constrains }
  - { to: "[[terminal-core]]", rel: spec }
  - { to: "[[yarn]]", rel: spec }
  - { to: "[[2026-10-22-architecture-round-1]]", rel: decisions }
created: 2026-10-23T16:26:37Z
updated: "2026-10-24T15:28:01Z"
---
# Licensing

Owner: Corvin Duskmantle <ARCHITECT>. Decided 2026-10-23 (decision [[d34-licensing|#34]]). Operator: "ok, lets do dual license, be smart about this, ask architect as well what project will have what license." **Default: MIT OR Apache-2.0**, the Rust ecosystem's dual license, for every crate we wrote. **One exception forced by provenance: `bbt-core` is Apache-2.0 only**, because it is a fork of `alacritty_terminal`, whose upstream is Apache-2.0 only; a derivative cannot be offered under MIT alone, and relicensing someone else's Apache code is not ours to do. Facts at main when written: workspace `Cargo.toml:22` says `MIT OR Apache-2.0`; `crates/bbt-core/Cargo.toml:5` already says `Apache-2.0` and carries `LICENSE-APACHE` and `LICENSE-MIT` (the MIT text travels for the dual-licensed `vte` it vendors, so it stays beside the Apache text and NOTICE); no license file existed at the repository root until LIC1 (Kestrel, 38cb776, 2026-10-23: root `LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE`, both files in every crate, `scripts/licenses.sh` as [[licenses-present]]); no alacritty code exists outside `bbt-core` (`git grep -i alacritty` over render, ui and rt is empty, confirming [[terminal-core]] D3: design lifted, no code); `kittens-old` at 293e134 ships `LICENSE-MIT` and `LICENSE-APACHE`.

Why dual and not Apache-only: dual keeps GPLv2 compatibility through the MIT arm and matches what every dependency we lean on does; a user who wants the patent grant picks Apache. Why not MIT-only: it would drop the patent grant and clash with the one Apache crate at the bottom of the stack. Lens: the person opening a crate's directory on crates.io or GitHub must see its exact terms without reading the monorepo; so each publishing crate carries its own files.

## 0a. When MIT, when Apache ([[d50-licence-when-mit-when-apache]], operator: "corporate friendly, hardcore developer first")

Ours and clean: `MIT OR Apache-2.0`; the developer takes MIT (one paragraph, GPLv2-compatible), the corporation takes Apache-2.0 (the patent grant its lawyers already approved), nobody asks us. Contains Apache-only upstream: `Apache-2.0`, because a derivative of Apache code cannot be offered under MIT and we do not relicense other people's work. Contains MIT-only upstream: still dual for our code, upstream headers verbatim, NOTICE lists it. Never copyleft or source-available, the app included; no CLA, the signed commit is the contribution record. The name, the fox and the brand are identity, excluded from every code licence by a NOTICE line ([[d52-brand-and-face]]).

## 1. The map

| Crate | Publishes as | License | Why | Vendored or third-party code |
|---|---|---|---|---|
| `bbt-core` | `bboxy450-term-core` | **Apache-2.0** | fork of `alacritty_terminal` (Apache-2.0 only) with the `vte` parser (MIT OR Apache-2.0) vendored; the crate takes the stricter upstream | `vendor/alacritty/*` (Apache-2.0, alacritty authors), `vendor/vte` (MIT OR Apache-2.0, vte authors), and `src/keyboard.rs`, an adaptation of alacritty `input/keyboard.rs build_sequence` at 5421470 (Apache-2.0, upstream header kept plus a "modified by bboxy450" line; decision [[d36-apache-code-placement\|#36]]); their notices stay verbatim |
| `bbt-rt` | `bboxy450-rt` | MIT OR Apache-2.0 | ours | none |
| `bbt-yarn`, `bbt-yarn-macros` | `bboxy450-yarn`, `bboxy450-yarn-macros` | MIT OR Apache-2.0 | the operator's own library, dual upstream ([[yarn]] §1) | from `Gonzih/kittens-old` 293e134, the operator's code; upstream README and license files kept as provenance |
| `bbt-mermaid` | `bboxy450-mermaid` | MIT OR Apache-2.0 | ours; the engine is our own parse and layout | test fixtures copied from the Mermaid docs are test data, attributed in the fixtures README (Mermaid is MIT) |
| `bbt-ui-model` | `bboxy450-ui-core` | MIT OR Apache-2.0 | ours, including the design tokens | none |
| `bbt-render` | `bboxy450-render` | MIT OR Apache-2.0 | ours; alacritty's renderer design was lifted, no code (verified) | none |
| `bbt-ui` | `bboxy450-ui`, and with `bbt-ui-model` and `bbt-render` as the umbrella **`bboxy450-frontmatter`** ([[d51-frontmatter-is-the-gui-kit]]) | MIT OR Apache-2.0 | ours | none |
| `bbt-doc` | `bboxy450-doc` | MIT OR Apache-2.0 | ours | none |
| `bboxybook` | `bboxy450-book` | MIT OR Apache-2.0 | ours; `capture.mjs` drives a local Chrome, nothing vendored | none |
| `bbt-supervisor` | `bboxy450-harness-host` ([[harness-kits]] §3) | MIT OR Apache-2.0 | ours; `portable-pty` is a dependency, not vendored | none |
| `bboxy450-harness-wake-up` | itself (`crates/bboxy450-harness-wake-up`, [[harness-kits]] §2; extracted from `bbt-mux/src/adapters`) | MIT OR Apache-2.0 | ours; hook event names and socket protocols are facts about other programs, nothing vendored | none |
| `bbt-plugin` | `bboxy450-plugin` ([[plugins]] §1, the shape and contract schemas, tiers, ceilings, the trust-record type) | MIT OR Apache-2.0 | ours | none |
| `bbt-proto` | internal until decision [[d8-open-source\|#8]]'s plateau | MIT OR Apache-2.0 | ours | none |
| `bbt-mux` | internal until decision [[d8-open-source\|#8]]'s plateau | MIT OR Apache-2.0 | ours | none |
| `bboxy-firehose` | internal tool | MIT OR Apache-2.0 | ours | none |
| `bboxy450-desktop-notifications`, `bboxy450-desktop-notifications-macos`, `bboxy450-desktop-notifications-windows`, `bboxy450-desktop-notifications-linux` | themselves ([[desktop-notifications]]) | MIT OR Apache-2.0 | ours; `objc2-user-notifications` (Zlib OR Apache-2.0 OR MIT) is a dependency, not vendored | none |
| `bboxy450-term` (the app) | the distributed binary | MIT OR Apache-2.0 for our code, and **only our code**: anything adapted from an Apache-only source goes into `bbt-core` (decision [[d36-apache-code-placement\|#36]]), so the app never needs a mixed SPDX expression; the binary ships the NOTICE because it links `bbt-core` | the identity image `bboxy.png` is the operator's; fonts and icon sets, when bundled, carry their own license beside them and in NOTICE (none bundled today) |

## 2. Files and where they go

- **Root:** `LICENSE-MIT`, `LICENSE-APACHE`, and `NOTICE`. NOTICE also carries the identity line: the name bboxy450, the fox in `.bboxy/images/bboxy.png` and the brand are the operator's identity and are not licensed with the code ([[d52-brand-and-face]]). NOTICE is otherwise the third-party inventory: alacritty's copyright line and Apache-2.0, vte's dual license, kittens-old's upstream notice, and every bundled font or icon set as it arrives. The README states the dual default and the `bbt-core` exception in one sentence.
- **Each publishing crate directory** carries its own copies, because `cargo package` includes only files inside the crate: `LICENSE-MIT` and `LICENSE-APACHE` for dual crates; `LICENSE-APACHE` and `NOTICE` for `bbt-core`. Copies, not symlinks (cargo does not follow them reliably).
- **Manifests:** `license.workspace = true` (dual) everywhere except `bbt-core`'s `license = "Apache-2.0"`. No `license-file` fields; SPDX expressions only.
- **Vendored code** keeps upstream license files and file headers verbatim inside its vendor directory; we never edit a header. A vendored file we modify gains a one-line "modified by bboxy450" note under the original header, as Apache §4(b) asks.
- **Internal crates** carry the same two files from day one, so a later publish is a `cargo package` and nothing else ([[open-source]] §3 velocity rule).

## 3. The sweep (Executor task) and its gate

One task: add the root files, copy the per-crate files, write NOTICE, add the README sentence, and `scripts/licenses.sh`: for every crate in the workspace, assert the manifest license is on the map and the files that license requires are present in the crate directory, plus the three root files; exit 0 held. That script becomes `.bboxy/invariants/licenses-present.md` in the same commit (the switch lands with the load). Acceptance: the script exits 0; `cargo package --list -p <crate>` for each publishing crate shows its license files; `clippy-clean` and `layering` unchanged.

## 4. Out of scope

Choosing a license for the lab's web prototypes (they are references, not shipped), CLA or DCO policy, SBOM generation (`cargo-about` or similar, later, with the first release).
