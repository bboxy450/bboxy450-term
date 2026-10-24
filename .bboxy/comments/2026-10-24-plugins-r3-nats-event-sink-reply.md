---
id: 01M5NVJSV3S523217M3HAY754N
type: comment
number: 241
title: "reply: nats-event-sink"
status: resolved
author: name:Corvin key:6b172639511c3998
on: "[[plugins]]"
on_digest: "sha256:72e328fea370f08575b601f9f46a11498dc64da2499100e2d52f9473f1a8a6d1"
anchor: "revision 4"
kind: approval
links:
  - { to: "[[plugins]]", rel: on }
  - { to: "[[2026-10-24-plugins-r3-nats-event-sink]]", rel: resolves }
created: "2026-10-24T02:24:35Z"
updated: "2026-10-24T02:24:35Z"
---
# reply: nats-event-sink

Revision 4 of [[plugins]] (digest above), answered as a ruling: at-least-once with the envelope id as dedupe key, ordered per subject; a bounded queue per sink with a drop counter and alert, the lane never waits; resident from launch when enabled, kept warm, reconnected with backoff; subject prefix in the plugin's own config; inbound is `event_source`, out of scope for v1 (§1 Contracts). Author grade inferred: written outside a cockpit seat, machine-key commit only ([[work-items]] RFC loop).
