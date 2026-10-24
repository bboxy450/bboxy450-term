---
id: 01M5NVBXDDA1G2YT8KG0W31C8N
number: 218
type: comment
title: "the digest is checked, then a path is executed"
status: open
author: name:Seraph key:eec764b7d2f4b9b8
on: "[[plugins]]"
on_digest: "sha256:a3b7a2b411f819d0fbaa532d587847fe91891bf9a4b9b14e0c2419c9c65d9212"
anchor: "§1 Discovery and trust"
kind: objection
links:
  - { to: "[[plugins]]", rel: on }
created: 2026-10-24T02:20:49Z
updated: 2026-10-24T02:20:49Z
---
# the digest is checked, then a path is executed

Review of `.bboxy/specs/plugins.md` revision 3 at main 9c2022a7 (digest pinned above), fresh eyes from an idle Executor at the operator's request ("review plugin draft and send message to you with feedback!"). Author grade inferred: written outside a cockpit seat, signed only by the machine-key commit. Not repeated here: the 21 comments of 2026-10-23, all resolved.

A trust record "pins its path and sha256", and the app later runs the binary by path. Between the digest check and `exec` the file can be replaced, and the replacement runs under the old trust. macOS has no `fexecve`, so "check the open fd, run that fd" is not available. Separately, the digest covers the entry file only. A plugin shipped as a launcher script (`#!/bin/sh exec python main.py`) or with sidecar libraries is trusted for content the digest never saw.

**Ask.** At trust time, copy the binary into the operator's store as content-addressed (`plugins/bin/<sha256>`, read-only), and always exec that copy, never the discovered path. The trust prompt says plainly that only that file is pinned. Plugins that are directories either declare a payload manifest whose digest is pinned too, or are refused in v1.
