---
id: 01M5NVBXD0W90XFDBQXG18BDAC
number: 213
type: comment
title: "task_system is six methods; the built-in it must reference has more"
status: open
author: name:Seraph key:eec764b7d2f4b9b8
on: "[[plugins]]"
on_digest: "sha256:a3b7a2b411f819d0fbaa532d587847fe91891bf9a4b9b14e0c2419c9c65d9212"
anchor: "§1 Contracts · task_system"
kind: objection
links:
  - { to: "[[plugins]]", rel: on }
created: 2026-10-24T02:20:49Z
updated: 2026-10-24T02:20:49Z
---
# task_system is six methods; the built-in it must reference has more

Review of `.bboxy/specs/plugins.md` revision 3 at main 9c2022a7 (digest pinned above), fresh eyes from an idle Executor at the operator's request ("review plugin draft and send message to you with feedback!"). Author grade inferred: written outside a cockpit seat, signed only by the machine-key commit. Not repeated here: the 21 comments of 2026-10-23, all resolved.

§1 lists `task_system` as `list, get, delegate, report, accept, reject` with "the same shapes as [[work-items]] and the W2.5d tools". The W2.5d tool set the built-in actually serves is wider: `task_status`, `task_reassign` and the `task_path` addressing (`crates/bbt-mux/src/mcp/work_items.rs:21-52`). A Linear or Jira plugin written against the six methods could not back the tools the cockpit already calls.

Also missing for a first real plugin: how a change made outside the app (a Linear edit) reaches the lane (poll on an interval, or a change feed the plugin pushes); the mapping between the app's record identity (ULID, global `number`) and the external id; and an optimistic-concurrency token so `report` or the stored signed record is not lost to a concurrent external edit.

**Ask.** Derive the contract from the W2.5d tool schemas (one method per tool, same JSON Schemas), add `watch` or `changes_since(cursor)`, an `external_id` field next to the record id, and an `etag` on every write. Then the built-in is a true reference implementation of it.
