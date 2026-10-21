# Messaging — proposed architecture

2026-10-21 · Nyx Copperwing <EXECUTOR> · assignment B · proposal for Oracle review.

[[vision]] · [[agent-messaging]] · [[2026-10-21-agent-messaging-research]] · [[2026-10-21-agent-messaging-checks]]

## Decision and cost

Choose **option C: rebuild the messaging service on Quinn**, following the operator's decision in [[vision]]. ICQ 0.20.2 supplies design evidence for traversal, expected-key authentication, durable receipts and harness adapters; it is not the production runtime dependency. The comparison and source pointers are in [[agent-messaging]].

The cost is ownership of six boundaries: traversal/helper operations; account/device/seat keys; authenticated room membership; durable inbox/outbox and catch-up; role scheduling/permission enforcement; harness adapters and their changing protocols. This is more work than either embedding ICQ or fronting its daemon. No duration estimate or working implementation is claimed.

Linking ICQ preserves its storage and scheduler coupling and unconditional settlement dependencies. Fronting its daemon can sort a UI but cannot add trusted fields to its signed v2 envelope or replace its internal native-delivery order. Its 64-identity cap also conflicts with the 1000-seat target (`ICQ/src/store.rs:233`). These are concrete reasons for the chosen boundary, beyond preference for a rewrite.

## Transport: decided baseline

Use Quinn with our own bounded direct dial, coordinated simultaneous UDP punch, then an end-to-end encrypted relay fallback. Model the ladder on `ICQ/src/runtime.rs:824`, `src/rendezvous.rs:851` and `src/rendezvous/relay.rs:383`, whose mechanisms are recorded in [[agent-messaging]]. A reachable common helper supplies signed rendezvous observations and carries opaque inner QUIC packets when direct traversal fails. For UDP-blocked networks, support an authenticated TLS/WebSocket helper carrier; merely having a relay address does not establish reachability.

The network connection authenticates the remote mux device's delegated key, bound to the actual TLS session; each message separately proves its author seat and role grant. Share bounded connections between device pairs across seats instead of forcing one network connection per seat. Local seats take the same admission/signature/queue path without a network round trip. Use a new ALPN and discovery namespace: this design does not claim ICQ wire interoperability.

Own the contact/discovery records and helper operation as part of this service. Public channels may use namespaced DHT discovery with signed contact records; never publish private group capabilities as public discovery names. A helper carries no application execution authority and is not an offline mailbox. Persist messages through path changes, relay expiry and reconnect, then retry the same message IDs. Port ICQ's authentication and bounding properties, not its eight-circuit or 64-identity limits unchanged. Export per-stage outcomes and backpressure so capacity exhaustion cannot masquerade as successful delivery.

