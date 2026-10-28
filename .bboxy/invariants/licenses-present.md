---
id: 01M5MWWVY42W6XV6GK64N36N8N
number: 155
type: invariant
title: every workspace crate carries its declared licence files
status: active
check: sh scripts/licenses.sh
slow: false
cost: missing or conflicting terms break distribution provenance and make a packaged crate ambiguous
links:
  - { to: "[[d34-licensing]]", rel: decision }
  - { to: "[[licensing]]", rel: constrains }
created: 2026-10-23T17:28:19Z
updated: 2026-10-28T08:20:00Z
---
# Every workspace crate carries its declared licence files

**Property.** Every actual workspace member follows [[licensing]]: local
`license-file = "LICENSE"`, the complete Respectful Slop License 1.0-Draft,
byte-identical `NOTICE` and `ATTRIBUTION.md` copies, and retained original
MIT/Apache texts. The map explicitly preserves upstream terms and prior grants.
Alacritty/vte, Yarn, Mermaid fixtures, xterm reference assets and the JetBrains
Mono font retain their original licenses and notices. Required documents are
nonempty regular files, never symlinks. New source excerpts retain project
identity and an absolute policy reference without overwriting upstream headers.

**Check.** `sh scripts/licenses.sh` reads actual Cargo workspace members and the
map, checks declarations and canonical copies, and preserves upstream file
requirements. Exit 0 means held; any other exit names the missing or conflicting
material. `make test-licenses` also exercises negative fixtures and source
coverage. `make check-license-packages` verifies Cargo's selected package files.
The fast check remains in the default invariant set; no credentials or operator
interaction is required.

**Cost of a violation.** A published crate omits or misstates its terms, and
source excerpts or binary distributions lose their origin and required notices.
