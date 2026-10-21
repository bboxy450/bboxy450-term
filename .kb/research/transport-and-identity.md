# Transport and identity — iroh vs quinn; Apple/Google sign-in

Status: research. Written 2026-10-21 by a research subagent for Vesper Greywind <ORACLE>. Facts dated as fetched on 2026-10-21.

Links: [[vision]] [[agent-messaging]] [[messaging]]

---

## Transport: iroh vs quinn

### iroh — facts

**License.** Every workspace crate is `MIT OR Apache-2.0`. Verified per crate in the repo's `Cargo.toml` files: `iroh`, `iroh-relay`, `iroh-base`, `iroh-dns-server`, `iroh-dns` (`source: https://raw.githubusercontent.com/n0-computer/iroh/main/<crate>/Cargo.toml`, each `license = "MIT OR Apache-2.0"`). crates.io agrees for every published version of `iroh` and `iroh-relay` back through 1.0.0-rc.1 (`source: https://crates.io/api/v1/crates/iroh`, `source: https://crates.io/api/v1/crates/iroh-relay`). GitHub's detector reports Apache-2.0 only because it reads one file; the README says "at your option" dual license (`source: https://github.com/n0-computer/iroh`). No dual-commercial, no CLA-with-relicense clause found. Workspace members: `iroh-base, iroh-dns, iroh-dns-server, iroh, iroh/bench, iroh-relay` (`source: https://raw.githubusercontent.com/n0-computer/iroh/main/Cargo.toml` lines 2-8).

**It is not built on quinn.** iroh 1.x depends on `noq`, n0's own fork of quinn: `noq = { version = "1.4.0" ... }`, `noq-proto`, `noq-udp` (`source: https://raw.githubusercontent.com/n0-computer/iroh/main/iroh/Cargo.toml` lines 44-46, 78). `noq` 1.4.0, MIT OR Apache-2.0, repo `n0-computer/noq`, released 2026-10-20 (`source: https://crates.io/api/v1/crates/noq`). The fork dates to May 2024, motive: quinn did not expose congestion-controller internals they needed to cut latency settling on bad paths from ~20 s to ~3 s; they intended to track upstream (`source: https://www.iroh.computer/blog/why-we-forked-quinn`). The intermediate fork `iroh-quinn` stopped at 0.16.1 on 2026-01-31 (`source: https://crates.io/api/v1/crates/iroh-quinn`). Adopting iroh means adopting a second QUIC stack maintained by one company. **Gap:** how far `noq` has diverged from quinn 0.11.x today — not measured.

**Maintainer and funding.** Company: number 0 (`n0`), GitHub org `n0-computer`, "we make iroh"; founder/CEO Brendan O'Brien (b5) (`source: https://github.com/n0-computer`, `source: https://www.youtube.com/watch?v=b2iX5vKIN-k`). FAQ: funding "partly venture capital and partly founder backed"; revenue from Iroh Services (managed relays + DNS lookup); "We rely on iroh remaining open source, and are committed to keeping it that way", including relay and DNS server code (`source: https://docs.iroh.computer/about/faq`). Investors and round sizes are not published anywhere I could reach. **Gap:** investor names, amounts, runway — no data. Top contributors by commits: dignifiedquire 792, flub 513, rklaehn 308, Frando 287, Arqu 150 (`source: https://api.github.com/repos/n0-computer/iroh/contributors`) — one company's payroll, not a community.

**Release cadence and health.** 1.0.0 on 2026-06-15; 1.1.0 08-25; 1.2.0 09-09; 1.3.0 09-28; 1.4.0 10-20; 1.4.1 10-21 — a minor every 2-4 weeks (`source: https://crates.io/api/v1/crates/iroh`). GitHub: 12,785 stars, 737 forks, 156 open issues, 929 closed, 25 open PRs, last push 2026-10-21; oldest open issue 2023-10-19 (`source: https://api.github.com/repos/n0-computer/iroh`, `source: https://api.github.com/search/issues?q=repo:n0-computer/iroh+is:issue+is:open`). Healthy by numbers. 3.49M total downloads.

