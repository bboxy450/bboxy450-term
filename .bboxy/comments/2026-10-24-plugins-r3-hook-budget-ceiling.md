---
id: 01M5NVBXDQ1P5V8FNXXZ4JH7WQ
number: 222
type: comment
title: "budget_ms is the plugin's own number; nothing caps it"
status: open
author: name:Seraph key:eec764b7d2f4b9b8
on: "[[plugins]]"
on_digest: "sha256:a3b7a2b411f819d0fbaa532d587847fe91891bf9a4b9b14e0c2419c9c65d9212"
anchor: "§1 Hooks"
kind: objection
links:
  - { to: "[[plugins]]", rel: on }
created: 2026-10-24T02:20:49Z
updated: 2026-10-24T02:20:49Z
---
# budget_ms is the plugin's own number; nothing caps it

Review of `.bboxy/specs/plugins.md` revision 3 at main 9c2022a7 (digest pinned above), fresh eyes from an idle Executor at the operator's request ("review plugin draft and send message to you with feedback!"). Author grade inferred: written outside a cockpit seat, signed only by the machine-key commit. Not repeated here: the 21 comments of 2026-10-23, all resolved.

The operator asked for "flamegraphs and timings and we can tell users hey this is taking 10s on each hook, its not ok". The spec measures overruns, but `budget_ms` comes from the plugin's shape, and a `before` hook is awaited. A shape declaring `budget_ms: 600000` on `PreToolUse` legally stalls every tool call of every seat for ten minutes, and never "overruns". Harnesses clamp their own: Codex clamps `SessionEnd` and `Interrupt` to 1-3 s, Claude Code gives `SessionEnd` 1.5 s ([[plugins-sota]] :184, :193). A plugin hook also runs inside the harness's own hook timeout.

**Ask.** An app-owned ceiling per event class, as data in `bbt-plugin`: for example 2 s on `before` for tool events, 1 s on session end, and longer only by an operator grant at trust time. A shape budget above the ceiling is refused at trust. A plugin hook's budget always fits inside the harness hook timeout that carries it.
