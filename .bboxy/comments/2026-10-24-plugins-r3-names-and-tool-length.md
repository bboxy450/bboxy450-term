---
id: 01M5NVBXE7CXGYRHSDJ7GXP4YS
number: 228
type: comment
title: "parsing <app>-<system>-<provider> and the MCP tool-name budget"
status: resolved
author: name:Seraph key:eec764b7d2f4b9b8
on: "[[plugins]]"
on_digest: "sha256:a3b7a2b411f819d0fbaa532d587847fe91891bf9a4b9b14e0c2419c9c65d9212"
anchor: "§1 Discovery and trust · §1 MCP"
kind: question
links:
  - { to: "[[plugins]]", rel: on }
created: 2026-10-24T02:20:49Z
updated: "2026-10-24T02:24:35Z"
---
# parsing <app>-<system>-<provider> and the MCP tool-name budget

Review of `.bboxy/specs/plugins.md` revision 3 at main 9c2022a7 (digest pinned above), fresh eyes from an idle Executor at the operator's request ("review plugin draft and send message to you with feedback!"). Author grade inferred: written outside a cockpit seat, signed only by the machine-key commit. Not repeated here: the 21 comments of 2026-10-23, all resolved.

The app name `bboxy450-term` contains a hyphen, systems use underscores (`task_system`), and providers may contain hyphens (`google-drive`). Under the operator's per-app prefix ("named <app name, bboxy450-term in this case>-<system it plugs in>-<provider name>"), the parse is unambiguous only if stated: strip the known app prefix, match the longest known system token, and the rest is the provider. Then `plugin_<system>_<provider>_<command>` must fit harness tool-name rules (character set and a maximum length; verify the current limits of the Claude and Codex tool APIs). `plugin_data_source_google-drive-shared_search_documents` is already 54 characters.

**Ask.** State the parse rule, the provider character set, and the maximum tool-name length in §1. A shape whose tool names would exceed it is refused at trust, not at the first MCP list.
