---
id: 01M5N6JP76ZEQ9DAR99859BSW7
type: comment
title: "where trust records live is unstated"
status: resolved
author: name:Wren key:22c0e081a5f4ac21
on: "[[plugins]]"
on_digest: "sha256:2c3ad29192ad9c89939bf8e8e8d2c69db0eddc11c33b0e15c715520f9559362f"
anchor: "§1 Discovery and trust"
kind: objection
links:
  - { to: "[[plugins]]", rel: on }
created: 2026-10-23T20:17:31Z
updated: 2026-10-23T20:26:43Z
---
# where trust records live is unstated

Red-team follow-up on `.bboxy/specs/plugins.md` revision 2 at edc5dc5 (decision #41). Author grade inferred: written outside a cockpit seat, signed only by the machine-key commit ([[work-items]] RFC loop, aee9fda).

Revision 2 closes most of [[2026-10-23-plugins-repo-trust]]: nothing runs before a trust record pins path and sha256, and repo-local discovery is off by default. It never says **where trust records are stored**. If a trust record can live in the repository (`.bboxy/` already holds `plugins/`), a hostile repository ships its binary together with a pre-made trust record that pins the binary's own digest, and the gate opens on clone exactly as before.

**Ask.** Trust records live only in the operator's own store (the app or suite support directory, next to the plugin registry), never in a repository and never read from one; a trust record found in a repository is ignored and reported. One sentence in §1 Discovery and trust closes it.
