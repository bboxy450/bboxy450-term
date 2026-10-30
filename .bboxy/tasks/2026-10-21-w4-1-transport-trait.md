---
id: "203JEPTGQNYW9DM0CG5FN5ANV9"
type: "task"
number: 108
title: "W4.1 — bbt-net: Transport trait, loopback, quinn endpoint with expected-key auth"
created: "2026-10-21T22:26:59Z"
updated: "2026-10-24T00:10:38Z"
---
# W4.1 — bbt-net: Transport trait, loopback, quinn endpoint with expected-key auth

Owner: Vesper Greywind <ORACLE>. Assignee: free Executor. Status: SUPERSEDED (8fd15659). Previous brief wording: OPEN. Spec: [[networking]] §1 Abstraction, §8 steps 1–2, [[messaging]] envelope and evidence states, [[identity]] device certificates. Plan [[plan]] W4.

New crate `crates/bbt-net` (the spec says `net`; the workspace prefix is `bbt-`). Allowed: `crates/bbt-net/**`, root `Cargo.toml` (add the member), `Cargo.lock`, `.bboxy/runs/w4-1-*.log`.

1. `Transport` trait exactly as §1: `connect(peer_key) -> Connection`, `listen`, `subscribe(topic) -> Stream`, `publish(topic, bytes)`, `presence`; `TopicId`, `PathEvidence` types; the messaging envelope passes through as opaque signed bytes.
2. `LoopbackTransport`: in-process, two "devices" exchange a signed envelope and a topic broadcast; evidence states reported as §1 defines.
3. `QuinnTransport` skeleton: quinn 0.11.12 endpoint, self-signed rcgen cert, ALPN `bboxy450-term/1`, expected-key authentication via the TLS exporter signed by the device leaf key both directions (icq `transport.rs:112-134` pattern, cited in [[networking]]), wrong expected key rejected before any stream opens. Local only, no punch, no relay, no DHT in this brief.
4. Tests: `cargo test -p bbt-net loopback::` and `cargo test -p bbt-net quinn::` green; the wrong-key rejection test; a two-endpoint loopback QUIC round trip of one envelope with evidence.

Verify: `cargo test -p bbt-net --locked` green; `cargo build --workspace --locked` green. Report as usual; name every fact from the spec not exercised.
