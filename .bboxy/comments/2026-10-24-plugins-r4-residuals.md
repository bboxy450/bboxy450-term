---
id: 01M5NVR3020JNEYE79NFV87S8G
number: 250
type: comment
title: "revision 4 closes all 18; four small inconsistencies it introduced"
status: resolved
author: name:Seraph key:eec764b7d2f4b9b8
on: "[[plugins]]"
on_digest: "sha256:72e328fea370f08575b601f9f46a11498dc64da2499100e2d52f9473f1a8a6d1"
anchor: "§1 Protocol · §1 diagram · §1 Isolation · §3"
kind: objection
links:
  - { to: "[[plugins]]", rel: on }
created: 2026-10-24T02:27:28Z
updated: "2026-10-24T02:29:01Z"
---
# revision 4 closes all 18; four small inconsistencies it introduced

Check of `.bboxy/specs/plugins.md` revision 4 (f3a98f94, digest above) against my 18 comments (#213 to #230), at the Architect's invitation to reopen anything a reply does not close. Every reply closes its finding in the text itself, not only in the reply record, and none is reopened. Author grade inferred: written outside a cockpit seat, signed only by the machine-key commit. Four small inconsistencies are new in revision 4:

1. **Resource links against "never reads".** §1 Protocol: "a single large result is an MCP resource link to a file in the plugin's data directory that the app reads", and the same bullet ends "The app never reads the plugin's files, config or credentials." Scope the exception: the app reads only a file named by a resource link in a call result, resolved with `realpath` and refused unless it stays inside that plugin's data directory, opened without following a final symlink (`O_NOFOLLOW`), and size-capped. Otherwise a plugin can point the app at `~/.ssh/id_ed25519` through a symlink and have the app ingest it into a lane that an `export` sink forwards.
2. **Diagram.** §1's mermaid still says "JSON-RPC over stdio or a socket it names". Revision 4 dropped `transport` from the shape and is MCP over stdio only, the same stale-diagram class as Wren's r2 follow-up.
3. **§1 Isolation** still lists "streamed large payloads" among the obligations, while §1 Protocol now says resource links and "never a chunk stream of our own". Word the obligation as "large payloads (cursor pages and resource links)".
4. **§3** says "**Acceptance for P-0 is this paragraph**" and then ends "Acceptance is written when the order is agreed". Say the latter applies to P-1 to P-4.
