---
id: "38TCRNC4PG2FKGRAZ66M0A6PWG"
type: "spec"
number: 107
title: "Networking — transport, discovery, failover, pubsub, remote hosts"
created: "2026-10-21T22:26:59Z"
updated: "2026-10-24T00:10:38Z"
---
# Networking — transport, discovery, failover, pubsub, remote hosts

2026-10-21 · spec-drafting subagent for Vesper Greywind <ORACLE> · for Oracle review.

[[vision]] · [[messaging]] · [[mux-daemon]] · [[identity]] · [[transport-and-identity]] · [[cloud-and-mosh]] · [[plan]]

Status: DECIDED where marked, ASSUMED where marked, **Gap:** where open. Facts fetched 2026-10-21 (`date` → Wed Oct 21 15:19 PDT 2026). `ICQ/` = `/Users/feral/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/icq-0.20.2/src/`.

## Decision

**Transport = upstream quinn** (DECIDED by the Oracle 2026-10-21, after the operator deferred the call). The operator's iroh reversal earlier the same day is recorded in [[vision]]; the Oracle's call is quinn plus our own punch ladder, relay (UDP and WebSocket/TLS carrier) and mainline-DHT discovery, modelled line by line on icq 0.20.2. iroh is the **phase-3 candidate** (§9): re-evaluated when global messaging starts, against written criteria, not by mood.

Grounds, short: icq already proves the quinn + STUN + BEP44 + encrypted-relay + WebSocket-carrier shape in ~5k lines on this machine; quinn 0.11.12 is independently maintained (`source: https://crates.io/api/v1/crates/quinn`, 2026-09-14, MIT OR Apache-2.0); iroh 1.4.1 would bring `noq`, a second QUIC stack owned by one company, and a relay-first connection order the opposite of ours. Everything above `connect(key) -> Connection` is identical either way, so the call stays cheap to reverse.

Pinned dependency set for the transport crate (versions verified on crates.io 2026-10-21): `quinn 0.11.12` / `quinn-proto 0.11.19` (2026-09-30), `rustls 0.23.46` (2026-10-14, ring provider), `rcgen 0.13` (self-signed transport certs, as `ICQ/Cargo.toml:126`), `ed25519-dalek 2`, `stunclient 0.4.2` (2025-12-09), `mainline 8.0.1` (2026-09-30, MIT; icq pins a patched `icq-mainline 8.0.1-icq.1`, `ICQ/Cargo.toml:111-113` — **Gap:** what the patch changes, read before choosing upstream or patched), `tokio-tungstenite 0.30.0` (2026-07-11, `rustls-tls-webpki-roots`).

## 1. Abstraction

One crate, `net`, exports the trait. Nothing above it sees a `quinn::` type. [[messaging]] already requires this boundary ("keep the transport boundary independent of rooms/priority").

```rust
pub trait Transport: Send + Sync {
    /// Dial a device by its Ed25519 device key. `hints` are cached Contacts,
    /// pairing-ticket endpoints or a helper list; the ladder in §3 runs inside.
    async fn connect(&self, peer: DeviceKey, hints: Hints) -> Result<Connection, DialError>;
    /// Incoming authenticated connections. The remote device key is proven
    /// before the first application frame is readable.
    fn listen(&self) -> impl Stream<Item = Connection>;
    /// Join a topic (§4). Yields signed envelopes as received, unordered.
    async fn subscribe(&self, topic: TopicId, membership: MembershipProof) -> Result<Subscription>;
    /// Hand bytes to the topic layer. Returns the local dissemination record id,
    /// never a delivery claim.
    async fn publish(&self, topic: TopicId, bytes: Bytes) -> Result<PublishRecord>;
    /// Current path evidence for every live peer plus own observed addresses.
    fn presence(&self) -> PresenceSnapshot;
}

pub struct Connection {
    pub peer: DeviceKey,
    pub path: PathEvidence,            // §3: Direct | Punched | RelayUdp | RelayWs, rtt, since
    // open_bi / open_uni / accept_bi / accept_uni / close, bounded framing from mux-daemon.md
}
```

