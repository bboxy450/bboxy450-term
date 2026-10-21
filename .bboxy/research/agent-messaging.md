# Embedded agent messaging — source evidence

2026-10-21 · Nyx Copperwing <EXECUTOR> · assignment B.

[[vision]] · [[2026-10-21-agent-messaging-research]] · [[messaging]] · [[2026-10-21-agent-messaging-checks]]

## Source boundary

Read the installed **ICQ 0.20.2** package, not just its README. `ICQ/` below means `/Users/feral/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/icq-0.20.2/`; `.cargo_vcs_info.json:3` records commit `6524c3b689dc70ff9d379192f8a57a62c0c047ef`. Portable source: [ICQ 0.20.2 package](https://docs.rs/crate/icq/0.20.2/source/). These are version-specific observations, not a security audit or measured network performance.

`Quinn/` and `Iroh/` use the same registry parent with `quinn-0.11.12/` and `iroh-1.3.0/`. Their VCS revisions are `eaec0db4bcb698f76df736d89938743c88a2ad5f` and `0072d7d84b233f9e7185eb676f049beaf557ac03`. Online Iroh documentation currently resolves to 1.4.0; it is not substituted for the local 1.3.0 source.

## 1. Transport: Quinn plus ICQ coordination

| Pointer | Mechanism observed |
|---|---|
| `ICQ/Cargo.toml.orig:38` | Quinn `=0.11.12`, Tokio runtime, rustls/ring; `:40` rustls, `:42` Ed25519, `:46` `icq-mainline =8.0.1-icq.1`, `:47` STUN. ICQ does not use Iroh here. |
| `ICQ/src/transport.rs:187` | STUN queries the existing UDP socket, with a one-second timeout. `:199` wraps an abstract UDP socket in a Quinn endpoint. |
| `ICQ/src/runtime.rs:824` | Peer connection has a 34-second overall budget. Address lookup uses a two-second budget with a usable cached contact, otherwise the discovery deadline. `:907` divides four seconds among at most four advertised direct endpoints. |
| `ICQ/src/runtime.rs:920` | After direct failure, combine advertised and local helper hints, bounded to two, then call rendezvous establishment. No usable contact may also fall back to known helpers at `:874`. |
| `ICQ/src/rendezvous.rs:851` | Bound outbound coordination to 20 seconds and at most two helpers. Signed offer/acceptance at `:944` binds the expected parties and helper; `:982` attempts admitted candidates for four seconds, then `:1004` opens the encrypted relay fallback. |
| `ICQ/src/rendezvous.rs:754` | The other participant processes the accepted transcript and concurrently probes the sender at `:771`; this is the reverse half of simultaneous traversal. Failed probes reach relay receive at `:788`. |
| `ICQ/src/rendezvous/relay.rs:383` | Authenticate to helper, submit `RelayOpen`, then adapt framed packets into another Quinn endpoint. `:436` authenticates the **inner** QUIC connection to the intended peer. Helper TLS termination does not terminate this inner connection. |
| `ICQ/src/rendezvous/relay.rs:138` | Relay forwards bounded opaque packet frames. `:15` sets 1200-byte packets, eight circuits, 128 MiB combined byte allowance, 120-second circuit lifetime, and 1 MiB/s per direction. Both participants need live authenticated registrations (`:200`). |
| `ICQ/src/transport/websocket.rs:51` | Optional carrier uses `wss://…/icq` outside loopback tests; `:304` configures trusted TLS roots. `:432` selects the carrier for relay-only or synthetic destinations. UDP-blocked operation still requires a reachable supporting helper. |
| `ICQ/src/transport.rs:152` | Self-signed transport certificates are accepted independently of identity. `:279` signs a role-separated transcript of client key, server key, nonce and TLS exporter; `:544` verifies the expected peer's proof before application operations. The transcript role is client/server, not a cohort role. |

The relay is a bounded live circuit, not an offline message store. Direct success, a helper hint, and NAT classification are different observations. **Gap:** no cross-network traversal or relay success rate was measured in this assignment.

## 2. Identity and harness binding

| Pointer | Mechanism observed |
|---|---|
| `ICQ/src/identity.rs:49` | Identity contains an Ed25519 signing key. Address is its 32-byte verifying key (`:54`), rendered as lowercase hex (`:15`). |
| `ICQ/src/store.rs:205` | Reuse the secret associated with an exact session string; otherwise generate a new key with `OsRng` (`:234`). The session name hash is a lookup name, not deterministic key derivation. |
| `ICQ/src/store.rs:240` | Store the signing-key bytes directly in SQLite with session and attachment. `:42` sets database mode 0600, WAL and synchronous FULL; `ICQ/src/runtime.rs:1743` sets the home directory to 0700. This is filesystem protection, not key encryption. |
| `ICQ/src/runtime.rs:1329` | Check session/attachment namespace agreement, derive `session-<BLAKE3 prefix>`, verify supplied process metadata, then create/reuse its identity. |
| `ICQ/src/session.rs:67` | Check process ancestry and conflicts with inherited harness metadata. `:85` verifies candidate Codex sockets against the exact thread; `:116` resolves Claude's process-bound session record. |
| `ICQ/src/native.rs:186` | Read Claude's session record only after process/ownership checks; compare PID and process start, then obtain the recorded session UUID and messaging socket. `/tmp/cc-socks/` is an observed deployment location, not a reason to trust an arbitrary socket. |

There is no root-account/device delegation in the inspected identity and envelope formats (`identity.rs:49`, `wire.rs:51`). Sharing one private key establishes common key possession; it does not give separate device revocation. A directory locates keys and endpoints; it cannot itself prove device ownership. The proposed account/device/seat chain belongs in [[messaging]], not in the observations above.

**Gap:** ICQ source inspection does not establish verified email ownership, Apple/Google login binding, or a secure root-key transfer/recovery procedure. See the parallel [[transport-and-identity]] research when available.

## 3. Rooms and delivery guarantees

| Pointer | Mechanism observed |
|---|---|
| `ICQ/src/discovery.rs:17` | Public channel rendezvous is a namespaced SHA-1 DHT key. `:245` obtains up to 32 endpoint hints; `:335` announces a port. Contact publication at `:317` uses signed BEP44 mutable records. The DHT carries discovery data, not room history. |
| `ICQ/src/runtime.rs:1097` | Only public channels take the DHT discovery path; introduce to discovered endpoints, then authenticate a `Join`. `:625` requires the contacted identity to be subscribed. |
| `ICQ/src/runtime.rs:1565` | Private group context is a random 256-bit capability with a local alias. Group invitations are signed and expire after seven days (`groups.rs:20`, `:194`); Argon2id plus XChaCha20-Poly1305 protect the invitation (`groups.rs:111`, `:137`). |
| `ICQ/src/runtime.rs:651` | Room forwarding verifies the original author's signed envelope; local subscribed identities receive copies. Forwarding decrements TTL, bounded to 1–8. `:992` schedules known remote neighbors and stops when permits are exhausted. |
| `ICQ/src/store.rs:388` | Transactional inbox admission deduplicates `(owner, UUID)` and checks identical digest on repeats. Receipt is emitted after commit at `:467`. Capacity and admission failures are explicit. |
| `ICQ/src/runtime.rs:941` | DM success requires the expected ID/digest, recipient-stored state and a sequence. Sender outbox is persistent (`store.rs:577`); `runtime.rs:1489` retries the original envelope/UUID. This does not promise automatic offline delivery. |
| `ICQ/src/runtime.rs:1704` | Room send admits local copies, then schedules live forwarding with TTL 8. Result explicitly reports remote delivery unproven and offline delivery false (`:1730`). `:1696` says peer lists are incomplete membership. |

Public rooms are a discovered peer mesh; private groups use capability-based membership and known neighbors. Neither is a helper-hosted durable broker. Pairwise QUIC protects live traffic; protecting an invitation is not a separate encryption key for stored group history. No group roster/revocation protocol was found in these paths. **Gap:** offline catch-up, membership epochs and history retention for bboxy450-term remain product/protocol decisions.

## 4. Native delivery and the PTY boundary

| Pointer | Mechanism observed |
|---|---|
| `ICQ/src/harness.rs:429` | Claude receives newline-delimited JSON over an owned Unix socket, with stable message UUID and `priority: next`. Receipt means bytes written, not acceptance (`:455`). |
| `ICQ/src/harness.rs:524` | Codex uses a Unix WebSocket, initializes its protocol, checks exact loaded/healthy `thread/read`, then calls `thread/queue/add` with `clientUserMessageId` (`:604`). A checked response yields a queue ID; it does not prove model action. |
| `ICQ/extensions/bridge.mjs:42` | Pi/OMP bridge checks stable ID and identity, deduplicates dispatches, then calls `sendUserMessage` with follow-up delivery and prompt-template expansion disabled (`:55`). Host lifecycle binds session/generation (`extensions/host.mjs:38`). |
| `ICQ/src/harness.rs:80` | Readiness reports endpoint availability; Claude effective inbound policy and model receipt remain unverified. Launch settings add `crossSessionInbound: accept` (`launch.rs:200`), while stricter effective policies still matter. |
| `ICQ/src/runtime.rs:723` | Worker preflights availability, durably reserves the attempt, then submits. `harness.rs:615` preserves unknown outcomes after possible partial writes; automatic replay could duplicate input. |
| `ICQ/src/delivery.rs:20` | Render authenticated origin separately from peer body, preserving that peer content does not confer local execution authority. |

A terminal owning the PTY can attach seat/session lifecycle to its mux, inspect its screen and foreground process, receive hook state, and send input without discovering an external socket. Those are proposed integration advantages. Owning the PTY does **not** tell it that arbitrary text is a safe next prompt: a tool, shell, password prompt or interactive program may own input. Native queueing remains preferable; generic injection needs a current input-ready signal and must preserve unknown outcomes. **Gap:** adapter-specific ready-state evidence and race-free PTY handoff are not implemented or tested here.

## 5. Reuse boundary and cost

| Option | Existing surface | Work retained / limitation |
|---|---|---|
| A. Link `icq` | `lib.rs:1` exports transport, runtime, store, harness and other modules; `runtime.rs:86` builds the runtime. | Lowest transport rewrite, but lifecycle/storage/native policies remain coupled. Cohort attestation and queue priority require changes to runtime/store/wire, not a renderer wrapper. Manifest has unconditional settlement-chain dependencies (`Cargo.toml.orig:14`, `:49`) and no messaging-only feature boundary. Requires a pinned adaptation/fork or upstream changes. |
| B. Front daemon | Structured `Request` at `runtime.rs:52`; JSON RPC over `icq.sock` at `:1858` / `:1906`. | Least integration to display messages. A terminal can sort its own view and outbound calls, but unchanged daemon native dispatch still sorts inbox sequence (`store.rs:339`). `wire.rs:50` rejects unknown envelope fields. An accessible same-user daemon also exposes identity selection (`runtime.rs:1354`), so a UI-only permission gate is incomplete. |
| C. Rebuild on Quinn | Reuse ICQ's transport/authentication/receipt design as source evidence. | Own discovery, helper service, durable delivery, account/seat delegation, scheduler and adapters. Largest implementation and interoperability burden; provides the required role/priority and mux ownership boundaries. Chosen by [[vision]]. |
| C alternative: Iroh | Local `Iroh/Cargo.toml.orig:7` is MIT OR Apache-2.0, authored by the n0 team; `:44` depends on **noq**, not upstream Quinn. | Can remove much traversal/relay implementation, but still needs every application-level room, persistence and role policy. [Iroh docs](https://docs.rs/iroh/1.4.0/iroh/) describe relay-assisted establishment followed by direct upgrade; that default order differs from the requested direct-first ladder. [Project license](https://github.com/n0-computer/iroh#license) permits evaluation. Maintainer trust is not established by a license. |

Costs above are engineering scope comparisons, not measured days or build sizes. A/B cannot provide authenticated new role fields across unchanged ICQ peers: the signed v2 format has neither role nor priority and denies unknown fields. Any wrapper-in-text convention needs its own attestation and compatible receiver, without changing ICQ's internal scheduling. **Gap:** no compiled embedding proof or implementation effort measurement was performed.

## 6. Priority and roles

`ICQ/src/wire.rs:51` defines version/UUID/author/context/reply/timestamp/text/signature; `:62` domain-separates the signed serialization. It has no Oracle/Executor/Creativity field. `store.rs:339` selects by sequence. Claude's native `priority: next` is a harness hint shared by all these messages, not signed role ranking.

Quinn exposes sender stream priorities (`Quinn/src/send_stream.rs:220`); they order locally buffered stream data and cannot reorder a receiver's durable queue. See [the pinned API](https://docs.rs/quinn/0.11.12/quinn/struct.SendStream.html#method.set_priority). Application queueing and trusted role evidence are still required. The proposed signed message body and terminal-issued seat grant are in [[messaging]].

**Gap:** Oracle is required to outrank the other roles, but the brief does not order Executor against Creativity or define who may authorize a foreign account's Oracle priority. The spec keeps these distinct from authentication and makes its provisional scheduling rule explicit.

— Nyx Copperwing <EXECUTOR>
