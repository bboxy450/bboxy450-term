---
id: 01M5N5S76CKT4XCMT311G578DH
number: 171
type: comment
title: "the Codex hook trust claim needs a primary source"
status: resolved
author: name:Wren key:22c0e081a5f4ac21
on: "[[plugins-sota]]"
on_digest: "sha256:46e7cfe68bb89f0ebd743e48dfcd4d768a50ce3708261a8aa1d577f893ae3d1e"
anchor: "§6 · Codex hooks"
kind: question
links:
  - { to: "[[plugins-sota]]", rel: on }
created: 2026-10-23T20:01:17Z
updated: 2026-10-23T20:13:06Z
---
# the Codex hook trust claim needs a primary source

Red-team review of `.bboxy/research/plugins-sota.md` at 16b4c98 (decision [[d41-plugins|#41]], the RFC loop's first run). Author grade inferred: written outside a cockpit seat, signed only by the machine-key commit ([[work-items]] RFC loop, aee9fda).

:96 cites `learn.chatgpt.com/docs/hooks` for Codex's hook behaviour, including "Codex requires you to review and trust the exact hook definition", changed hooks skipped until re-trusted. That is the strongest precedent for the trust design the spec lacks (see the repo-trust comment on [[plugins]]), and that host is not where Codex documents its configuration.

**Ask.** Confirm against Codex's own documentation or source (the openai/codex repository) at a pinned revision, or mark the line `unverified`.
