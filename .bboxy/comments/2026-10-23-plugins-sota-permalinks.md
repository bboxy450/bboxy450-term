---
id: 01M5N5S76BSWW66E3EG4MV31WM
type: comment
title: "load-bearing sources point at moving branches"
status: resolved
author: name:Wren key:22c0e081a5f4ac21
on: "[[plugins-sota]]"
on_digest: "sha256:46e7cfe68bb89f0ebd743e48dfcd4d768a50ce3708261a8aa1d577f893ae3d1e"
anchor: "header · sources"
kind: objection
links:
  - { to: "[[plugins-sota]]", rel: on }
created: 2026-10-23T20:01:17Z
updated: 2026-10-23T20:13:06Z
---
# load-bearing sources point at moving branches

Red-team review of `.bboxy/research/plugins-sota.md` at 16b4c98 (decision #41, the RFC loop's first run). Author grade inferred: written outside a cockpit seat, signed only by the machine-key commit ([[work-items]] RFC loop, aee9fda).

The format is `source — fact`, but the sources are live URLs on moving branches (`github.com/docker/cli/blob/master/...` :20-21, `raw.githubusercontent.com/hashicorp/go-plugin/main/...` :40, :43) with one retrieval date for the whole file (:13). The facts the spec builds on (Docker's search order and shadowing, go-plugin's bare-run refusal and handshake) can change under the link, and nobody can later show what was read.

**Ask.** For each load-bearing claim, a permalink at a commit SHA or a tagged release, plus its retrieval date. That is the provenance rule the rest of `.bboxy/` follows.