**Conditional alternative:** Iroh passes the source-level license/configuration checks: local 1.3.0 is MIT OR Apache-2.0 and uses noq (`Iroh/Cargo.toml.orig:7`, `:44`); `Iroh/src/endpoint.rs:1993` supports a custom relay map and `:596` clears default address lookup services. The [project documents self-hosting the relay](https://docs.iroh.computer/iroh-services/relays/self-hosted). That enables an explicit own-infrastructure configuration; it is not a tested deployment here. Its [documented default](https://docs.rs/iroh/1.4.0/iroh/#connection-establishment) normally establishes via a home relay before upgrading direct. Reconsider it only if maintainer review, direct-first policy and a deployment with no n0 dependency pass the operator's requirements. The parallel [[transport-and-identity]] review is now read and agrees on Quinn; this alternative does not reopen that decision.

**Gap:** no self-hosted Iroh independence/traversal test or independent maintainer trust review was performed in B. The parallel review was checked against local license, noq dependency and configuration source; its broader funding, cadence and OAuth claims were not independently re-audited here. Keep the transport boundary independent of rooms/priority so a future change does not expose Quinn/noq types throughout the application.

## Service boundary

The persistent mux service owns seat IDs, harness-session mappings, message storage, signing and delivery attempts. The GUI subscribes to its events and renders them; closing the GUI must not terminate message ownership. This is a messaging requirement on the mux, not an implementation of the separate mux task.

Each seat's MCP/CLI connection receives a local capability bound to its seat ID. The service looks up the role and allowed destinations from terminal state; a caller cannot select an arbitrary signing identity or supply its own elevated role. Oracle control tools may change those grants only under the terminal's configured control permissions.

Transport workers handle connections outside the storage transaction. Bounded active workers can serve 1000 persisted seat records without one network socket, OS thread or in-memory queue per seat. **Gap:** throughput, storage capacity and delivery latency at that scale have not been measured.

## Identity: one account, separate seats

Use a root public key as the account ID. The email is account metadata with a separately represented verification state. The same root key identifies the account across machines; each device and each seat has its own key rather than sharing one seat secret. Google sign-in is in v0.1 and Apple is deferred, as now decided in [[vision]]; provider assertions supplement this key model.

Proposed chain:

```text
account root key
  signs device grant {device key, scopes, epoch, expiry}
    signs seat grant {seat id, session key, cohort, role,
                      allowed destinations, epoch, expiry}
      session key signs messages
```

The terminal stores the grants and issues role changes as new epochs. A receiver verifies the complete chain, destination scope and validity before accepting a role assertion. Root recovery and trusted device enrollment must be specified before claiming account continuity; neither a public email string nor a directory lookup proves it. A directory can publish signed address/grant records without holding the signing authority.

Foreign root keys do not acquire local control by naming a seat Oracle. Elevated scheduling for a remote cohort requires a receiver-trusted controller/account mapping. Sender authentication, permission to send a message and permission to execute local work remain separate decisions.

**Gap:** root custody, device enrollment/revocation propagation and recovery are unresolved. Apple/Google identity binding belongs to [[transport-and-identity]]; this spec does not invent a login or key-escrow scheme.

## Signed envelope sketch

This is a new protocol, not fields silently appended to ICQ's strict v2 schema. Example symbolic values describe the schema; they are not sample live keys.

```json
{
  "version": 1,
  "id": "message UUID",
  "account": "root public key",
  "device": "device public key",
  "seat": "stable seat UUID",
  "author": "seat session public key",
  "cohort": "cohort UUID",
  "context": {"kind": "dm|channel|group", "id": "destination", "epoch": 1},
  "role": "executor",
  "role_grant_id": "digest of terminal-issued grant",
  "priority_class": "worker",
  "sender_sequence": 42,
  "reply_to": null,
  "timestamp": 0,
  "body": {"type": "text", "text": "message"},
  "signature": "seat signature"
}
```

The seat key signs a fixed-order tuple of every field except the signature, using a protocol-specific domain separator. Define the exact UTF-8 JSON tuple encoding and integer bounds in the wire implementation; transport route/TTL and receiver-local sequence are outside this signed body. Exchange the full signed grant chain when its digest is unknown, with bounded parsing and no priority elevation while verification is pending.

The service derives `role` and `priority_class` from the seat grant and signs them with the rest of the message. The receiver checks equality with the grant and applies its own trusted policy. A seat's signature alone proves authorship; the controller's grant supplies role authority. Deduplicate by author plus UUID, reject conflicting content, and persist the original bytes/signature.

## Priority without deleting messages

Provisional policy: **Oracle > worker**, with Executor and Creativity sharing the worker class. This preserves the decided Oracle precedence without inventing an ordering between the other roles. FIFO uses receiver-local commit sequence within each class, independent of sender clocks. Per-seat serialization prevents concurrent injections into one harness.

Priority applies to pending outbound work, receiver dispatch and message badges. It does not move sidebar seat rows or interrupt an input write that already began. A native harness may impose its own admission order after submission; no adapter can claim to reorder already accepted input. Quinn stream priority is only an outbound optimization, not the receiver scheduler ([Quinn API](https://docs.rs/quinn/0.11.12/quinn/struct.SendStream.html#method.set_priority)).

Persist an accepted message before scheduling it. Lower priority delays eligibility; it never deletes the record. On full storage or revoked permission, return an explicit rejection/backpressure result before acknowledging acceptance. Sustained Oracle traffic can starve workers under strict precedence; do not promise bounded worker latency without a further scheduling decision.

**Gap:** confirm Executor/Creativity equality, foreign-controller trust policy, and any Oracle admission quotas needed to bound starvation. These are policies, not properties supplied by QUIC.

**Oracle decision (Vesper Greywind, 2026-10-21):** Executor and Creativity are equal in the worker class. Only the local account's root key, and grants chained from it, can confer Oracle precedence on this receiver; a foreign Oracle is a worker here. Starvation bound: Oracle class gets a token bucket (default 10 messages per second per receiver, burst 30), overflow falls to worker FIFO. Numbers are defaults in settings, not protocol.

## Delivery, rooms and restart behavior

Use distinct evidence states: sender-stored, recipient-stored, native-queued, native-written, native-unknown, and application-acknowledged. The last state requires an explicit harness/application acknowledgment. A QUIC acknowledgment or socket write is insufficient.

Sender outbox and recipient inbox live in the mux's durable store. Retries reuse the original message identity; a recipient commit can be safely deduplicated and acknowledged again. Persist receipt and attempt records. Resume network retries after restart, but do not blindly repeat an ambiguous native input attempt. Keep that message visible with its unknown outcome.

Global public channel names need a shared namespace and discovery protocol; their peer list is not complete membership. Maintain an explicit pending dissemination record for publications, and expose known-recipient receipts rather than claiming all global subscribers received them. For private groups and group DMs, use a signed membership epoch/recipient set and retain pending work for those known recipients. A capability-protected invitation alone does not provide membership revocation or offline history synchronization.

**Gap:** public-channel catch-up/replication service, private membership epochs, history encryption and retention limits need a protocol decision. Until then, only accepted local durability and known-recipient acknowledgment are specified; global offline delivery is unproven.

## Harness delivery and PTY fallback

Prefer the already evidenced native touch points: Claude's owned Unix messaging socket; Codex's exact-thread queue API; Pi/OMP's session/generation-bound extension. Keep each behind an adapter and record its actual evidence. Preserve effective inbound policy and session focus controls. A verified endpoint is not proof the model consumed a message (`ICQ/src/harness.rs:80`).

For a generic PTY, require a current ready-state signal from the harness adapter/hook and match the seat's foreground process and session generation immediately before writing. No ready-state evidence means keep the message queued. Output silence alone is insufficient. Serialize operator keystrokes and injection through the mux's input arbiter; do not type peer text into an unrelated shell/tool/password prompt. Record a possible partial write as native-unknown rather than retrying Enter.

**Gap:** generic PTY readiness and the input race need adapter-specific tests. The available ICQ source proves its native mechanisms, not a universal safe PTY injection method. See the separate harness-touchpoints work.

## Permission and verification gaps

**Gap:** terminal-enforced permissions need a same-user-process threat model. A same-UID seat with access to the broker's raw signing keys or unrestricted RPC can bypass a UI-only rule; ICQ's filesystem ownership checks are not per-seat isolation. Decide credential custody and any OS isolation before describing these ACLs as an adversarial sandbox.

Proportionate implementation evidence should cover: a forged role/unknown issuer cannot gain Oracle priority; destination grants reject an unauthorized send; duplicate retries yield one inbox record; a restart preserves pending lower-priority records; native-unknown is not automatically resubmitted; injection waits through an interactive foreground tool; and 1000-seat queue/latency measurements. These are future acceptance checks, not tests run by this research assignment.

— Nyx Copperwing <EXECUTOR>
