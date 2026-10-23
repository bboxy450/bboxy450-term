---
id: 01M5NK2SRDYGAJ6R19X0G4NB0G
type: decision
number: 51
title: "the GUI kit publishes as bboxy450-frontmatter; internal crate names stay until the first release"
status: decided
decided: 2026-10-23
operator:
  - "bboxy450-gui library should be called bboxy450-frontmatter unless its too much to rename it, if too much its ok leave it as is"
links:
  - { to: "[[licensing]]", rel: decides }
  - { to: "[[open-source]]", rel: decides }
  - { to: "[[layering]]", rel: constrains }
  - { to: "[[d39-suite-is-the-product]]", rel: builds-on }
created: 2026-10-23T23:56:02Z
updated: 2026-10-23T23:56:02Z
---
# Decision 51: the GUI kit publishes as bboxy450-frontmatter

Decided 2026-10-23. The operator named the GUI kit **Frontmatter**: the UI is the front, and frontmatter is where every record's state lives ([[d53-evolve-and-restart]]). The name is taken; the rename is deferred, exactly as the operator allowed.

**Alternatives.** (a) Rename `bbt-ui`, `bbt-ui-model` and `bbt-render` now. (b) Keep the internal names and publish the kit as one umbrella crate `bboxy450-frontmatter` that re-exports `bboxy450-ui`, `bboxy450-ui-core` and `bboxy450-render` at the first release (O1), the individual crates publishing beside it under the names in [[licensing]] §1. (c) Leave the kit unnamed until release.

**Choice.** (b).

**Reason.** The rename today touches 12 `Cargo.toml` dependency lines and 455 references across records, scripts and the lab while five Executors hold worktrees against those names: that is the "too much" the operator named. The umbrella costs one crate and no churn, and it is what a user types: `bboxy450-frontmatter` in one dependency line gives the whole kit. The internal `bbt-*` names are allowed by [[bboxy450-prefix]] only while a crate is internal, so the rename happens at O1 as part of publishing, never as a separate sweep.

**What it names.** The component library, its model and the renderer, as one kit; `bboxybook` stays the kit's gallery (`bboxy450-book`); `bbt-doc` is a kit beside it (`bboxy450-doc`).

Owner: Corvin Duskmantle <ARCHITECT>.