**Relay: optional, self-hostable.** `RelayMode` enum in iroh 1.4.1: `Disabled` ("Disable relay servers completely. This means that neither listening nor dialing relays will be available."), `Default` (n0 production relays), `Staging`, `Custom(RelayMap)` (`source: https://docs.rs/iroh/latest/iroh/endpoint/enum.RelayMode.html`). `iroh-relay` ships the server binary with built-in ACME TLS; docs admit "Full configuration options are not published currently" (`source: https://docs.iroh.computer/iroh-services/relays/self-hosted.md`). FAQ: "You're not dependent on us"; "Running your own relay doesn't affect interoperability"; public relays "free to use for development and testing", Shared Relays are a paid Pro-plan item (`source: https://docs.iroh.computer/about/faq`, `source: https://docs.iroh.computer/deployment/dedicated-infrastructure.md`). Relay traffic is end-to-end encrypted; the relay "cannot read" it (`source: https://docs.iroh.computer/concepts/relays`).

**Discovery: defaults to n0, replaceable.** Default publishes signed Pkarr records "on an `iroh-dns-server` instance that is run by n0.computer" and resolves via origin `dns.iroh.link`, "a server run by n0". Alternatives: self-hosted `iroh-dns-server`, mDNS (separate crate), BitTorrent Mainline DHT (opt-in, no n0 servers) (`source: https://docs.iroh.computer/concepts/discovery`). Fedimint runs this way in production: `RelayMode::Custom` + own `PkarrPublisher` + flags to disable n0 discovery (`source: https://docs.fedimint.org/src/fedimint_core/net/iroh.rs.html`). So: fully off n0 infra = run your own relay + your own iroh-dns-server, or use DHT-only discovery.

**iroh-net / iroh split.** Dead since 0.29: "iroh-net is now just iroh. we no longer publish `iroh-net`"; blobs/docs/gossip split into separate crates; netcheck/portmapper moved to `net-tools` (`source: https://iroh.computer/roadmap`, `source: https://iroh.computer/blog/iroh-0-29-net-is-the-new-iroh`). Today `iroh` = the connection layer only.

**Hole punching mechanism.** Tailscale lineage: "all hole punching in a 'MagicSocket,' using DERP for relaying and Disco for hole punching, but running over QUIC instead of WireGuard"; "Hole punching requires connection to at least one DERP+STUN server" (`source: https://number-zero.notion.site/Iroh-Hole-Punching-b9a38cc228fb47328a73e71d10520418`, `source: https://docs.rs/iroh-net`). Flow: both peers hold a relay connection, relay observes each reflexive address, peers exchange candidates through the relay, both fire UDP simultaneously, the direct path is validated and the relay path falls away; relay stays as fallback; "roughly 9 in 10" networks get direct (`source: https://docs.iroh.computer/concepts/holepunching`). Since ~0.32 STUN is replaced by QAD — QUIC Address Discovery, an `OBSERVED_ADDRESS` frame inside the relay QUIC connection (`source: https://iroh.computer/blog/qad`). Multipath via the QUIC multipath draft; stated direction is to move punching into QUIC itself per the QUIC-NAT-TRAVERSAL draft (`source: https://iroh.computer/blog/iroh-on-QUIC-multipath`). Dial-by-key is done by mapping each EndpointId to a synthetic ULA IPv6 address handed to the QUIC endpoint. This is ICE-like (candidate exchange + simultaneous open) over a DERP-style relay, with QUIC migration doing the path switch.

### quinn — facts

`quinn` 0.11.12 (2026-09-14), prior 0.11.11 (2026-06-22), 0.11.9 (2025-08-27): quarterly-ish patch cadence, 367M downloads, `MIT OR Apache-2.0` (`source: https://crates.io/api/v1/crates/quinn`). GitHub: 5,303 stars, 138 open issues, last push 2026-10-18; maintainers Ralith (Benjamin Saunders, 1689 commits) and djc (Dirkjan Ochtman, 1155), independent, not one company (`source: https://api.github.com/repos/quinn-rs/quinn`, `source: https://api.github.com/repos/quinn-rs/quinn/contributors`). quinn has no NAT traversal of its own; everything above UDP sockets is yours.

### What icq builds on (the blueprint)

From `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/icq-0.20.2/`:

- `Cargo.toml:115-121` — `quinn = "=0.11.12"`, features `runtime-tokio`, `rustls-ring`, default-features off. Transport is upstream quinn.
- `Cargo.toml:141-147` — `rustls 0.23` (`ring`, `std`); `Cargo.toml:126` — `rcgen 0.13` for self-signed certs; `Cargo.toml:94-96` — `ed25519-dalek 2` (identity key); `Cargo.toml:78-79` — `chacha20poly1305 0.11`.
- `Cargo.toml:196-197` — `stunclient = "=0.4.2"`; used in `src/transport.rs:187-193` `map_socket()` to learn the reflexive address with 1 s timeout (classic STUN, not QAD).
- `Cargo.toml:111-113` — `mainline = "=8.0.1-icq.1"` (`package = "icq-mainline"`, a patched BitTorrent Mainline DHT); `src/discovery.rs:9` — `use mainline::{Dht, Id, MutableItem}`; signed mutable items keyed by the identity key (`src/discovery.rs:322`). Discovery is DHT, no hosted DNS.
- `Cargo.toml:206-212` — `tokio-tungstenite = "=0.30.0"` (`rustls-tls-webpki-roots`); `src/transport/websocket.rs:1-2` — "A bounded WebSocket carrier for QUIC datagrams. It conveys no application authority: ICQ's expected-key/exporter proof still runs inside QUIC." The relay is a dumb datagram carrier; the QUIC session stays end-to-end.
- `src/rendezvous.rs:1` — "Authenticated direct-path coordination with a bounded encrypted relay fallback."; `src/rendezvous/relay.rs:1-2` — "Bounded opaque carrier for the ordinary end-to-end QUIC connection. The helper authenticates circuit participants, never application operations."; `src/rendezvous/lan.rs:1` — private LAN candidates admitted only when both sides share an observed public IP. `src/rendezvous.rs:1084-1100` — `accepted_candidates()` merges observed + offered candidates; `src/rendezvous.rs:782` — `transport::probe_observed()` races them.
- Module size: `rendezvous*` 3,606 lines + `transport*` 1,422 lines ≈ 5k lines for discovery, punch, relay and diagnostics (`wc -l`).

Mechanism is the same shape as iroh's: STUN-observed address → candidates exchanged through an authenticated helper → simultaneous probe → relay fallback carrying opaque QUIC. Differences: icq uses STUN + DHT, no DNS, no multipath; iroh uses QAD + Pkarr/DNS, multipath, forked QUIC.

### Alternatives, one line each

- `str0m` 0.24.1 (2026-10-03), MIT OR Apache-2.0, sans-IO WebRTC; ICE split into the `is` crate — full ICE agent but drags SDP/DTLS/SRTP concepts into a non-media app (`source: https://crates.io/api/v1/crates/str0m`, `source: https://docs.rs/str0m`).
- `webrtc` 0.21.1 / `rtc` 0.21.1 (2026-10-11), MIT/Apache-2.0, webrtc-rs async + new sans-IO stack; same objection, heavier (`source: https://crates.io/api/v1/crates/webrtc`, `source: https://webrtc.rs/blog/2026/01/04/announcing-rtc-v0.3.0`).
- `libp2p-dcutr` 0.15.0 (2026-09-11), MIT, DCUtR over circuit-relay-v2 with libp2p-quic; buys the whole libp2p swarm/identify/multiaddr stack for one feature (`source: https://crates.io/api/v1/crates/libp2p-dcutr`).

### Recommendation — transport

Build on `quinn`, icq-style. Grounds: the operator's gate was "open source under an acceptable license AND trustworthy". iroh passes the license gate cleanly (MIT OR Apache-2.0 everywhere, including relay and DNS server). It half-passes trust: well-funded-looking, high cadence, committed to open source in writing, but it is a single VC-backed company's codebase on top of that same company's fork of quinn (`noq`), with defaults that phone its hosted DNS and relays. Opting out is documented and real (Fedimint does it), but it is configuration against the grain, and every iroh release is also a `noq` release you did not pick. icq already proves the quinn + STUN + DHT + WebSocket-relay shape in ~5k lines; the blueprint is in the registry on this machine.

What iroh would buy that icq's shape does not have: QAD instead of STUN, multipath, ~90% direct-connect figure measured at scale, 1000-agent-tested relay server. Those are nice, not gating for v0.1.

Reversibility: high at the application layer if the messaging crate exposes `connect(key) -> QUIC connection` and the rooms/priority code never touches sockets. Switching to iroh later = swap the transport crate behind that trait; both hand you a QUIC `Connection` keyed by an Ed25519 public key. Switching away from iroh later would be harder (noq types leak into stream APIs). So quinn-first is the cheap-to-reverse order.

**Gap:** no measured direct-connect success rate for the icq puncher on real NATs — no data. **Gap:** `noq` divergence from quinn — not measured.

---

