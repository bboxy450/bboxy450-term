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
updated: "2026-10-28T22:15:55Z"
---
# Licensing

## Current adoption — operator, 2026-10-28

The operator supplied **Respectful Slop License 1.0-Draft** and directed:
“integrate this everywhere in our crates!” The complete supplied text is root
`LICENSE`, including the appendix. This replaces the default for bboxy450-owned
contributions. The earlier dual-license decisions remain historical records;
this adoption does not withdraw prior grants or relicense upstream material.
Project identity is **bboxy450-term**, canonical URL
https://github.com/bboxy450/bboxy450-term. Root `ATTRIBUTION.md` describes the
scope and provides downstream examples; `NOTICE` carries factual notices.

Copied, adapted or translated protected material under the new license carries
accurate source attribution under Section 10(a)–(e), even for internal reuse and
automated tools. Section 10(f)'s independent-inspiration credit remains an
explicitly nonbinding community request. Do not claim SPDX listing, OSI approval,
blanket ownership of upstream code, or that changing a header revokes old grants.

## The map

All workspace crates carry local `license-file = "LICENSE"`, because the custom
license is not an SPDX-listed expression. Each local copy has the complete
license; companion `NOTICE` and `ATTRIBUTION.md` identify mixed material and
prior grants. The package's own source headers and upstream notices control
the scope within that package.