**Authentication binding (DECIDED).** TLS 1.3 inside QUIC with a per-process self-signed certificate accepted blind (`ICQ/src/transport.rs:152`); identity is proven separately by an Ed25519 signature over a role-separated transcript `(client key, server key, nonce, TLS exporter)` (`ICQ/src/transport.rs:279-298`), checked against the *expected* device key before any application stream is served (`ICQ/src/transport.rs:544`). The device key is the raw Ed25519 key certified in [[identity]]'s device certificate; the connection carries the `DeviceGrant` digest and exchanges the full chain when the peer does not know it ([[messaging]] "Exchange the full signed grant chain when its digest is unknown"). ALPN `bboxy450-term/1`; no icq wire interoperability is claimed (icq is `icq/2`, `ICQ/src/transport.rs:150`).

**Envelope and evidence sit on it unchanged.** [[messaging]]'s signed envelope is the payload of a length-framed stream message; the transport never parses it. Evidence states map as: a QUIC stream write completed = nothing (not even `sender-stored`, which is the outbox commit); a signed receipt frame from the peer's mux = `recipient-stored`; everything after that (`native-queued`, `native-written`, `native-unknown`, `application-acknowledged`) is produced by the receiving mux and travels back as a receipt envelope. Transport acks never upgrade an evidence state. Connections are shared per device pair across seats (bounded, as [[messaging]] decides), with one bi-stream per logical channel (messages, receipts, mux state stream per seat from [[mux-daemon]] "Logic ↔ GUI state-stream protocol", control). Stream priority is a send-side hint only.

**Loopback implementation (v0.1).** `LoopbackTransport` serves every seat on one machine: `connect(own device key)` returns a `Connection` backed by an in-process duplex channel inside mux logic; `subscribe`/`publish` are a per-topic broadcast channel; `presence` reports `Path::Local`. Same admission, signature and queue path as a network peer ([[messaging]] "Local seats take the same admission/signature/queue path without a network round trip"), no TLS, no sockets. A second GUI or script on the same machine reaches seats through the seat Unix sockets of [[mux-daemon]], not through this transport. The `QuinnTransport` replaces loopback for any peer whose device key is not ours; a mux may hold both.

## 2. Discovery

Three records, one DHT. The DHT is BitTorrent mainline, BEP44 mutable items, exactly icq's design.

**DeviceRecord (the signed truth).** Published by each device under its own Ed25519 device key as the BEP44 mutable key (BEP44 requires an Ed25519 key; `ICQ/src/discovery.rs:321-326` builds `MutableItem::new(signing_key, value, seq, Some(SALT))`). Salt `bboxy450-term/addr/v1` (icq: `icq-address-v3`, `:16`). Value = JCS JSON per [[identity]] "Signed formats":

```text
DeviceRecord { version, root_pubkey, device_id, device_pubkey, device_grant_id,
               endpoints[≤4], helpers[≤2]{key, endpoint}, websocket_url?, timestamp }
```

BEP44 signs `salt+seq+value` with the device key (`ICQ/src/discovery.rs:349-355`), so one signature covers the record; the inner contact signature icq also carries (`ICQ/src/wire.rs:170-173`, domain `icq-contact-v3`) is kept so a cached record stays verifiable off-DHT. Bounds copied: value ≤ 1000 bytes (`:319`, `wire.rs:214-216` ≤ 996 encoded), ≤ 4 endpoints and ≤ 2 helpers all with non-zero port, non-unspecified, non-multicast (`wire.rs:199-212`), endpoints filtered to globally routable addresses unless `allow_local` (`:310-317`, `global()` at `:22-47`). `seq = max(now_unix, previous + 1)` persisted per owner (`ICQ/src/store.rs:892-904`). **TTL:** a record is accepted when `timestamp ≤ now + 30` and `now − timestamp ≤ 180` (`wire.rs:219-223`); republished every 60 s (`ICQ/src/rendezvous.rs:449`). Resolution picks the highest-`seq` item that passes key/salt/seq/size/signature checks (`:49-82`), with an 8 s lookup deadline (`:15`) and a 2 s budget when a usable cached contact exists (`ICQ/src/runtime.rs:856-860`). Resolvers verify the record's `device_grant_id` against a chain to the pinned root before dialling; a record proves key possession and current addresses, not authority (identity.md: "A directory can publish signed address/grant records without holding the signing authority").