## Identity: Apple/Google sign-in

### Sign in with Apple — native macOS

**API surface exists in Rust.** `objc2-authentication-services` 0.3.2 (2025-10-04), `Zlib OR Apache-2.0 OR MIT`, madsmtm/objc2, 1.0M downloads; exposes `ASAuthorizationAppleIDProvider`, `ASAuthorizationController`, `ASAuthorizationControllerDelegate`, `ASAuthorizationControllerPresentationContextProviding`, `ASAuthorizationAppleIDCredential`; requires `objc2 >=0.6.2` (`source: https://crates.io/api/v1/crates/objc2-authentication-services`, `source: https://docs.rs/objc2-authentication-services/latest/objc2_authentication_services/`). Delegate implementation in objc2 is `define_class!` boilerplate — a day of work, not a week.

**Entitlement.** `com.apple.developer.applesignin`, array of strings, only value `Default`, macOS 10.15+ (`source: https://developer.apple.com/tutorials/data/documentation/bundleresources/entitlements/com.apple.developer.applesignin.json`). Needs a paid Apple Developer Program App ID with the capability enabled and a provisioning profile that carries it.

**The blocker: not available to Developer ID apps.** Apple DTS, June 2022: "Is it possible to use 'Sign in with Apple' capability in applications signed with Developer ID certificate? No." (`source: https://developer.apple.com/forums/thread/707074`). Apple DTS, Nov 2024: "Sign in with Apple is not supported in Developer ID apps ... Note that Sign in with Apple is not available in the Developer ID column." (`source: https://developer.apple.com/forums/thread/768236`). A developer who signed anyway got an invalid-signature crash at launch (`source: https://developer.apple.com/forums/thread/760861`). A terminal distributed outside the Mac App Store (Homebrew cask, GitHub release, notarized Developer ID) **cannot use the native ASAuthorization flow**. Only Mac App Store distribution gets it. **Gap:** the live Supported-capabilities table (`developer.apple.com/help/account/reference/supported-capabilities-macos/`) did not render through my fetcher; the DTS quotes are the evidence.

**The web (Services ID) flow is the only route for a Developer ID app.** Register a Services ID, verify a domain, register HTTPS return URLs — Apple rejects `http://`, `localhost` and IP addresses as return URLs (`source: https://actingweb.readthedocs.io/en/latest/_sources/docs/guides/apple-sign-in.rst.txt`, `source: https://developer.apple.com/forums/thread/827464`). `response_mode=form_post` is required when asking for name/email. That means a server you run receives the POST and hands the result back to the desktop app — a hosted callback, not a loopback. Apple's own doc pages for return-URL rules did not render for me; **Gap:** verify "no localhost" against `developer.apple.com/documentation/signinwithapple/configuring-your-webpage-for-sign-in-with-apple` by eye.

**What you get.** `ASAuthorizationAppleIDCredential`: `user` (stable per-team id), `identityToken` (JWT), `authorizationCode`, `email` and `fullName` (first authorization only; `nil` afterwards), `realUserStatus`, `state`, `authorizedScopes` (`source: https://developer.apple.com/documentation/authenticationservices/asauthorizationappleidcredential` — page body did not render; properties from the credential class reference as paraphrased by the fetcher). Email may be a `@privaterelay.appleid.com` address; the token carries `is_private_email`. The JWT: `iss=https://appleid.apple.com`, `sub`, `aud` = bundle ID or Services ID, `email`, `email_verified`, `nonce`; verify against JWKS at `https://appleid.apple.com/auth/keys` (`source: https://developer.apple.com/tutorials/data/documentation/signinwithapple/authenticating-users-with-sign-in-with-apple.json` confirms nonce binding and that email is in the token on subsequent responses; claim list paraphrased).

### Google sign-in — desktop

**Flow is standard and unblocked.** Desktop app client type, loopback redirect `http://127.0.0.1:<port>` or `http://[::1]:<port>` on a random free port, system browser, "Always use PKCE", S256; `client_secret` is optional and not confidential for desktop clients; custom URI schemes and OOB copy-paste are gone (`source: https://developers.google.com/identity/protocols/oauth2/native-app`). Loopback stays supported for desktop while being retired for mobile/Chrome types (`source: https://developers.google.com/identity/protocols/oauth2/resources/loopback-migration`).

