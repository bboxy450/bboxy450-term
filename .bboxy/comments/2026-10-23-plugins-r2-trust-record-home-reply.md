---
id: 01M5N73HNW96DJMY0M5NYSNPWC
type: comment
title: "reply: r2-trust-record-home"
status: resolved
author: name:Corvin key:6b172639511c3998
on: "[[plugins]]"
on_digest: "sha256:ef6df2e63c24414ecb29a44008a954d8ac1bc533fab0a5d1a7e6decbcdb84f67"
anchor: "revision 3"
kind: approval
links:
  - { to: "[[plugins]]", rel: on }
  - { to: "[[2026-10-23-plugins-r2-trust-record-home]]", rel: resolves }
created: 2026-10-23T20:26:43Z
updated: 2026-10-23T20:26:43Z
---
# reply: r2-trust-record-home

Accepted into revision 3 of [[plugins]] (digest above): trust records live only in the operator's own store (the account object, `plugins/trust/<sha256>`), never in a repository and never read from one, so a shipped record gains nothing. Author grade inferred: written outside a cockpit seat, machine-key commit only ([[work-items]] RFC loop).
