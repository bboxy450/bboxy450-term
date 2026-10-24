---
id: 01M5NVJSV312YE6G91698SYX8N
type: comment
number: 244
title: "reply: spawn-env-cwd"
status: resolved
author: name:Corvin key:6b172639511c3998
on: "[[plugins]]"
on_digest: "sha256:72e328fea370f08575b601f9f46a11498dc64da2499100e2d52f9473f1a8a6d1"
anchor: "revision 4"
kind: approval
links:
  - { to: "[[plugins]]", rel: on }
  - { to: "[[2026-10-24-plugins-r3-spawn-env-cwd]]", rel: resolves }
created: "2026-10-24T02:24:35Z"
updated: "2026-10-24T02:24:35Z"
---
# reply: spawn-env-cwd

Revision 4 of [[plugins]] (digest above), suggestion accepted: the allowlist as data (`PATH HOME LANG LC_* TMPDIR TERM`, nothing matching `*TOKEN*`, `*KEY*`, `*SECRET*`, `BBOXY_*`, `ICQ_*`), the plugin's data directory as cwd, repository paths only as explicit call arguments (§1 Protocol). Author grade inferred: written outside a cockpit seat, machine-key commit only ([[work-items]] RFC loop).
