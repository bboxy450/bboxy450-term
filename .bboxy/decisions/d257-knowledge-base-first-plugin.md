---
id: 01M5P2MXKS6D96N2QMK9GSZNKD
type: decision
number: 257
title: "the knowledge base is the first bundled plugin: the .bboxy store behind the data_source contract, swappable in Settings"
status: decided
decided: "2026-10-24"
operator:
  - "so with plugin systems - start designing our document base or knowledge base or knowledge/document graph system as a bundled plugin as is, if you do it well we can then build others and swap them in settings, only that for now, later more will be added!"
links:
  - { to: "[[knowledge-base]]", rel: decides }
  - { to: "[[plugins]]", rel: builds-on }
  - { to: "[[d255-visualization-is-ours]]", rel: builds-on }
  - { to: "[[d41-plugins]]", rel: amends }
created: "2026-10-24T04:28:04Z"
updated: "2026-10-24T04:28:04Z"
---
# Decision 257: the knowledge base is the first bundled plugin

Decided by the operator on 2026-10-24. Spec [[knowledge-base]].

**Alternatives.** (a) Keep the document graph as app code and add a plugin seam later. (b) Design the existing `.bboxy` store as the bundled provider of the `data_source` contract now: in process by default, served over stdio for `plugin check` and for other suite apps, listed in the registry, swappable per knowledge root in Settings › Knowledge. (c) Build a foreign provider first (Obsidian, Notion) and retrofit the built-in.

**Choice.** (b).

**Reason.** The operator's words: do it well and others can be swapped in. The store already exists with records, wikilinks, numbers and images, so the cost is the contract and the trait, not a system; proving the seam on the thing we use every day is the only proof that counts; and the built-in stays the opinionated default ([[d255-visualization-is-ours]]). (a) ships nothing pluggable; (c) proves the seam on a system we do not run.

**Rules decided with it.** The contract has eight methods (resolve, fetch, search, list, links, graph, write, changes_since) in the app's shapes; the bundled provider implements it in process and over stdio from one code path; roots mount several at once with the repository's `.bboxy` first; a swap moves no data; numbers come from the app's Index under #48 for the bundled provider and `external_id` beside them for a foreign one; semantic search is a later additive minor version; the provider's code moves to `bboxy450-knowledge` at the first release, named now, not created. Rejected: an MCP hop for the default path (a cost with no seam gained); numbers allocated by a provider (two allocators); a provider that draws (#255).

**Scope.** Only this plugin for now; `code_host`, `event_source` and the rest stay named and unbuilt. Amends [[d41-plugins]]: the plugin system's first implementation is the knowledge base, not P-0 in the abstract; K-0 and P-0 are the same brief where they overlap (the `data_source` schema in `bbt-plugin`).

Owner: Corvin Duskmantle <ARCHITECT>.
