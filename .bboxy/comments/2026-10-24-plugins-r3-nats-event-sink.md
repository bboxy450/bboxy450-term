---
id: 01M5NVBXDZKCDZGEWG7X43AX06
number: 225
type: comment
title: "what a NATS sink still needs: delivery, backpressure, lifecycle, inbound"
status: resolved
author: name:Seraph key:eec764b7d2f4b9b8
on: "[[plugins]]"
on_digest: "sha256:a3b7a2b411f819d0fbaa532d587847fe91891bf9a4b9b14e0c2419c9c65d9212"
anchor: "§1 Contracts · event_sink"
kind: question
links:
  - { to: "[[plugins]]", rel: on }
created: 2026-10-24T02:20:49Z
updated: "2026-10-24T02:24:35Z"
---
# what a NATS sink still needs: delivery, backpressure, lifecycle, inbound

Review of `.bboxy/specs/plugins.md` revision 3 at main 9c2022a7 (digest pinned above), fresh eyes from an idle Executor at the operator's request ("review plugin draft and send message to you with feedback!"). Author grade inferred: written outside a cockpit seat, signed only by the machine-key commit. Not repeated here: the 21 comments of 2026-10-23, all resolved.

`event_sink` is "deliver envelopes to an external bus such as NATS; acknowledge". Before a NATS plugin can be written:

- delivery semantics: at-least-once with the envelope id as the dedupe key, or at-most-once? Ordered per subject?
- backpressure: a slow or dead sink must never make the events lane wait ([[ui-never-waits]]). A bounded queue per sink, with a drop counter in the registry?
- lifecycle: a sink is resident, but §1 Shape says "launch spawns nothing until a contract is first called". Is a sink started at launch, kept warm, reconnected on crash?
- mapping: how app subjects become NATS subjects (a prefix in the plugin's config, or in the contract)?
- inbound: the operator also named "external sessions, external data sources" and an external messaging system. Bus to app has no contract. Is that `session`, a new `event_source`, or out of scope for v1?
