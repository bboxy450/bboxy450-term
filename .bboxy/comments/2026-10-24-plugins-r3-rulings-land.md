---
id: 01M5NVBXECGCJ6FRKCR97J8G1N
number: 230
type: comment
title: "the operator's three later rulings land cleanly"
status: open
author: name:Seraph key:eec764b7d2f4b9b8
on: "[[plugins]]"
on_digest: "sha256:a3b7a2b411f819d0fbaa532d587847fe91891bf9a4b9b14e0c2419c9c65d9212"
anchor: "§0 · §1 Discovery and trust · revision 3"
kind: approval
links:
  - { to: "[[plugins]]", rel: on }
created: 2026-10-24T02:20:49Z
updated: 2026-10-24T02:20:49Z
---
# the operator's three later rulings land cleanly

Review of `.bboxy/specs/plugins.md` revision 3 at main 9c2022a7 (digest pinned above), fresh eyes from an idle Executor at the operator's request ("review plugin draft and send message to you with feedback!"). Author grade inferred: written outside a cockpit seat, signed only by the machine-key commit. Not repeated here: the 21 comments of 2026-10-23, all resolved.

Checked against revision 3. The per-app prefix ("named <app name, bboxy450-term in this case>-<system it plugs in, eg knowledge_base>-<provider name>") is in §1 Discovery, and the contracts stay shared in `bbt-plugin` for a second suite app. Jev as the `decision_model` provider, optional and never load-bearing ("we want to use it if it makes sense or we don't have to use it"), is in §0 and §1 Contracts, with the decision primitive stated correctly. Trust records live only in the operator's store, never read from a repository, in §1 Discovery and trust. Residual edges, filed separately: #41's summary still describes the pre-revision design (d41-drift), the name parse with hyphenated providers (names-and-tool-length), and `decision_model`'s tier (tier-table).
