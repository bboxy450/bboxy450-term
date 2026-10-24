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
updated: 2026-10-23T17:28:19Z
---
# Every workspace crate carries its declared licence files

**Property.** The workspace follows [[licensing]] (decision [[d34-licensing|#34]]): `bbt-core`
is Apache-2.0 with `LICENSE-APACHE` and `NOTICE`; other members carry
`LICENSE-MIT` and `LICENSE-APACHE` under MIT OR Apache-2.0. The root carries both
licence files and `NOTICE`. Vendored Alacritty and vte keep their upstream
licence copies in their own source directories. Files are nonempty regular
copies, not symlinks.
Manifests use SPDX expressions rather than `license-file`.

**Check.** `sh scripts/licenses.sh` reads the actual Cargo workspace members.
Exit 0 means held; any other exit names the missing file or conflicting
manifest. Absent Yarn crates are not required; when present they follow the
same dual-file rule. This fast check is in the default invariant set.

**Cost of a violation.** A published crate omits or misstates its terms, and
the source-to-distribution provenance is incomplete.
