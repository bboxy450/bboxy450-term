---
id: 01M5NVBXDNTQFASSW65WZM073F
number: 221
type: comment
title: "MINOR-additive needs an ignore-unknown rule, and streaming needs a primitive"
status: resolved
author: name:Seraph key:eec764b7d2f4b9b8
on: "[[plugins]]"
on_digest: "sha256:a3b7a2b411f819d0fbaa532d587847fe91891bf9a4b9b14e0c2419c9c65d9212"
anchor: "§1 Protocol · versions · §1 Isolation"
kind: suggestion
links:
  - { to: "[[plugins]]", rel: on }
created: 2026-10-24T02:20:49Z
updated: "2026-10-24T02:24:35Z"
---
# MINOR-additive needs an ignore-unknown rule, and streaming needs a primitive

Review of `.bboxy/specs/plugins.md` revision 3 at main 9c2022a7 (digest pinned above), fresh eyes from an idle Executor at the operator's request ("review plugin draft and send message to you with feedback!"). Author grade inferred: written outside a cockpit seat, signed only by the machine-key commit. Not repeated here: the 21 comments of 2026-10-23, all resolved.

"MINOR is additive" works only if both sides ignore fields they do not know. Docker states it outright ("Plugins should ignore unknown fields ... to remain forwards-compatible", [[plugins-sota]] :23). The spec does not, so a strict plugin deserialiser breaks on the app's next minor. And "streamed large payloads" is listed in §1 Isolation as a design obligation, but NDJSON request/response has no streaming primitive. A `data_source.fetch` of a large Notion page or a `repo_graph.describe` of a monorepo will hit it first.

**Ask.** P-0 states the ignore-unknown-fields rule in both directions and tests it in `plugin check` (it sends an unknown field and expects success). Pick one streaming form now: MCP-style cursor pagination for lists, plus chunked `notifications/<method>/chunk` with an id and a final marker (or a resource link to a temp file the app reads) for single large results.
