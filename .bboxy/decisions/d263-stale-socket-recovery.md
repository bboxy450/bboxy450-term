---
id: 01M5PARDSNCAMX621S3FZBC8G3
type: decision
number: 263
title: "a stale mux.sock is recovered under the ownership lock after a connect probe refuses; a live one is never touched"
status: decided
decided: "2026-10-24"
operator: []
links:
  - { to: "[[mux-daemon]]", rel: decides }
  - { to: "[[d11-supervisor-death-survivable]]", rel: builds-on }
  - { to: "[[restart-keeps-state]]", rel: constrains }
created: "2026-10-24T06:49:47Z"
updated: "2026-10-24T06:49:47Z"
---
# Decision 263: stale control socket recovery

Ruled by the Architect on 2026-10-24 from Vesper's report: the operator's own app refused to start from the default run directory with `ControlSocketOwnershipUnknown: existing endpoint has no ownership lock`, then `supervisor exited during startup`, because a `mux.sock` from 2026-10-22 22:34 remained with no `mux.sock.lock` and nothing listening, most likely created before the `flock` existed (`bbt-supervisor` `transport.rs`, `socket_ownership`).

**Alternatives.** (a) Refuse, as today, and tell the operator to delete the file. (b) Take the ownership lock first, then probe the socket with `connect()`; `ECONNREFUSED` or `ENOENT` proves it dead, so unlink and bind; a successful connect proves a live owner, so release and refuse with the reason. (c) Unlink any socket that has no lock file, without probing.

**Choice.** (b).

**Reason.** An app that will not start over a file nobody is listening on fails the second mantra, a restart loses nothing ([[mantras]]), for no safety gained: a dead socket protects no one. (c) is unsafe against a live supervisor that predates the lock, which is exactly the file found. (b) is the ordering that cannot race: two starters contend for the lock, the loser sees a live owner on its probe. The lock is taken before the probe, never after, so there is no window between "dead" and "unlinked" in which a second starter binds.

**Rules.** In [[mux-daemon]], Stale control socket recovery: lock first, probe second, unlink and bind only on `ECONNREFUSED` or `ENOENT`, one log line with the stale mtime, refuse with the owner's pid on a live socket, never unlink without the lock. Seraph implements; a test leaves a dead socket without a lock and a live one with a lock and asserts recovery and refusal respectively.

Owner: Corvin Duskmantle <ARCHITECT>.
