---
id: 01M5NVBXD8V3AZGY83MW9MQSJ4
number: 216
type: comment
title: "a hook subscription exports data whatever the contract tier says"
status: resolved
author: name:Seraph key:eec764b7d2f4b9b8
on: "[[plugins]]"
on_digest: "sha256:a3b7a2b411f819d0fbaa532d587847fe91891bf9a4b9b14e0c2419c9c65d9212"
anchor: "§1 Hooks"
kind: objection
links:
  - { to: "[[plugins]]", rel: on }
created: 2026-10-24T02:20:49Z
updated: "2026-10-24T02:24:35Z"
---
# a hook subscription exports data whatever the contract tier says

Review of `.bboxy/specs/plugins.md` revision 3 at main 9c2022a7 (digest pinned above), fresh eyes from an idle Executor at the operator's request ("review plugin draft and send message to you with feedback!"). Author grade inferred: written outside a cockpit seat, signed only by the machine-key commit. Not repeated here: the 21 comments of 2026-10-23, all resolved.

A plugin's shape may ask for any hook event (§1 Hooks: "[[harness-adapters]] hooks, the app's own lifecycle events"). Those payloads carry prompts, tool inputs and outputs (file contents, shell commands, sometimes secrets). A `repo_graph` plugin, tier `read`, that subscribes to `PreToolUse` sees every tool call of every seat. That is export in practice, and the trust prompt as written shows only the contract's tier.

**Ask.** Classify hook events by the data they carry (lifecycle only, versus prompt or tool payload), shown at trust time next to the contract tier. A plugin's effective tier is the higher of its contract tier and its hook subscriptions. Payload-bearing events need their own grant, like `block` does.
