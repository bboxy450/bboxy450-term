---
id: 01M5NVBXE2TFQ7X828RZWSAPHR
number: 226
type: comment
title: "what a Notion plugin still needs from data_source"
status: resolved
author: name:Seraph key:eec764b7d2f4b9b8
on: "[[plugins]]"
on_digest: "sha256:a3b7a2b411f819d0fbaa532d587847fe91891bf9a4b9b14e0c2419c9c65d9212"
anchor: "§1 Contracts · data_source"
kind: question
links:
  - { to: "[[plugins]]", rel: on }
created: 2026-10-24T02:20:49Z
updated: "2026-10-24T02:24:35Z"
---
# what a Notion plugin still needs from data_source

Review of `.bboxy/specs/plugins.md` revision 3 at main 9c2022a7 (digest pinned above), fresh eyes from an idle Executor at the operator's request ("review plugin draft and send message to you with feedback!"). Author grade inferred: written outside a cockpit seat, signed only by the machine-key commit. Not repeated here: the 21 comments of 2026-10-23, all resolved.

`data_source` is "search, fetch, write". For Notion specifically:

- the document model: Markdown, or Notion's block tree? The app's records are Markdown with frontmatter; a lossless round trip needs a stated mapping, or `write` is append-only.
- pagination: Notion paginates with cursors; `search` and `fetch` need a cursor field.
- rate limits: Notion throttles per integration. The error taxonomy needs a retryable command error with `retry_after_ms`, so the app backs off instead of the plugin sleeping past its deadline.
- accounts: one plugin, several workspaces. Is the workspace an argument of every call, or a setup choice?
- write conflicts: an `etag` or last-edited time on `write`, as for `task_system`.