| Crate | Publishes as | License | Why | Vendored or third-party code |
|---|---|---|---|---|
| `bbt-core` | `bboxy450-term-core` | Respectful Slop License 1.0-Draft; upstream terms and prior grants retained | Custom terms for separately identified bboxy450-owned additions; pre-adoption fork stays Apache-2.0; vte retains its dual terms | `vendor/alacritty/*` (Apache-2.0, alacritty authors), `vendor/vte` (MIT OR Apache-2.0, vte authors), and `src/keyboard.rs`, an adaptation of alacritty `input/keyboard.rs build_sequence` at 5421470 (Apache-2.0, upstream header kept plus a "modified by bboxy450" line; decision [[d36-apache-code-placement\|#36]]); their notices stay verbatim |
| `bbt-rt` | `bboxy450-rt` | Respectful Slop License 1.0-Draft; upstream terms and prior grants retained | bboxy450-owned contributions use LICENSE; retained upstream material keeps its own terms | none |
| `bbt-yarn`, `bbt-yarn-macros` | `bboxy450-yarn`, `bboxy450-yarn-macros` | Respectful Slop License 1.0-Draft; upstream terms and prior grants retained | Custom terms for separately identified bboxy450-owned additions; pinned kittens-old/Yarn remains MIT OR Apache-2.0 | from `Gonzih/kittens-old` 293e134, the operator's code; upstream README and license files kept as provenance |
| `bbt-mermaid` | `bboxy450-mermaid` | Respectful Slop License 1.0-Draft; upstream terms and prior grants retained | bboxy450-owned contributions use LICENSE; retained upstream material keeps its own terms | Mermaid MIT documentation fixtures, with attribution and license in tests/fixtures; original import revision unknown |
| `bbt-ui-model` | `bboxy450-ui-core` | Respectful Slop License 1.0-Draft; upstream terms and prior grants retained | bboxy450-owned contributions use LICENSE; retained upstream material keeps its own terms | none |
| `bbt-render` | `bboxy450-render` | Respectful Slop License 1.0-Draft; upstream terms and prior grants retained | bboxy450-owned contributions use LICENSE; retained upstream material keeps its own terms | JetBrains Mono test font under OFL-1.1; tests/fonts/OFL.txt retained |
| `bbt-ui` | `bboxy450-ui`, and with `bbt-ui-model` and `bbt-render` as the umbrella **`bboxy450-frontmatter`** ([[d51-frontmatter-is-the-gui-kit]]) | Respectful Slop License 1.0-Draft; upstream terms and prior grants retained | bboxy450-owned contributions use LICENSE; retained upstream material keeps its own terms | none |
| `bbt-doc` | `bboxy450-doc` | Respectful Slop License 1.0-Draft; upstream terms and prior grants retained | bboxy450-owned contributions use LICENSE; retained upstream material keeps its own terms | none |
| `bboxybook` | `bboxy450-book` | Respectful Slop License 1.0-Draft; upstream terms and prior grants retained | bboxy450-owned contributions use LICENSE; retained upstream material keeps its own terms | none |
| `bbt-supervisor` | `bboxy450-harness-host` ([[harness-kits]] §3) | Respectful Slop License 1.0-Draft; upstream terms and prior grants retained | bboxy450-owned contributions use LICENSE; retained upstream material keeps its own terms | none |
| `bboxy450-harness-wake-up` | itself (`crates/bboxy450-harness-wake-up`, [[harness-kits]] §2; extracted from `bbt-mux/src/adapters`) | Respectful Slop License 1.0-Draft; upstream terms and prior grants retained | bboxy450-owned contributions use LICENSE; retained upstream material keeps its own terms | none |
| `bbt-plugin` | `bboxy450-plugin` ([[plugins]] §1, the shape and contract schemas, tiers, ceilings, the trust-record type) | Respectful Slop License 1.0-Draft; upstream terms and prior grants retained | bboxy450-owned contributions use LICENSE; retained upstream material keeps its own terms | none |
| `bbt-proto` | internal until decision [[d8-open-source\|#8]]'s plateau | Respectful Slop License 1.0-Draft; upstream terms and prior grants retained | bboxy450-owned contributions use LICENSE; retained upstream material keeps its own terms | none |
| `bbt-mux` | internal until decision [[d8-open-source\|#8]]'s plateau | Respectful Slop License 1.0-Draft; upstream terms and prior grants retained | bboxy450-owned contributions use LICENSE; retained upstream material keeps its own terms | none |
| `bboxy-firehose` | `bboxy450-term-firehose` (was `bboxy-firehose` 0.1.0, to be yanked) | Respectful Slop License 1.0-Draft; upstream terms and prior grants retained | bboxy450-owned contributions use LICENSE; retained upstream material keeps its own terms | xterm.js MIT reference assets in reference/vendor, excluded from Cargo package |
| `bboxy450-keychain`, `bboxy450-keychain-macos`, `bboxy450-keychain-linux`, `bboxy450-keychain-windows`, `bboxy450-keychain-file` | themselves ([[id1-seat-chain]] §5, decision #281) | Respectful Slop License 1.0-Draft; upstream terms and prior grants retained | bboxy450-owned contributions use LICENSE; retained upstream material keeps its own terms | none |
| `bboxy450-term-desktop-core` | itself ([[d362-crate-names-app-then-package]], [[d46-cross-os-facades]]) | Respectful Slop License 1.0-Draft; upstream terms and prior grants retained | bboxy450-owned platform-free desktop actions, menus and stop ports use LICENSE; retained upstream material keeps its own terms | none |
| `bboxy450-term-desktop-macos` | itself ([[d362-crate-names-app-then-package]], [[d46-cross-os-facades]]) | Respectful Slop License 1.0-Draft; upstream terms and prior grants retained | bboxy450-owned AppKit adapter and native probe use LICENSE; retained upstream material keeps its own terms | none |
| `bboxy450-term-desktop` | itself ([[d362-crate-names-app-then-package]], [[d46-cross-os-facades]]) | Respectful Slop License 1.0-Draft; upstream terms and prior grants retained | bboxy450-owned OS-agnostic facade over desktop-core and per-OS adapters uses LICENSE; retained upstream material keeps its own terms | none |
| `bboxy450-desktop-notifications`, `bboxy450-desktop-notifications-macos`, `bboxy450-desktop-notifications-windows`, `bboxy450-desktop-notifications-linux` | themselves ([[desktop-notifications]]) | Respectful Slop License 1.0-Draft; upstream terms and prior grants retained | bboxy450-owned contributions use LICENSE; retained upstream material keeps its own terms | none |
| `bboxy450-term` (the app) | the distributed binary | Respectful Slop License 1.0-Draft; upstream terms and prior grants retained | bboxy450-owned contributions use LICENSE; linked code retains its own terms | Linked upstream notices ship in the bundle; identity and logos retain their respective rights |

## Files, extracts and distributions

- Root and every member carry regular, byte-identical copies of `LICENSE`,
  `NOTICE`, and `ATTRIBUTION.md`. Original `LICENSE-MIT` / `LICENSE-APACHE` files
  remain byte-for-byte intact; core and Yarn have distinct upstream copies.
- Project-authored source headers carry identity, canonical source, custom
  license and an absolute attribution-policy link. Retained core/Yarn fork
  source receives only accurately scoped project integration credit; its
  original license/copyright headers remain intact. Vendor assets, golden
  data and upstream fixture contents are never overwritten by blanket headers.
- Distinctive implementation/design comments carry compact origin references
  so a copied function can retain credit without its file header. Formats
  without comments use the path/material map in `ATTRIBUTION.md` or a companion
  attribution file.
- Mirrors under `release-inputs/` come from canonical sources through Make.
  Generated sources retain their generator origin. Change generators rather
  than editing generated files independently.
- App install/release bundles include `LICENSE`, `NOTICE`, `ATTRIBUTION.md`
  and the retained MIT/Apache texts. Cargo source packages include the same
  files plus any actual upstream assets and accompanying licenses. No ordinary
  user-interface advertising attribution is required by the new license.

## Gates

Use `make sync-licenses` to refresh only the canonical custom license/policy/
notice copies. It must not overwrite retained upstream license texts.
`make test-licenses` checks declarations, copies, source notices and negative
fixtures without operator input. `make check-license-packages` verifies actual
Cargo package file lists for every member and required upstream content.
`make package-crate` checks licensing before packaging/signing; application
bundle recipes check and copy the documents. The fast `licenses-present`
invariant checks the declared map and required files. `make test` on the exact
signed source and `scripts/land.sh` remain required before landing.

## Known provenance limits

The Mermaid crate README identifies forty fixtures as copied documentation
examples, but their original import revision was not recorded. Keep their MIT
license and companion attribution, name that gap, and never invent a revision.
Pre-existing harness logos do not acquire bboxy450 ownership through these
headers. Third-party dependency licensing remains governed by each dependency;
the existing dependency gate is separate from this project's custom license.
