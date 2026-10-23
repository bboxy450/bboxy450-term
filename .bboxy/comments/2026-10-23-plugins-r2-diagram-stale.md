---
id: 01M5N6JP77HH2CR5V4R0YBDTJD
type: comment
title: "the diagram still shows the revision 1 design"
status: resolved
author: name:Wren key:22c0e081a5f4ac21
on: "[[plugins]]"
on_digest: "sha256:2c3ad29192ad9c89939bf8e8e8d2c69db0eddc11c33b0e15c715520f9559362f"
anchor: "§1 Shape of the system · diagram"
kind: objection
links:
  - { to: "[[plugins]]", rel: on }
created: 2026-10-23T20:17:31Z
updated: 2026-10-23T20:26:43Z
---
# the diagram still shows the revision 1 design

Red-team follow-up on `.bboxy/specs/plugins.md` revision 2 at edc5dc5 (decision #41). Author grade inferred: written outside a cockpit seat, signed only by the machine-key commit ([[work-items]] RFC loop, aee9fda).

The §1 mermaid diagram still reads `R -- "run bare: shape JSON" --> P` and `per-call timings, flame graphs` into Settings. Revision 2's text says the shape comes from `--shape`, read once at trust time and cached by digest, and that the registry shows a call and hook timeline that is a flame graph only for `trace: true` plugins. The diagram is the first thing a reader (or the guide's harness) sees, and it contradicts both rules.

**Ask.** Update the two edges: `--shape (once, at trust; cached by sha256)` and `call and hook timeline (spans when trace)`; add the trust step between discovery and the registry.
