---
id: 01M5NK2SRCGPPGZC2Q4DCX9MH2
type: invariant
number: 54
title: everything we publish is branded bboxy450-
status: active
check: sh scripts/brand.sh
cost: an unbranded crate, binary or channel is one more name for a user to learn and one less hit for the one they search; the brand is how the suite reads as one thing
links:
  - { to: "[[licensing]]", rel: constrains }
  - { to: "[[plugins]]", rel: constrains }
  - { to: "[[desktop-notifications]]", rel: constrains }
  - { to: "[[d52-brand-and-face]]", rel: decision }
created: 2026-10-23T23:56:02Z
updated: 2026-10-23T23:56:02Z
---
# Everything we publish is branded bboxy450-

**Property.** Every name the outside world meets starts with `bboxy450-`: every
published crate (`bboxy450-rt`, `bboxy450-ui`, `bboxy450-frontmatter`, …), the
app binary, the GitHub organisation `bboxy450/`, the channels `#bboxy450-*`,
plugin binaries `bboxy450-term-<system>-<provider>` ([[plugins]]), and every
cross-OS kit ([[desktop-notifications]]). A crate for one app is named
`bboxy450-<app>-<package>` and a crate any app can use `bboxy450-<package>`
([[d362-crate-names-app-then-package]]). Internal crate names (`bbt-*`) are
allowed only while a crate is internal and sets `publish = false`; its row in [[licensing]] §1 names the
`bboxy450-*` it publishes as, or says internal (operator, 2026-10-23: "everything
is branded as bboxy450- prefix, everything!").

**Check.** `sh scripts/brand.sh` — `cargo metadata` over the workspace against
the licensing map: a crate is named `bboxy450-*`, or its row publishes as
`bboxy450-*`, or its row says internal; exit 0 held, non-zero names the crate.
Fast, in the default set. A new crate adds its row in the same commit.

**Cost of a violation.** One more name for a user to learn and one less hit for
the one they search; the suite stops reading as one thing.

**Evidence.** `sh scripts/brand.sh` — `brand: held` at the commit that adds it.

Owner: Corvin Duskmantle <ARCHITECT>.
