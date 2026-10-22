# W2.4 — local messaging: envelopes, the channel tree, priority, harness delivery

Owner: Vesper Greywind <ORACLE>. Assignee: Rook Ashwhisker <EXECUTOR>. Epic [[2026-10-22-c-cockpit]]. Specs: [[messaging]] (envelope, priority, delivery and evidence states, mute, harness delivery), [[channel-tree]] (namespace, patterns, sinks, symlinks, grants, on the bus), [[events]]. **Local only**: one machine, one account, no network transport (W4.1 is deferred).

Allowed: `crates/bbt-mux/**`, `Cargo.lock`, `.bboxy/runs/w2-4-*.log`.

1. **Envelope:** the signed v1 envelope from [[messaging]], signed by the seat session key (W2.2 identity material); thread id, `reply_to`, priority, kind, and attachments as references to tasks and artifacts (wikilinks).
2. **Channel tree:** rows in the swarm object `(path, kind, target, filter, rate, paused, grants)`; resolve `*` and `**`; dedupe per event id; cycle-safe; record the arrival path; tree edits are bus events.
3. **Delivery:** persist before scheduling; Oracle > worker class with the token bucket (10/s, burst 30); FIFO per class; per-seat serialization; the evidence states sender-stored → recipient-stored → native-queued, native-written or native-unknown → application-acknowledged. Delivery goes through the W2.3 native paths (Claude socket, Codex queue) or the guarded PTY fallback; it never retries an ambiguous native write.
4. **Mute:** held in, held out, release on unmute under the current grants, survives restart.
5. **Client API** on the seat socket and logic client, for the app's StreamDock and composer (lab `stream.html` and `compose.html`): send, list a thread or channel with cursors, ls a tree path, subscribe/unsubscribe/link, mute, and receipts.

Acceptance tests: a DM between two fixture seats delivered native-written; `#bboxy/**` fan-out to nested sinks once each; a folder-link bridge; filter plus rate batching on an Oracle sink; Oracle precedence and token-bucket overflow; mute holds and unmute releases after a logic restart; an ambiguous PTY write stays native-unknown; grant refusal names the folder and the missing right.
Verify: `cargo test -p bbt-mux --locked`, clippy `-D warnings`, workspace build. Same handoff.