**Crate.** `oauth2` 5.0.0, MIT OR Apache-2.0, authorization-code flow with `PkceCodeChallenge::new_random_sha256()` / `set_pkce_challenge` (`source: https://docs.rs/oauth2/latest/oauth2/`). The loopback receiver is a `std::net::TcpListener` on 127.0.0.1 reading one GET line; nothing to research. **Gap:** the repo's `examples/google.rs` returned 404 for me; the docs page links it but I did not read it.

**Console setup.** Google Cloud project, OAuth client, consent screen. Testing status: 100 test users, consent expires after 7 days — **except** when the app requests only `openid`, `email`, `profile`: then test users need not be listed, see no warning, and authorizations do not expire (`source: https://support.google.com/cloud/answer/15549945`). Verification (brand or scope) is tied to sensitive/restricted scopes and to showing a logo/name; basic identity scopes do not require it to go In production (`source: https://support.google.com/cloud/answer/10311615`, `source: https://support.google.com/cloud/answer/15549945`). So Google identity-only sign-in has no review gate.

**What you get.** ID token JWT: `sub` ("unique among all Google Accounts and never reused"; "Use `sub` within your application as the unique-identifier key"), `email`, `email_verified`, `hd` (Workspace domain), `aud`, `iss`; "Don't use the `email` field as a unique identifier"; verify with Google JWKS from the discovery doc (`source: https://developers.google.com/identity/openid-connect/openid-connect`).

### What OAuth adds over key + email

A self-generated Ed25519 key plus a typed email gives: a stable identity (the key) and an unverified claim (the email). Apple/Google add exactly one thing: a **third-party-signed assertion** that the person holding this device controls that email (`email_verified`) and a provider-stable `sub`. That is what lets a stranger's agent trust that `#channel` member `bboxy450@gmail.com` is the Google account of that name, and what lets you recover or link a second machine's key to the same account without trusting self-assertion. It does not replace the key; messaging, signing and transport still authenticate the key. Nothing in either provider gives you anything about the agent seats — only the human.

### Credible path: key is root, OAuth binds an email claim to the key

Shape: on sign-in, the app generates a nonce = hash(key_pubkey || random); runs the provider flow with that nonce; receives an ID token whose `nonce` claim binds the provider's `sub`+`email` to this key; stores `{key, provider, sub, email, id_token}` locally as a self-contained attestation; publishes `sign_key(attestation)` wherever the identity record lives (DHT item / Pkarr record / user profile). Peers verify the ID token against the provider's JWKS and check `nonce` covers the key. No account server required for Google. Apple refresh/revocation and anything needing a client secret needs a server.

**Google: one week.** `oauth2` + loopback listener + `open` crate + JWT verify (`jsonwebtoken` + JWKS fetch) + attestation record + UI button. The console setup is an afternoon; no review for identity-only scopes. Estimate two to four days of Executor work plus one day of Oracle verification.

**Apple: one month, and only if a server exists.** Native flow is off the table for a Developer ID app (two DTS answers above). The web flow needs: Apple Developer Program, Services ID, verified HTTPS domain, hosted callback endpoint receiving `form_post`, a hand-back to the desktop app (poll or deep link), private-key-signed client secret for the code exchange, JWKS verification, private-relay email handling. That is backend work plus an Apple review surface, and it only becomes "cheap" if the Mac App Store becomes the distribution channel.

### Recommendation — identity

Ship day one as decided: key + email. Add **Google** in v0.1 as the first binding — it is one week, loopback+PKCE, no server, no review, and it exercises the "attestation signed by key" record that Apple will later reuse. **Defer Apple** until (a) a hosted callback domain exists for other reasons (relay/DNS server, cloud harnesses) or (b) the Mac App Store is a distribution channel. Design the attestation record provider-agnostic now so Apple is a second row, not a redesign.

Reversibility: the binding is an additive signed record; removing or ignoring it costs nothing. Doing Apple first and discovering the Developer ID wall would cost the month.

**Gap:** what "same user across machines" proves once a provider `sub` is bound — this gives linkage of keys by one email, but key revocation/rotation policy is undesigned (ties to vision open gap on seat keys).

---

## Sources not reachable

Apple developer.apple.com documentation pages (entitlement, credential class, web-flow configuration, supported-capabilities table) render client-side; only the `/tutorials/data/.../*.json` payloads returned text. Those facts are marked paraphrased above. Apple Developer Forums pages rendered and are quoted verbatim. `ossinsight.io` showed placeholders. `ramosbugs/oauth2-rs` examples directory 404.
