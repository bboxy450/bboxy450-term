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
- **Semantic launcher matching** (design system v3, tier 5): local embeddings via icq's pocket-encoder model, embed-once rows, RRF merge under lexical hits. Not v0.1: lexical tiers ship first; the encoder is a bundled model and a dependency decision for later.
- Linux.