**AccountIndex (how a device finds the operator's other devices).** The account root is an OpenPGP primary that stays offline ([[identity]] "never bring the account primary online for every launch"), so it cannot sign a 60-second heartbeat. Two hints instead, both unsigned, both verified by the DeviceRecord they lead to: (a) `announce_peer` under infohash `SHA-1("bboxy450-term/account/v1\0" || root_fingerprint_hex)`, the same call icq uses for channels (`ICQ/src/discovery.rs:335-346`; lookup `get_peers` capped at 32 addresses, `:245-261`), each device announcing its QUIC port; (b) every DeviceRecord carries `root_pubkey`, and the `AccountHead` the devices already exchange on contact lists `active_devices` with their keys, so finding one sibling finds the rest by key. A newly enrolled device starts from the pairing ticket's endpoint hints and the introducer's key ([[identity]] enrollment step 2), never from the DHT alone. Only a DeviceRecord whose chain verifies to *this* pinned root fingerprint is treated as "mine".

**ChannelIndex (global channels, phase 3).** `announce_peer` under `SHA-1("bboxy450-term/channel/v1\0" || name)` (icq `icq-channel-v2`, `ICQ/src/discovery.rs:17-21`); peers ≤ 32; the peer list is a sample, never membership. Private groups and swarm topics are **never** published; membership there comes from grants (§4) and contact records exchanged inside authenticated connections.

**Alternatives, kept as rows not code.** LAN: private-interface candidates admitted only when both sides observe the same public IP (`ICQ/src/rendezvous/lan.rs:61-75`, `rendezvous.rs:1084-1108`); no mDNS. Self-hosted DNS: a `_bboxy._udp.<device-id>.<domain>` TXT publication of the same DeviceRecord is a phase-3 option alongside `iroh-dns-server` (§9); not built before. Helper catalogue: helpers announce under `SHA-1("bboxy450-term/helper/v1\0")` and publish a signed advertisement with their `websocket_url` (icq `TOPIC` `ICQ/src/rendezvous/protocol.rs:13`, worker `rendezvous.rs:421-470`: tick 5 s, publish 60 s, discover 15 s).

**Third-party dependency named:** the mainline bootstrap routers (public BitTorrent nodes). Nothing from n0. A device with no DHT reach still works from cached contacts, pairing tickets and pinned helpers.

## 3. Failover ladder

Budgets are icq's, not invented. Total dial budget 34 s (`ICQ/src/runtime.rs:838`).

| Stage | Mechanism | Budget | Source |
|---|---|---|---|
| 0 lookup | Cached DeviceRecord if it verifies and has an allowed endpoint or helper; refresh in parallel. | 2 s with cache, 8 s without | `runtime.rs:839-867`, `discovery.rs:15` |
| 1 direct | Dial ≤ 4 advertised endpoints, time split evenly across the remaining ones, expected-key auth inside each attempt. Cloud VMs with a public IP end here. | 4 s shared | `runtime.rs:897-915` |
| 2 hole punch | For ≤ 2 helpers: register (lease 30 s, `protocol.rs:11`), send a signed `Offer` with LAN candidates, receive the helper-signed `Accepted` carrying both observed addresses, merge observed + admitted LAN candidates (≤ 2), both sides probe **simultaneously** (sender `rendezvous.rs:982-1000`, recipient `:771-788`). | per helper round 10 s (`protocol.rs:12`); registration 6 s; acceptance 2 s; probes 4 s | `rendezvous.rs:851-1010` |
| 3 relay (UDP) | Same helper opens a bounded opaque circuit; the *inner* QUIC connection is dialled through a synthetic address and authenticated to the expected peer, so the helper carries ciphertext only. | remainder of the round | `relay.rs:383, 420-454`; limits `relay.rs:15-19`: 1200 B datagrams, 8 circuits, 128 MiB, 120 s, 1 MiB/s per direction |
| 4 relay over WebSocket/TLS | When UDP is blocked or `relay_only`, the QUIC datagrams ride a `wss://` carrier to the helper (ws allowed only on loopback tests). TLS roots = webpki; URL ≤ 200 bytes. Selected per destination at the socket layer, so QUIC above is unchanged. | same round; stage `websocket_fallback` | `transport/websocket.rs:1-2, 30-31, 51-56, 301-315, 432-435`; `rendezvous.rs:701` |

Changes from icq for our use: relay circuit lifetime is **renewable** (a mux link lasts hours; icq's 120 s is for a message exchange), byte and rate caps are per account per helper and configurable, and QUIC idle timeout is minutes, not 15 s ([[cloud-and-mosh]] §4 item 6). Observed address comes from classic STUN on the same socket with a 1 s timeout (`ICQ/src/transport.rs:187-193`); QAD is an iroh nicety we do not need. Candidate dial order: direct candidates first, punched path second, relay last; once a punched path validates, the relay circuit is closed (icq `Close` guard, `relay.rs:26-33`).

**What we self-host (DECIDED):** the helper = rendezvous + UDP relay + WebSocket listener, run as a private role of the one binary's `--daemon` mode (ASSUMED: `--daemon --helper`, same pattern as supervisor/logic in [[mux-daemon]]; the Oracle decides whether it is a third public mode). A device's own always-on cloud VM is the natural first helper, pinned in settings as `key@ip:port` (`ICQ/src/wire.rs:136-150`). Default helper list ships empty; `DEFAULT_HELPERS` (`rendezvous.rs:32`) is an icq convenience we do not copy. Optional self-hosted DNS is phase 3.

**What we never depend on:** any n0 service, any public relay we do not run, any hosted DNS. The only outside infrastructure is the mainline DHT and the peer's own ISP.

**Per-stage evidence to the UI.** Every dial produces a `Round` record with staged outcomes (icq `Stage` enum `transport.rs:16-25`: `QuicConnect, StreamOpen, AuthenticationWrite/Read/Verify, RequestWrite, ResponseRead, Complete`; rendezvous stages `helper_registration, offer_acceptance, candidate_selection, peer_dial, relay_pairing, relay_authentication, websocket_fallback`). The mux exports the live `PathEvidence` per remote device (`Direct | Punched | RelayUdp | RelayWs`, RTT, since, helper key) and the last failed round. The sidebar draws it as the seat's where-mark state ([[vision]] seat `where`); the MCP control plane exposes it read-only. Capacity exhaustion (work permit denied, `rendezvous.rs:859-873`; rate limit `:875-890`) is a distinct outcome, never a silent retry ([[messaging]] "capacity exhaustion cannot masquerade as successful delivery").

## 4. Pubsub

Own design over the quinn transport. Design target: swarm streams for the operator's own devices (phase 2), global channels and phantom presence (phase 3). iroh-gossip is the comparison, not the dependency.

**Topics.** `TopicId = SHA-256("bboxy450-term/topic/v1\0" || kind || "\0" || id)`, 32 bytes. Kinds: `swarm:<swarm ULID>` (the swarm stream: messages, presence, mute notices), `channel:<name>` (global), `group:<capability digest>` (private group), `presence:<root fingerprint>` (an account's device/seat presence for its own devices and, in phase 3, phantoms).

**Membership = grants.** Joining `swarm:` or `group:` requires a `MembershipProof`: the joiner's `SeatGrant`/`DeviceGrant` chain whose `swarm_id` or `destinations` names the topic, verified to a root the receiver trusts; a `group:` additionally requires the capability (random 256-bit, as icq `runtime.rs:1565`) which is only ever exchanged inside authenticated connections. Joining `channel:` requires only an authenticated device; nothing else is checked, so a channel's peer set is an open sample. Membership views are per receiver: no node knows the complete set, and a `SeatGrant` revocation reaches peers on contact, not instantly ([[identity]] "Revoking here is not globally instant").

**Fanout.** Phase 2: the operator's own devices are a handful; every member forwards to every other known member (full mesh) and gossip machinery is dormant. Phase 3: bounded **active view** of ≤ 8 peers per topic (passive view fed by ChannelIndex and peer exchange), eager push of each message to the active view, dedupe by `(author, message id)` in a bounded seen-cache, hop TTL 8 (icq forwards with TTL bounded 1–8, `ICQ/src/runtime.rs:651-660`), and a periodic digest of the last K message ids to each active peer for pull repair. That is HyParView plus a PlumTree-lite lazy repair; we do not build the full tree optimisation until a measured need.

**Encryption.** In flight: each hop is a QUIC/TLS connection between authenticated devices, and the envelope is signed end to end by the seat key ([[messaging]] envelope), so authenticity is end to end on every path including relays. Confidentiality is **hop to hop** for gossip: every forwarder is a verified member and reads what it forwards; for `channel:` that is the definition of public. For `group:` and cross-device `swarm:` topics content must be sealed under a group key before confidentiality across non-owner forwarders can be claimed. **Gap:** group content key derivation and rotation on membership epoch change — protocol decision; until then private topics are forwarded only by the operator's own devices. At rest: the mux store (SQLite, owner-only, synchronous FULL, as icq `store.rs:42`), keys in Keychain/gpg-agent per [[identity]]; nothing cached on helpers (a helper is "not an offline mailbox", [[messaging]]).

**What gossip does not guarantee**, and what covers it: no ordering across authors (per-author `sender_sequence` in the envelope orders one author; the receiver's commit sequence orders delivery); no delivery (the outbox retries the same message id to known recipients and exposes known-recipient receipts, [[messaging]] "Delivery, rooms and restart behavior"); no offline delivery (a member away during publication misses it until a peer's digest repair or a catch-up fetch; **Gap** carried from [[messaging]]: public-channel catch-up service); no complete membership (the pending dissemination record lists who was reached). Presence on gossip is last-reported state, never reachability proof ([[messaging]] presence gap).

**iroh-gossip, for comparison** (`source: https://docs.rs/iroh-gossip/0.101.0/iroh_gossip/`, `source: https://crates.io/api/v1/crates/iroh-gossip`): 0.101.0, 2026-06-15, MIT/Apache-2.0, HyParView + PlumTree, 32-byte `TopicId`, `Gossip::builder().spawn(endpoint)`, `subscribe(topic_id, bootstrap_peers).split()` into sender/receiver, `broadcast(bytes)`, `Event::Received`; its docs state no ordering, delivery or persistence guarantee and no max message size. Same shape as above, so swapping in phase 3 is a module replacement if §9 passes; it requires an iroh `Endpoint`, hence it is not usable over quinn.

## 5. `CloudProvider` trait

```rust
pub trait CloudProvider: Send + Sync {
    fn id(&self) -> ProviderId;                          // fly | hetzner | aws | gcp | generic
    async fn list(&self) -> Result<Vec<Vm>>;
    async fn create(&self, spec: VmSpec) -> Result<Vm>;  // spec: region, size, image, ssh pubkey, user_data
    async fn destroy(&self, id: &VmId) -> Result<()>;
    async fn bootstrap(&self, vm: &Vm) -> Result<SshTarget>;  // yields a host the SshProvider can bootstrap
}
```

`create` ends with an `SshTarget`; everything after that is §6. Provisioning never puts the supervisor binary in `user_data` (16 KiB on EC2); `user_data` only opens the user account and, where chosen, a UDP port. Auth material lives in Keychain (`kSecUseDataProtectionKeychain`, device-local, per [[identity]] "Keychain is the fallback for operational secrets") or in the provider's own CLI credential store; never in the repo, never in a seat's environment, never in a `SeatRecord` launch spec (which holds `secret_references`, [[mux-daemon]]).

| Provider | API / entry | Auth material and where it lives | Bootstrap hook | UDP reality | Source |
|---|---|---|---|---|---|
| Fly.io | `flyctl`: `fly machine run`, `fly ssh console -C "<cmd>" --machine <id>` | `FLY_API_TOKEN` or `fly auth` token file; we call the `fly` CLI, token stays in its store | ssh over Fly's WireGuard private net | no public UDP by default; VM must dial home (stages 2–4); set `auto_stop_machines = false` | [[cloud-and-mosh-exec]] §4 table; https://docs.fly.io/flyctl/cmd/fly_ssh_console |
| Hetzner Cloud | `POST /v1/servers` with `ssh_keys`, `user_data` | Bearer API token in Keychain item `bboxy450-term/hetzner` | cloud-init `#cloud-config` creates user, authorizes key | public IPv4/IPv6 by default → stage 1 direct works; firewall optional | hcloud-go `server.go:303,305` via [[cloud-and-mosh-exec]]; **Gap:** `user_data` size limit unconfirmed |
| AWS EC2 | `RunInstances` with key pair + `UserData`; or SSM `start-session` without ssh | AWS profile in `~/.aws` (its own credential chain); we never copy keys | cloud-init, 16 KiB raw user data | security group must open the QUIC UDP port for direct, else dial home | https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/user-data.html |
| GCP Compute | `instances.insert` with `startup-script` metadata; `gcloud compute ssh` (OS Login) | `gcloud` ADC credentials in its own store | guest agent runs startup script | firewall rule for UDP ingress, else dial home | https://docs.cloud.google.com/compute/docs/instances/startup-scripts |
| generic | none — the user gives an ssh host | none held by us | §6 directly | whatever the host has | [[cloud-and-mosh]] §4 "bring your own VM" |

Common fact ([[cloud-and-mosh]] §4): every provider gives ssh in minutes and public UDP only on request, so **the VM dialling home is primary and inbound direct is the optimisation**. A VM with a public IP is also the obvious self-hosted helper (§3). Cost control (stop/destroy from the sidebar) is `destroy` plus a confirm, phase "cloud"; billing visibility is out of scope.

## 6. `SshProvider` trait

```rust
pub trait SshProvider: Send + Sync {
    fn hosts(&self) -> Vec<SshHost>;                              // parsed from ~/.ssh/config Host blocks (+ Include)
    async fn probe(&self, host: &SshHost) -> Result<RemoteTriple>; // uname -sm, libc, existing install version
    async fn bootstrap(&self, host: &SshHost, enrollment: Enrollment) -> Result<RemoteDaemon>;
    async fn stdio_proxy(&self, host: &SshHost) -> Result<Connection>; // ssh -T fallback carrier for the quinn link
}
```

Mechanism follows [[mux-daemon]] "Remote seat bootstrap over SSH" step for step: system `ssh` binary (agents, jump hosts, `Match`, hardware keys all work; mosh does the same, `mosh.pl:409` in [[cloud-and-mosh]]), `-T` non-PTY control I/O, fixed probe commands, exact release selected and verified locally, upload to a user-owned versioned path, stage and atomic rename, start `--daemon` under a per-user lock, versioned framed handshake returning the remote **device public key**, daemon versions and bootstrap request id (banners are not a handshake). Enrollment material travels on framed stdin, never argv. The remote device is then enrolled under [[identity]]'s account→device→seat chain through the verified ssh session; its secret never leaves the host, ours never reaches it.

**Dial-home through `Transport`.** After enrollment the remote mux publishes its DeviceRecord (§2) and dials the laptop's device key with the ladder (§3); the laptop also dials the VM. Whichever authenticates first is kept; the other is closed (same device pair, bounded shared connections). `stdio_proxy` wraps `ssh -T host -- <binary> --daemon --stdio` as a `Connection` carrier for hosts where UDP, helpers and WebSocket are all unreachable; it is the last rung, not a normal path.

**Reconnect semantics** are [[mux-daemon]]'s, not ours: on a new connection, reauthenticate the expected device, compare supervisor boot id, process and view generations, resume or request snapshots; "do not respawn a seat or reinstall software merely because a network connection closed". Network loss leaves remote processes running. Transport contributes only the new `Connection` and its `PathEvidence`; the GUI's line cache is marked stale and refetched lazily. An ssh logout and a VM shutdown remain different failures (bootstrap step 4).

## 7. Phasing

Order is [[vision]] "v0.1 scope": local → cloud → private cross-instance → global.

| Phase | Built | Stubbed or absent |
|---|---|---|
| **v0.1 local** | `Transport` trait, `LoopbackTransport`, `TopicId` and in-process topics, `PathEvidence::Local`, messaging envelope and evidence states over it, the `net` crate boundary so no `quinn::` type appears outside it. `SshProvider::hosts()` parses `~/.ssh/config` so the launcher can already list hosts greyed out. | `QuinnTransport` behind a feature flag with `connect` returning `DialError::NotBuilt`; no DHT, no helper, no providers. |
| **Cloud** | `QuinnTransport` stage 1 (direct) and stage 0 cache; expected-key auth over TLS exporter; `SshProvider` bootstrap, enrollment, `stdio_proxy`; remote mux state streams over quinn; `CloudProvider` for **generic** and **Hetzner** (public IP, so direct works) — others are rows in settings that return `NotConfigured`. DeviceRecord publication to DHT so a roaming laptop can be found by the VM. | Punch and relay (a VM with a public IP needs neither); gossip dormant (full mesh of owner devices). |
| **Private cross-instance** | Stages 2–4: helper role, offer/accept, simultaneous probe, UDP relay, WebSocket/TLS carrier; AccountIndex announce; per-stage evidence in the sidebar; `swarm:` and `presence:` topics across the operator's devices; Fly/AWS/GCP providers (dial-home path). | Open channels; gossip active view; group content keys. |
| **Global** | `channel:` topics, ChannelIndex, bounded active view and digest repair, phantoms; §9 iroh re-evaluation runs **before** this phase's first assignment. | Offline catch-up service and history replication remain [[messaging]] gaps until decided. |

## 8. Build order (Executor-sized steps)

Each step is one assignment with one verification command; ROLE: EXECUTOR via ICQ per the agent-coordination rules.

1. **`net` crate skeleton and `Transport` trait** with `LoopbackTransport`, `TopicId`, `PathEvidence`. Verify: `cargo test -p net loopback::` — two in-process "devices" exchange a signed envelope and a receipt; a topic publish reaches two subscribers; no `quinn` in `cargo tree -p net --no-default-features`.
2. **Quinn endpoint and expected-key auth.** Self-signed rcgen cert, ALPN `bboxy450-term/1`, exporter transcript signature both directions, rejection of a wrong expected key before any stream. Verify: `cargo test -p net auth::` — right key connects, wrong key fails at `AuthenticationVerify`, stage record present.
3. **DeviceRecord + DHT publish/resolve** against a local `mainline` test DHT (icq's tests run with `allow_local`). Verify: `cargo test -p net discovery::` — highest-seq valid record selected, oversized/stale/wrong-salt/bad-signature rejected (icq test names at `ICQ/src/discovery.rs:371-572` are the checklist).
4. **Stage 0–1 dial with budgets and `Round` evidence**; mux state stream over quinn between two local mux-logic processes. Verify: `cargo test -p net dial::direct` plus a two-process smoke script that kills one side and shows the reconnect path from [[mux-daemon]].
5. **`SshProvider`**: config parsing, probe, upload, handshake, enrollment on framed stdin, `stdio_proxy`. Verify: integration run against a local `sshd` container or a second user on this Mac; `ssh -T` handshake returns device key; repeated bootstrap creates no second daemon (idempotent request id).
6. **`CloudProvider` generic + Hetzner**; Keychain-held token; create → bootstrap → seat → destroy. Verify: operator-run against a real Hetzner VM with a recorded `command — result` table; no token in logs or `SeatRecord`.
7. **Helper role**: registration leases, offer/accept, candidate merge, simultaneous probe. Verify: `cargo test -p net punch::` using two endpoints behind a userspace NAT simulator (**Gap:** pick or write one; icq tests use loopback only).
8. **UDP relay + WebSocket/TLS carrier.** Verify: `cargo test -p net relay::` — the helper cannot route an inner connection to another identity (icq `relay.rs:482`), WebSocket path works with UDP disabled via `relay_only`.
9. **Swarm/presence topics across devices**, full mesh. Verify: three mux instances, mute/unmute notice reaches all, dedupe by id, per-author order preserved.
10. **§9 re-evaluation memo**, then **channel topics + active view + digest repair**. Verify: ten-node local gossip test, every node receives every message within TTL, duplicates suppressed, a late joiner repairs from digests.

## 9. iroh as the phase-3 candidate

Facts verified 2026-10-21 (`source: https://crates.io/api/v1/crates/<name>` unless noted):

- `iroh 1.4.1` (2026-10-21), `iroh-relay 1.4.1`, `iroh-dns-server 1.4.1`, `iroh-dns 1.4.1`, `iroh-base 1.4.1`: all `MIT OR Apache-2.0`, repo `n0-computer/iroh`; minors 1.1.0 08-25, 1.2.0 09-09, 1.3.0 09-28, 1.4.0 10-20 — a release every 2–4 weeks.
- QUIC stack: `noq 1.4.0` / `noq-proto 1.4.0` (2026-10-20), repo `n0-computer/noq`, n0's fork of quinn; the local `iroh-1.3.0/Cargo.toml:289-290` depends on `noq 1.3.0`, features `fast-apple-datapath`, `qlog`, `ring`/`aws-lc-rs` pass through to noq (`:74, :81, :89, :94`). The earlier fork `iroh-quinn` stopped at 0.16.1 on 2026-01-31.
- `RelayMode` in 1.4.1: `Disabled` ("neither listening nor dialing relays will be available"), `Default` (n0 production relays), `Staging`, `Custom(RelayMap)` (`source: https://docs.rs/iroh/1.4.1/iroh/endpoint/enum.RelayMode.html`).
- Address lookup in 1.4.1 (`source: https://docs.rs/iroh/1.4.1/iroh/address_lookup/`): `MemoryLookup`, `DnsAddressLookup`, `PkarrPublisher` (n0 runs a public instance, "a reliable default choice"), `PkarrResolver` (HTTP from pkarr relays); mDNS in `iroh-mdns-address-lookup`; mainline in a separate crate the page names `iroh-mainline-address-lookup` while crates.io has `iroh-mainline-endpoint-discovery 0.1.1` (2026-09-30, repo `n0-computer/iroh-content-discovery`, depends `iroh-base ^1.1`, `n0-mainline ^0.7.1`; maps an infohash to a compact IPv4 socket and an index server maps that to a signed endpoint identity; republishes a signed server list every ten minutes; `source: https://docs.rs/iroh-mainline-endpoint-discovery/0.1.1/`). **Gap:** the two names; check which is current when phase 3 starts.
- Connection order (`source: https://docs.rs/iroh/1.4.1/iroh/`): "Endpoints first connect through their home relay. Once that connection exists, they try to set up a direct connection, using hole punching if needed." Relay connections "start as HTTP 1.1 over TLS, then upgrade to a plain TCP connection using a custom protocol" — that is the UDP-blocked carrier; `iroh-relay 1.3.0/Cargo.toml:83` also pulls `tokio-websockets` for the browser client. The relay server (`server` feature) optionally exposes a QAD endpoint for QUIC address discovery (`source: https://docs.rs/iroh-relay/1.4.1/`). Relay traffic is end-to-end encrypted: "Relay servers cannot read any of the traffic they handle" (`source: https://docs.iroh.computer/concepts/relays`).
- `pkarr 8.1.0` (2026-10-02, MIT), `mainline 8.0.1` (2026-09-30, MIT) — the upstream DHT crates both designs can share.
- Maintainer, funding and health facts stay in [[transport-and-identity]] (one company's payroll; "partly venture capital and partly founder backed"; 12.8k stars; 156 open issues).

**What it would replace:** §3 entirely (ladder, helper, relay, carrier, STUN), §2's DeviceRecord publication (Pkarr records on our own `iroh-dns-server` or mainline), and §4's fanout (iroh-gossip). It would **not** replace §1's trait, the envelope, evidence states, grants, topics-as-grants, §5, §6, or the loopback transport. Reversal cost at that point: the `net` crate's quinn module, ~5k lines by icq's measure.

**Re-evaluation criteria when phase 3 starts (all must hold, with evidence):**

1. `RelayMode::Custom` with only our relays and `Disabled`/own-server address lookup run end to end with **zero** connections to any n0 host, proven by a packet capture during a cross-NAT session.
2. Direct-first: a configuration or documented path where two directly reachable endpoints never touch a relay, or a measured relay-first cost under 200 ms that the operator accepts.
3. `noq` diff against `quinn 0.11.x` read and summarised; no divergence that forks rustls or breaks our exporter-based auth; security advisories on noq tracked.
4. Measured gain over our §3 implementation on real NATs: direct-connect rate, time to first byte, relay fallback rate, across at least the operator's home, a mobile hotspot and one corporate network. If our punch achieves ≥ 80% direct and iroh does not beat it by a margin the operator names, we stay.
5. iroh-gossip exposes the hooks §4 needs: membership gate before join (grant verification), per-message author dedupe, access to the active view for evidence; otherwise we keep our fanout on either transport.
6. The license picture is unchanged (`MIT OR Apache-2.0` on every workspace crate) and the FAQ commitment to keep relay and DNS server open still stands.

Failing any one keeps quinn. Passing all six is an operator decision, not an automatic switch.

## 10. Gaps

- **Gap:** `icq-mainline 8.0.1-icq.1` patch contents versus upstream `mainline 8.0.1` — read before pinning.
- **Gap:** helper as a third public mode versus a private `--daemon` role; vision says one binary, two modes.
- **Gap:** group content key derivation and rotation for private topics forwarded by non-owner members; until decided, private topics are forwarded only by the operator's own devices.
- **Gap:** NAT simulator for punch tests; icq's test suite stays on loopback.
- **Gap:** no measured direct-connect success rate for the icq-shaped puncher on real NATs (carried from [[transport-and-identity]]).
- **Gap:** relay circuit renewal protocol and per-account quotas for hours-long mux links — icq's 120 s / 128 MiB were sized for messages.
- **Gap:** AccountIndex is unsigned by design; a flood of bogus `announce_peer` entries under a root's infohash costs the resolver up to 32 failed dials. Bound dial fan-out and prefer `AccountHead` sibling keys.
- **Gap:** Hetzner `user_data` size limit; Fly machine persistence policy; whether Apple's Keychain item access controls survive the one-binary packaging ([[identity]] gap).
- **Gap:** public-channel catch-up, history replication and presence expiry — [[messaging]] gaps, unchanged here.
- **Gap:** `iroh-mainline-address-lookup` versus `iroh-mainline-endpoint-discovery` naming; and `noq` divergence from quinn — both measured only at §9 time.
- **Gap:** 1000-seat connection sharing: how many device pairs and streams one mux holds before backpressure; no measurement.

Written to `.bboxy/specs/networking.md` only. No code, dependency, task-board or Git change.
