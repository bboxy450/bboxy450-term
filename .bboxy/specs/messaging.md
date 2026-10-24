---
id: "2C06HC6R5PF2S85MPFMY6AZ563"
type: "spec"
number: 71
title: "Messaging — proposed architecture"
created: "2026-10-21T18:57:39Z"
updated: "2026-10-24T00:10:38Z"
links:
  - { to: "[[d23-recipient-batching]]", rel: decision }
---
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

**Conditional alternative:** Iroh passes the source-level license/configuration checks: local 1.3.0 is MIT OR Apache-2.0 and uses noq, n0's Quinn fork (`Iroh/Cargo.toml.orig:7`, `:44`; [[transport-and-identity]]); `Iroh/src/endpoint.rs:1993` supports a custom relay map and `:596` clears default address lookup services. The [project documents self-hosting the relay](https://docs.iroh.computer/iroh-services/relays/self-hosted). That enables an explicit own-infrastructure configuration; it is not a tested deployment here. Its [documented default](https://docs.rs/iroh/1.4.0/iroh/#connection-establishment) normally establishes via a home relay before upgrading direct. Reconsider it only if maintainer review, direct-first policy and a deployment with no n0 dependency pass the operator's requirements. The parallel [[transport-and-identity]] review is now read and agrees on Quinn; this alternative does not reopen that decision.

**Gap:** no self-hosted Iroh independence/traversal test or independent maintainer trust review was performed in B. The parallel review was checked against local license, noq dependency and configuration source; its broader funding, cadence and OAuth claims were not independently re-audited here. Keep the transport boundary independent of rooms/priority so a future change does not expose Quinn/noq types throughout the application.

## Service boundary

The persistent mux service owns seat IDs, harness-session mappings, message storage, signing and delivery attempts. The GUI subscribes to its events and renders them; closing the GUI must not terminate message ownership. This is a messaging requirement on the mux, not an implementation of the separate mux task.

Each seat's MCP/CLI connection receives a local capability bound to its seat ID. The service looks up the role and allowed destinations from terminal state; a caller cannot select an arbitrary signing identity or supply its own elevated role. Oracle control tools may change those grants only under the terminal's configured control permissions.

Discovery and session visibility default to the seat's own swarm. Explicit global-channel/DM grants remain possible; they do not grant another seat's screen or control. Oracle account-wide inspection requires its delegated control scope, as specified in [[mcp-control-plane]].

Transport workers handle connections outside the storage transaction. Bounded active workers can serve 1000 persisted seat records without one network socket, OS thread or in-memory queue per seat. **Gap:** throughput, storage capacity and delivery latency at that scale have not been measured.

## Identity: one account, separate seats

Use a root public key as the account ID. The email is account metadata with a separately represented verification state. The same root key identifies the account across machines; each device and each seat has its own key rather than sharing one seat secret. Google binding follows [[identity]] in the later cross-instance phase, outside the first prototype under [[vision]]; Apple remains deferred. Provider assertions supplement this key model.

