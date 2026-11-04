---
id: "5Q6E00XYC02G4T1GRJNN0ZHADD"
type: "task"
number: 76
title: "Task T: text rendering — how ghostty does it, what we take"
created: "2026-10-21T19:10:11Z"
updated: "2026-11-01T01:45:00Z"
status: "done"
reconciliation:
- by: seat:01M6ABH71TVWGZ621NTD701PSH
  disposition: superseded
  evidence:
  - https://github.com/bboxy450/bboxy450-term/commit/0f6a34d4
  fresh_accepted: false
  kind: historical
  previous_status: none
  reason: "KB cleanup 2026-10-31 (Oracle Nyx Copperwing, operator-requested): legacy record; body already marks it SUPERSEDED; plan replaced and later delivered by other tasks"
---
# Task T: text rendering — how ghostty does it, what we take

Date: 2026-10-21. Owner: Vesper Greywind <ORACLE>. Assignee: Oracle subagent (writes `research/text-rendering.md`). Status: SUPERSEDED (8fd15659). Previous brief wording: RUNNING. Redundant Executor pass welcome: write `research/text-rendering-exec.md`.

Operator: "we want the sexiest text possible, ghostty has nice text rendering, whatever they do." Renderer is raw wgpu, ours. See [[vision]] Stack.

Questions: ghostty's font discovery, shaping, rasterization, atlas, Metal pipeline, hinting and gamma choices, ligatures, emoji, fallback, subpixel positioning, HiDPI, cursor and underline styles. Then: the Rust equivalents (cosmic-text, swash, glyphon, rustybuzz, fontdb, core-text via objc2) and what we build ourselves to match ghostty's output on wgpu.
