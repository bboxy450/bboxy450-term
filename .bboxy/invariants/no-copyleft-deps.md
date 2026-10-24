---
id: 01M5NKJXM8F0FTX7HXE7HQVD0W
type: invariant
number: 211
title: "every dependency offers a permissive licence"
status: active
check: sh scripts/deps-licences.sh
cost: one copyleft-only crate in the lockfile relicenses the product the day it ships; corporate friendly dies on that line
links:
  - { to: "[[licensing]]", rel: constrains }
  - { to: "[[d50-licence-when-mit-when-apache]]", rel: decision }
  - { to: "[[licenses-present]]", rel: sibling }
  - { to: "[[mantras]]", rel: mantra }
created: 2026-10-24T00:04:50Z
updated: 2026-10-24T00:04:50Z
---
# Every dependency offers a permissive licence

**Property.** Every package in `Cargo.lock` outside the workspace declares a
licence expression with at least one permissive alternative (MIT, Apache-2.0,
BSD, ISC, Zlib, Unicode, CC0, Unlicense, 0BSD, BSL-1.0, MPL-2.0, OpenSSL,
MIT-0, NCSA); no copyleft-only crate (GPL, LGPL, AGPL) and no crate without a
licence field enters the tree ([[licensing]] §0a, [[d50-licence-when-mit-when-apache]]:
"corporate friendly, hardcore developer first"; [[identity]] "no LGPL in the tree").

**Check.** `sh scripts/deps-licences.sh` — `cargo metadata --locked` over the
whole lockfile; exit 0 held, non-zero names the crate and its expression. Fast,
in the default set. A new permissive licence id is added to the list by a
decision, not a silent edit.

**Cost of a violation.** One copyleft-only crate relicenses the product the
day it ships; the corporate half of the licence rule dies on that line.

**Evidence.** `sh scripts/deps-licences.sh` — `deps-licences: held`, 483
dependencies, none missing a field, at the commit that adds it.

Owner: Corvin Duskmantle <ARCHITECT>.