Google binding uses the system browser, authorization code with S256 PKCE, and a loopback listener on a random available port for the desktop OAuth client ([Google desktop flow](https://developers.google.com/identity/protocols/oauth2/native-app)). Bind the verified ID token to the account public key through the key-bound nonce and a key-signed attestation, as described in [[transport-and-identity]]. Verify the provider signature, issuer, audience, expiry, nonce and email-verification claim; identify the provider account by issuer plus `sub`, with email as metadata ([Google OpenID Connect](https://developers.google.com/identity/openid-connect/openid-connect)). This additive binding does not replace account/device/seat signatures.

Apple remains deferred for the Developer ID distribution: [Apple DTS confirms that native Sign in with Apple is unsupported for Developer ID apps](https://developer.apple.com/forums/thread/768236). The parallel review's hosted-web-callback or Mac App Store alternatives remain future choices.

Grant chain (formats, custody and verification are owned by [[identity]]):

```text
account root key
  signs device grant {device key, scopes, epoch, expiry}
    signs seat grant {seat id, session key, swarm, role,
                      allowed destinations, muted, epoch, expiry}
      session key signs messages
```

The terminal stores the grants and issues role changes as new epochs. A receiver verifies the complete chain, destination scope and validity before accepting a role assertion. Root recovery and trusted device enrollment follow [[identity]]; neither a public email string nor a directory lookup proves account continuity. A directory can publish signed address/grant records without holding the signing authority.

Foreign root keys do not acquire local control by naming a seat Oracle. Elevated scheduling for a remote swarm requires a receiver-trusted controller/account mapping. Sender authentication, permission to send a message and permission to execute local work remain separate decisions.

Root custody, device enrollment, revocation and recovery decisions are specified in [[identity]]; its remaining implementation, partition and recovery gaps still apply.

**Oracle decision on the login nonce (Vesper Greywind, 2026-10-21):** nonce = SHA-256(canonical root public key bytes || 32 random bytes), the random part generated per login and kept locally until the ID token returns. Canonical key encoding = raw 32-byte Ed25519 public key, no prefix. The attestation record is {provider, subject, email, root_pubkey, nonce_random, id_token_hash, issued_at} signed by the root key; it is valid until the root key rotates or the operator revokes it, and the terminal re-binds yearly in the background. Google's random-nonce requirement and our key binding are both satisfied by this construction.

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
  "swarm_id": "swarm UUID",
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

Presence is a second `body` variant in the same signed envelope, published by the owning terminal to an authorized channel:

```json
{
  "type": "presence",
  "event": "state",
  "name": "Nyx Copperwing",
  "harness": "codex",
  "state": "active",
  "muted": false,
  "policy_epoch": 0
}
```

The outer account/device/seat fields and verified seat grant identify the participant; the owner terminal signs using that seat's granted key. `state` uses the shared active/idle/waiting/errored vocabulary. Receivers validate the signature, grant, destination and publication order before updating a shadow keyed by `(account, seat)`. The shadow shows only the name, harness and state its owner publishes, following [[ux-prototype]]. Presence supplies no pane, logs, scrollback or control capability, and a foreign Oracle still receives worker precedence under local trust policy.

Persist accepted presence under the normal inbox rules, then update the derived shadow view. Ordinary `event:"state"` presence is metadata and never enters the native/PTY harness-wake queue. Channel text messages keep their normal role/priority delivery. **Gap:** presence freshness/expiry, withdrawal and sequence continuity across key/grant changes need a protocol decision; a cached publication is last-reported state, not proof of current reachability.

The same presence body carries terminal-authored `event:"mute"` / `event:"unmute"` notices, with `muted:true` / `false` and `policy_epoch` equal to the authorizing seat-grant epoch. The outer seat identifies the affected seat; the owning terminal publishes the notice with that seat's signature and the replacement device-signed seat grant proving its mute policy. Receivers require agreement with the current authorized grant and reject stale policy epochs; a seat's arbitrary text or signature alone cannot change swarm membership policy. Only the authenticated operator or delegated Oracle control path can request a transition. Audit the actual initiating controller separately.

Commit the seat's mute flag, replacement grant and notice outbox entry as one policy transition. Each real transition creates one stable notice ID addressed to the whole swarm; repeated mute/unmute requests return the existing state rather than duplicate notices. Publish this terminal-generated control notice even though ordinary outbound swarm traffic is held. Swarm members display it in the stream; informational harness delivery follows their existing swarm/adapter policy, deduplicated by that same notice ID. Persistence/dispatch proves neither universal delivery nor model receipt.

The seat key signs a fixed-order tuple of every field except the signature, using a protocol-specific domain separator. Define the exact UTF-8 JSON tuple encoding and integer bounds in the wire implementation; transport route/TTL and receiver-local sequence are outside this signed body. Exchange the full signed grant chain when its digest is unknown, with bounded parsing and no priority elevation while verification is pending.

The service derives `role` and `priority_class` from the seat grant and signs them with the rest of the message. The receiver checks equality with the grant and applies its own trusted policy. A seat's signature alone proves authorship; the controller's grant supplies role authority. Deduplicate by author plus UUID, reject conflicting content, and persist the original bytes/signature.

## Priority without deleting messages

Policy: **Oracle > worker**, with Executor and Creativity sharing the worker class. This preserves the decided Oracle precedence without inventing an ordering between the other roles. FIFO uses receiver-local commit sequence within each class, independent of sender clocks. Per-seat serialization prevents concurrent injections into one harness.

Priority applies to pending outbound work, receiver dispatch and message badges. It does not move sidebar seat rows or interrupt an input write that already began. A native harness may impose its own admission order after submission; no adapter can claim to reorder already accepted input. Quinn stream priority is only an outbound optimization, not the receiver scheduler ([Quinn API](https://docs.rs/quinn/0.11.12/quinn/struct.SendStream.html#method.set_priority)).

Persist an accepted message before scheduling it. Lower priority delays eligibility; it never deletes the record. On full storage or revoked permission, return an explicit rejection/backpressure result before acknowledging acceptance. Sustained Oracle traffic can starve workers under strict precedence; do not promise bounded worker latency without a further scheduling decision.

**Oracle decision (Vesper Greywind, 2026-10-21):** Executor and Creativity are equal in the worker class. Only the local account's root key, and grants chained from it, can confer Oracle precedence on this receiver; a foreign Oracle is a worker here. Starvation bound: Oracle class gets a token bucket (default 10 messages per second per receiver, burst 30), overflow falls to worker FIFO. Numbers are defaults in settings, not protocol.

## Delivery, rooms and restart behavior

Use distinct evidence states: sender-stored, recipient-stored, native-queued, native-written, native-unknown, and application-acknowledged. The last state requires an explicit harness/application acknowledgment. A QUIC acknowledgment or socket write is insufficient.

Sender outbox and recipient inbox live in the mux's durable store. Retries reuse the original message identity; a recipient commit can be safely deduplicated and acknowledged again. Persist receipt and attempt records. Resume network retries after restart, but do not blindly repeat an ambiguous native input attempt. Keep that message visible with its unknown outcome.

**Swarm mute (operator decision, 2026-10-21):** persist `muted` on the seat record and terminal-issued grant. While muted, keep incoming swarm traffic in the inbox without scheduling native/PTY delivery, and keep outgoing swarm traffic in the outbox without publishing it. Return a held state/reason and retain original message IDs; never drop the records. Self status and authenticated direct operator control remain available so the operator can work one to one. Peer messages cannot claim that bypass. Mute is independent of harness activity, process lifecycle and sidebar row position.

Unmute commits its presence notice before releasing eligible held work under the current destination grants, membership, receiver priority and adapter guard. It does not erase a record whose old grant no longer permits sending; keep it visibly blocked rather than publishing under revoked authority. Already submitted native traffic cannot be recalled; ambiguous attempts are never automatically retried. A crash restores mute before scheduling, and no reconnect/focus operation implicitly unmutes a seat. See [[mcp-control-plane]] for Oracle-only transition tools.

A mute-only policy update is not a key revocation and does not invalidate the immutable signatures/IDs of held traffic by itself. Release still checks current authority; never rewrite an already published signed envelope under the same ID.

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

## Task messages (operator decision 2026-10-21, see [[work-items]])

A third `body` variant in the same signed envelope: `task`. Kinds: `delegate{task_id, path, role, brief_excerpt}` sent oracle-class to the responsible seat; `release{task_id}` to a seat that is no longer responsible; `report{task_id, summary, commit?, verify_result?}` from the responsible seat, routed to `delegated_by`; `accept{task_id}` / `reject{task_id, reason}` from the delegating seat. The mux mirrors every task message into the item's frontmatter (`assignee`, `status`, `updated`) before delivery evidence is recorded, so the file and the stream never disagree for longer than one write. Priority: `delegate`, `release`, `accept`, `reject` carry the sender's class; `report` carries worker class but bypasses the Oracle token bucket (it is the thing the Oracle is waiting for).

## Subagent addressing (operator decision 2026-10-21; ref `runs/ref-iterm-background-task.png`)

iTerm2 shows "1 background task running" under a seat; we know the subagent ids from OSC 7501 ([[subagent-detection]]), so a message names its target precisely. The envelope's destination gains an optional agent part: `to: {seat: <ulid>, agent?: <7501 id>}`. Rules:

| Target | Delivery |
|---|---|
| seat only | the harness's main agent, as today |
| seat + agent, harness can address subagents | through the adapter's subagent channel (none exists today for Claude Code or Codex: their sockets and queues reach the main session only; capability flag `subagent_addressable` on the harness record, false for both) |
| seat + agent, harness cannot | delivered to the main agent with a routing header `for subagent <id> "<title>"`, so the main loop relays or answers on its behalf; evidence state notes `relayed_via_main` |
| seat + agent, agent already ended | held with `agent_ended`, shown to the sender, never silently redirected |

The sender side knows the targets: every subagent row in the sidebar is a message target (row menu "message this subagent"), and a reply from a subagent carries `from: {seat, agent}` when the harness reports it (Claude's 7501 `id` on the report that produced it; otherwise `from` is the seat and the UI says "via main"). Receipts and the tick ladder are per target. The stream shows a target chip on the message row: main, or the subagent's title.

## Routing discipline (operator, 2026-10-21: the cohort's own rule, made native)

bboxy450-term encourages the routing pattern its own builders use:

- **DM by default.** A seat's messages go to one target (a seat, a subagent, the Oracle) unless they are a large chunk.
- **The swarm stream is for large chunks**: a task accepted, an epic done, a decision that changes others' work, presence and roll calls. Per-step progress is a DM to the delegating seat, or nothing (the task's status events already say it).
- **The Oracle summarises** to the stream when a chunk lands; workers do not narrate.

How the terminal encourages it, not just documents it:
1. Role system prompts (Executor, Creativity, custom worker-class roles) carry the rule in two lines, injected at launch.
2. `message_send` to a swarm stream from a worker-class seat requires `kind: chunk | decision | presence`; a plain progress note to the stream is rejected with a hint pointing at DM-to-delegator. Oracle-class is unrestricted.
3. Task reports go by `task_report` to `delegated_by`, never to the stream; the stream line is generated by the mux on `accepted`.
4. The messaging graph surfaces noise: a seat whose stream-message share exceeds a threshold shows a hint on its row for the Oracle.
5. The communication graph's may-send-to grants default workers to {delegator, own subagents, peers on an explicit edge}, stream only for the chunk kinds above.

## Attention budget per role (operator, 2026-10-21; Architect added the same day)

**Architect** designs the system and writes the specs: the high-level picture, zoomed out. **Oracle** decomposes specs into epics and epics into tasks, delegates, tracks state, verifies. **Executors** build. **Creativity** designs the experience. Expensive models go where the picture is drawn, not where it is tracked.

Model and effort follow what each role spends attention on:

| Role | Model / effort | Receives | Never receives |
|---|---|---|---|
| Architect | the most capable model (Claude Fable class), high effort, zoomed out; **oracle-class** | the operator's vision, research summaries, the Oracle's escalations of design-level gaps, invariant violations and drift reports | task traffic, progress, per-pass reports |
| Oracle | cheaper model, low reasoning effort; its job is tracking state, routing, verifying, deciding small things fast | every report (batched DMs), every chunk, every escalation | raw progress chatter |
| Executor | higher effort | its brief, its delegator's DMs, explicit-edge peer DMs | the swarm stream (subscribed only to its own task docs and to chunk/decision kinds) |
| Creativity | highest model and effort | the operator's taste notes and the Oracle's design-relevant decisions, batched | everything else |

**Delivery is per recipient, not per message** (decision [[d23-recipient-batching|#23]], 2026-10-22): one native wake per recipient per pump pass carries every message pending for it, so a broadcast costs one delivery per recipient and the pump's per-pass cap bounds recipients, not messages. The Architect's halts and decisions go out as `kind: decision` at oracle-class priority, so they dequeue ahead of worker traffic; Creativity stays worker-class (Oracle decision above). Defaults are per-role settings (model, effort, subscriptions) injected at launch; the operator overrides per seat. The goal is little to no messaging distraction for workers: wake events reach a worker only for its own task, a DM addressed to it, or an Oracle-class interrupt. Everything else waits in its inbox until the seat is idle, and the Oracle absorbs the rest.

## Subchannels and subgroups (operator, 2026-10-21) — superseded in detail by [[channel-tree]]

The swarm stream splits so attention stays narrow. On the bus they are just subjects ([[events]] Bus design):

- **Subchannels** under a swarm: `swarm/<id>/ch/<name>/…`, created by the Oracle or the operator: e.g. `render`, `mux`, `design`, `decisions`. A seat subscribes to the subchannels on its grant; the bare swarm stream carries only chunks.
- **Subgroups**: named sets of seats (`swarm/<id>/grp/<name>`) such as "executors", "render crew", "Oracle + Creativity". A message to a subgroup fans out as DMs to members, with one thread id. Membership lives in the swarm object and is edited by the Oracle (`graph_set_edge` grows a `group` form).
- Defaults: each worker is in the subgroup for its workstream and the subchannel for its layer; the Oracle is in all of them, reads summarised; Creativity is in `design` only.
- UI: subchannels as tabs in the stream dock, subgroups as chips; unread counts per subchannel; mute applies per subchannel.
- **Graph of channels**: swarm → subchannels → subgroups → seats as a tree, with weighted edges from the messaging graph (traffic decayed by recency) and the may-send-to grants as the allowed edges. `graph_read channels` returns it; the Graph tab draws it, so the Oracle sees where noise flows and reshapes it with `graph_set_edge`.
