---
id: "2HNFMPQH4KGNZXZEJSKHNQ1RX6"
type: "note"
number: 58
title: "Roadmap"
created: "2026-10-21T18:53:32Z"
updated: "2026-10-24T00:10:38Z"
---
# Roadmap

Back to [[vision]].

## v0.1
Everything in [[vision]] not marked Future.

## Later
- Cloud harnesses, mosh-style reconnect, user-account-bound. See [[cloud-and-mosh]] once research lands.
- Switch a session between harnesses: move Codex logs to Claude and resume, and back.
- Usage limits for Claude and Codex accounts. Study `tachyon` (github.com/Gonzih/tachyon) for the approach. Operator note: Gonzih is bboxy450.
- MCP tool for an agent to request operator attention.
- **Launchables beyond harnesses** (operator via the lab, 2026-10-21): the ⌘T launcher is data-driven, so browser tabs, VNC sessions and ssh sessions become seat types later. Design the seat record so `harness_kind` is an open enum.
- **Semantic lookup** (operator 2026-10-21, promoted from later): the in-memory index gets an embedding index via icq's pocket-encoder model with sqlite-vec persistence ([[storage]]); exposed as the `find` MCP tool and the launcher's tier 5. Lands with W2.2's index; the launcher wiring follows in W3.2c.
- Linux.
