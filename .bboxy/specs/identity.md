# Identity and account

2026-10-21 · Nyx Copperwing <EXECUTOR> · task I2 · GPG-root rewrite for Oracle review.

[[vision]] · [[messaging]] · [[mux-daemon]] · [[mcp-control-plane]] · [[harness-adapters]] · [[transport-and-identity]] · [[ux-prototype]] · [[README]]

## Identity and authority

The account root is the operator's existing **OpenPGP primary key**, the same authority used for their signed commits. The terminal selects and pins its public certificate; it does not create a second account root. Same pinned primary key, same account. Changing the email, UID, character name, avatar or Google binding does not replace that root.

For Boxy, the selected primary fingerprint is **9B07 E849 480C 6832 8CA2 0684 844E CC05 87DE 7AB0**, UID **Boxy (bboxy450) <bboxy450@gmail.com>**. Show the full fingerprint in identity, pairing and recovery views; a short key ID or email match cannot select an account. Read-only `gpg --with-colons --fingerprint --list-keys` confirmed a 255-bit algorithm-22 Ed25519 primary on this machine on 2026-10-21.

The current wire profile keeps `root_pubkey` as lowercase hex of the primary's raw 32-byte Ed25519 public value. Resolve it only through the pinned, valid OpenPGP certificate and require an exact match; it is not a second root. Pin the complete OpenPGP primary fingerprint alongside that mapping in local account state and public proof bundles. An attacker-supplied certificate, key ID, UID or server lookup cannot replace a pinned root.

Email is taken from an operator-selected, valid self-signed GPG UID. It is self-asserted mailbox metadata until separately verified. Google is an optional later binding to that root, never account sign-in or recovery authority; Apple remains deferred. The later milestone cut in [[vision]] takes precedence over its older “Google in v0.1” wording: first render/local MCP/harness visibility, then cross-instance identity.

Only an explicit operator identity action can request account-primary signing, enrollment, root-level revocation or binding. No MCP role gets primary-secret export, arbitrary root signing or recovery. Role metadata never grants permission to execute peer message text; signed classes authorize only the delegated operations allowed by a verified grant.

Review lens: custody, delegation, signature format, revocation and recovery. Assumes the pinned primary and signing host are uncompromised. A compromised same-user process with access to the agent socket may still request signatures; GUI confirmation is an application boundary, not a claim of OS isolation from that process.

**Gap:** the preserved raw-key grant and Google nonce formats cover this tested Ed25519 profile. RSA/non-Ed25519 primaries, OpenPGP v6 fingerprints and two primary certificates sharing raw key material need an explicit versioned identity mapping before support; do not generate a replacement root or silently truncate a fingerprint.

## OpenPGP key hierarchy

Choose **GPG-certified Ed25519 leaf certificates**, rather than attaching every seat directly as a subkey of the account primary. A device needs to certify its own seats without bringing the account primary online for every launch. OpenPGP subkey bindings are made by their primary; independent certified device keys preserve that delegation boundary. [OpenPGP certificate and binding structure, RFC 9580](https://www.rfc-editor.org/rfc/rfc9580#section-10.1).

```text
pinned account primary (operator's existing GPG key)
    └─ certifies device Ed25519 certificate + signs DeviceGrant/AccountHead
         └─ certifies seat Ed25519 certificate + signs SeatGrant/SeatRoster
              └─ signs messages and presence; optional Git signing remains off
```

| Level | Public proof and purpose | Secret custody |
|---|---|---|
| Account | Existing primary certificate, selected self-signed UID, full fingerprint; signs account heads, device delegation and optional Google binding. | GnuPG/gpg-agent on a machine that actually holds or can use the primary; an offline primary stays offline until requested. |
| Device | Independent Ed25519 certificate, self-signed UID naming its durable device ID, certification by the pinned account primary; separate `DeviceGrant` supplies bounded application authority. Certifying/signing flags cover its intended operations. | Generated locally; gpg-agent if available, otherwise the device's protected Keychain backend. Never copied to another device by enrollment. |
| Seat | Independent Ed25519 certificate, self-signed UID naming seat ID and key epoch, certification by its authorized owning device; `SeatGrant` supplies swarm, role and destinations. | Owning terminal's protected local store/signer; a harness gets a seat-bound signing capability, not the secret or device signer. |

Generate independent device/seat keys with the OS cryptographic RNG. Their raw public values must exactly match the Ed25519 keys represented in their OpenPGP certificates; do not certify one key while signing messages with another. Transport session encryption keys remain separate. Seat close, resume or process restart does not imply a new account or leaf key; compromise/replacement explicitly rotates the affected key epoch.

Certifications bind the exact subject certificate/UID and issuer; application grants bind IDs, scopes, validity and policy. Require both. A root certification of a device is not unlimited permission for that device or its children. Public certificate chains expose only the chosen public identifiers; private grants and host metadata are not embedded in UIDs or published certifications.

## Signed formats and verification

Keep the existing typed grant payloads and their raw Ed25519 signatures. Encode payloads using [JCS, RFC 8785](https://www.rfc-editor.org/rfc/rfc8785); reject duplicate/unknown fields, malformed keys, unsupported versions and out-of-range values. Sign `UTF8("bboxy450-term/<type>/v1\0") || JCS(payload)`. The record digest is SHA-256 of those bytes; detached signature and digest sit outside the payload. This does not change [[messaging]]'s separately defined signed message tuple.

Keys remain lowercase 64-character raw-public hex; signatures remain unpadded base64url of 64 bytes. Durable device/seat/swarm/tab/operation/message IDs are canonical **ULIDs** under decision S1 in [[README]]. Record digests, OpenPGP fingerprints, random capabilities and harness-native session IDs retain their own types. Epochs/counters are bounded unsigned decimal strings; times are integer Unix seconds within JSON's safe-integer range. Scope lists have deterministic ordering and no duplicates. No record contains a private key, enrollment capability or provider bearer token.

| Record / signer | Required payload fields |
|---|---|
| `AccountHead` / account primary | `version, root_pubkey, account_epoch, previous_head_id, active_devices, revoked_devices, current_binding_ids, revoked_binding_ids, issued_at`. Active entries bind device ID, raw key and device epoch. Binding references are record digests. |
| `DeviceGrant` / account primary | `version, root_pubkey, account_epoch, device_id, device_pubkey, device_epoch, serial, scopes, role_ceiling, not_before, expires_at`. Scopes distinguish device transport, seat issuance and provenance publication; none delegates primary custody/recovery. |
| `SeatGrant` / owning device | `version, root_pubkey, device_grant_id, device_id, device_epoch, seat_id, seat_pubkey, key_epoch, grant_epoch, previous_grant_id, swarm_id, harness_kind, role, destinations, control_scopes, muted, where, not_before, expires_at`. Child authority cannot exceed its parent. |
| `SeatRoster` / provenance-authorized device | `version, root_pubkey, device_grant_id, device_epoch, roster_sequence, previous_roster_id, issued_at, entries`. Its public index refers to the same OpenPGP certification chain; it creates no independent root. |
| `GoogleBinding` / account primary | `provider, subject, email, root_pubkey, nonce_random, id_token_hash, issued_at`, plus the separate root signature/provider-proof bundle. No device/seat permissions. |

Two next-touch addenda apply without renaming other fields: `role={name,class}`, with `class∈{oracle,worker}`; and `where=local|ssh{host}|cloud{provider,host}`. The latter is a tagged value, not a new ID or proof of remote-host authentication. Host/provider/contact metadata stays in private seat grants/local records, never the public roster. `harness_kind` is an open string. Concrete tagged-value wire encoding is coordinated with [[mux-daemon]].

Roles are defined per account and assigned per seat. Name is display; **all authority/priority gates use the signed class**, current grant and receiver policy. `role_ceiling` is an explicit allowed-class set, not a numeric priority or a list of trusted names. Changing a role's class reissues every affected seat grant with a greater grant epoch before new effects. Only this receiver's own pinned root chain can confer Oracle precedence; foreign oracle-class seats remain worker-class here. Device-local scheduling follows [[messaging]].

Device grants default to at most **30 days**, seat grants to **24 hours**, never beyond parent or applicable certificate validity. These are finite configurable defaults. Root-approved renewal may retain the same device key; devices renew seats within their ceiling. Expiry blocks new authenticated effects; it does not terminate a harness or erase history.

A verifier pins the full primary certificate/fingerprint, matches all raw public keys, checks self-signatures and the exact account→device→seat certifications under an explicit crypto policy, then verifies every application signature/domain, parent reference, membership/epoch, validity, role/class and scope. Match the seat key to the signed envelope and the device key to the authenticated connection. GnuPG's trust database, an email string or a “good signature” label alone cannot grant app authority. Reject unsupported algorithms/ambiguous chains before scheduling privileged effects.

Stable seat IDs/keys can survive harness resume. Each new process generation reacquires its local capability and writer lease under [[mux-daemon]]/[[mcp-control-plane]]. Ordinary renewal/mute updates do not rewrite historical envelopes. Dispatch still requires current authority clamped to the original signed scope/class: demotion cannot regain precedence through an old message, and a revoked key cannot create new effects by backdating.

**Gap:** complete canonicalization/negative signature vectors, record/certificate size limits, exact proof-bundle framing and the current-grant exchange remain implementation work. Messaging's old role/ID wording must take this next-touch decision; no other spec is edited by I2.

## Root custody and local signing boundary

The app **never holds, imports or exports the account primary secret**. Public certificate selection may read GnuPG's public metadata. Root creation, secret backup/import/export and revocation-certificate management remain operator GPG workflows; the UI points to those workflows rather than offering a new root wizard or recovery-seed field.

First-device bootstrap selects the existing primary certificate/UID, creates the independent device key, then requests its primary certification and a constrained `DeviceGrant` plus `AccountHead` at epoch 1 with a null predecessor. Commit those proofs and the authority journal before activating device authority. An absent primary leaves bootstrap pending; never generate an account root to bypass it.

Resolve the explicitly selected local GnuPG context/socket, primary certificate and exact primary keygrip. Before each root operation, require the typed, bounded request and expected fingerprint; reject unavailable/mismatched primary signing instead of letting the agent choose a default key. Keep primary signing in the operator identity path, outside MCP/seat sockets. Never forward the account agent socket to seats, SSH/cloud harnesses or an unauthenticated broker.

Use the agent's normal **pinentry and cache** behavior. No app password, remember-password box, loopback passphrase capture or cache-preset operation. Cached availability may mean no new pinentry appears; do not claim every operation receives fresh user presence. Cancellation/unavailable agent leaves the operation pending/failed without installing a grant. [GnuPG agent signing protocol](https://www.gnupg.org/documentation/manuals/gnupg/Agent-PKSIGN.html).

For current raw grant signatures, select `SIGKEY <exact-primary-keygrip>`, supply the complete domain+canonical payload using `SETHASH --inquire` and its `TBSDATA` inquiry, then `PKSIGN`. Decode the returned EdDSA `r,s` values to the strict 64-byte signature and independently verify it before committing. Do not prehash the payload or substitute an OpenPGP packet signature for the existing raw signature. This path was tested on **GnuPG 2.5.24** only; negotiate/probe support and fail explicitly on unsupported agents.

For OpenPGP certifications and optional detached roster/Git signatures, the selected OpenPGP library constructs the packet hash/trailer and a public-key-backed adapter delegates signing to gpg-agent. These are different signing encodings using the same selected authority, not interchangeable 64-byte outputs.

Keychain is the fallback for **operational device/seat secrets** and public account state, not a way to load the primary into the application. Use non-synchronizing, device-local Data Protection Keychain items in the user-login service context, explicitly selecting `kSecUseDataProtectionKeychain` and the intended access controls; an unavailable store holds signing work, with no plaintext-file fallback. [Apple TN3137](https://developer.apple.com/documentation/technotes/tn3137-on-mac-keychains). A machine without the primary can operate within its current device grant but cannot enroll/renew root authority locally.

**Gap:** a separate opaque Keychain-backed *primary* signer preserving “the app never holds the primary secret” has not been implemented or demonstrated. Until that boundary exists, no-agent primary signing uses the offline-primary request flow below. macOS 14 packaging/entitlements, leaf-store lock behavior, hardware tokens and cached-pinentry behavior still need runtime tests.

## Rust integration and backend gate

**Recommend rPGP (`pgp`) plus our own narrow Assuan client.** The Oracle's default is no LGPL in the tree; our crates remain MIT OR Apache-2.0. Keep Sequoia as a comparison, not an adopted dependency. I2 adds no Cargo dependency.

| Candidate | Observed fit | Cost / decision |
|---|---|---|
| **rPGP `pgp 0.21.0`** | Pure-Rust OpenPGP packet/certificate parsing, third-party certification creation/verification and an external-signer trait. Package license **MIT OR Apache-2.0**, MSRV **1.88**; default feature is optional `bzip2`. | Preferred candidate with `default-features=false` for this keys/signatures-only path. We must implement policy evaluation and the agent adapter; no complete trust policy is supplied by the low-level API. Transitive licenses and the actual locked feature graph still need checking. |
| **Sequoia `sequoia-openpgp 2.4.2` + `sequoia-gpg-agent 0.6.2`** | Certificate/packet APIs, explicit policy machinery and an existing public-key-backed agent signer. | Both **LGPL-2.0-or-later**, therefore excluded by default unless the operator changes that decision. Default OpenPGP backend is Nettle; pure-Rust RustCrypto needs experimental/variable-time opt-ins that upstream qualifies. |
| **Own Assuan client → installed GnuPG agent** | The disposable probe verified raw Ed25519 signing without exporting a secret. The same selected agent can sign OpenPGP packet digests. | Required small custody boundary, independent of the OpenPGP parser choice. GnuPG remains a separate installed process; do not copy/link Sequoia's LGPL adapter implementation. |

Primary package evidence: [rPGP manifest](https://docs.rs/crate/pgp/0.21.0/source/Cargo.toml), [rPGP upstream scope/license](https://github.com/rpgp/rpgp), [Sequoia manifest](https://docs.rs/crate/sequoia-openpgp/2.4.2/source/Cargo.toml), [agent manifest](https://docs.rs/crate/sequoia-gpg-agent/0.6.2/source/Cargo.toml), [Sequoia backend guidance](https://docs.rs/crate/sequoia-openpgp/2.4.2/source/README.md).

| Exact source inspected in the official `pgp-0.21.0.crate` | Mechanism / implementation consequence |
|---|---|
| `src/composed/signed_key/public.rs:116`; `src/types/user.rs:42,57` | `SignedPublicKey::verify_bindings` and per-user self/third-party verification are available. Iterate the intended signatures by issuer/type; do not interpret “all stored signatures verify” as an effective authority policy or blindly treat third-party certifications as self-signatures. |
| `src/packet/signature/config.rs:267,331,337` | `sign_certification_third_party` hashes the subject/UID and signature trailer, then invokes an arbitrary `SigningKey` over the resulting digest. This permits certification by an agent-backed signer without importing the primary secret. |
| `src/packet/signature/types.rs:553` | `verify_third_party_certification` takes the exact subject, issuer and UID/tag. It supports certification revocations as well; validate the signature type and effective policy, not just a returned success. |
| `src/types/key_traits.rs:139`; `src/crypto/ed25519.rs:117` | `SigningKey::sign(password,hash,data)` is implementable by a handle to an external agent. In the certification caller, `data` is already the OpenPGP digest. Its `Password` argument is an interface artifact: the adapter never accepts/stores the primary passphrase and uses normal pinentry. Do not hash that digest a second time. |
| `src/composed/signed_key/shared.rs:118–126` | `as_unsigned` explicitly uses heuristics rather than cryptographic validation or newest-signature selection. Do not use it to derive trusted role/expiry/revocation decisions. |

Source links: [rPGP certificate verification](https://docs.rs/crate/pgp/0.21.0/source/src/composed/signed_key/public.rs), [certification construction](https://docs.rs/crate/pgp/0.21.0/source/src/packet/signature/config.rs), [signature verification](https://docs.rs/crate/pgp/0.21.0/source/src/packet/signature/types.rs), [signer trait](https://docs.rs/crate/pgp/0.21.0/source/src/types/key_traits.rs).

The application must evaluate valid self-signatures/issuer certifications, current key/UID flags, expirations and revocations explicitly. Accept only the chosen Ed25519/SHA-2 signing profile; a v4 fingerprint's SHA-1 identity calculation does not permit SHA-1 data signatures. Disable unsupported/lenient/draft features. Upstream reports repaired historical malformed-input/resource findings and an outstanding RSA side-channel concern; no RSA private signing/decryption is required by our current profile, and this is not a finding that the whole library is proven safe. [rPGP 0.21.0 security status](https://docs.rs/crate/pgp/0.21.0/source/docs/SECURITY_STATUS.md).

The Assuan adapter has two typed operations: raw grant bytes via the tested PureEdDSA inquiry, and an allowed OpenPGP digest/hash pair for certification/manifest signing. Use bounded percent-escaped Assuan framing/S-expression decoding, deadlines, cancellation and serialized `RESET → exact SIGKEY → signing input → PKSIGN` transactions. Validate the returned algorithm/key/signature and verify locally before use. Never expose arbitrary Assuan commands, keygrip selection, secret export/import or passphrase/cache control to a seat or MCP client. A failed transaction is reset/closed, not retried under a default key.

**Gap:** rPGP certification→agent→GnuPG verification has not been run; the Rust adapter has not been built. Before dependency adoption, check the locked transitive licenses/features, current advisories, bounded parser behavior and policy vectors, and exercise this exact Ed25519 certificate chain with rPGP and GnuPG. The recommendation meets the direct-license constraint; it does not claim these implementation gates are passed. Sequoia requires an explicit operator exception and backend decision if reconsidered.

## New-device enrollment, including an offline primary

An enrolled device may introduce a newcomer but cannot issue its own root-level grant. Primary access is required to complete enrollment; “existing device” does not mean “primary present.”

1. The newcomer generates an independent device key/certificate locally and proves possession. Its secret never leaves that host.
2. The operator creates a protected pairing ticket/QR with protocol/account fingerprint, introducer grant, invitation ULID, endpoint hints and a random 256-bit one-use capability; default expiry ten minutes. Store a verifier where possible. No ticket/capability in argv, public discovery, logs or MCP results.
3. Pin the account/introducer from the transferred ticket and open an authenticated encrypted pairing connection. Pairing admits only enrollment operations. Bind the invitation, new device certificate/raw key/ID, challenge, requested scopes and durable operation ULID to the newcomer's signature. Display the full account fingerprint and a comparison code on both ends.
4. Produce a bounded **primary-signing request** containing that exact proposal, expected previous account head/epoch and public certification input. If the primary is offline/unavailable, show “awaiting primary signature”; do not claim the device is enrolled, copy a secret, or ask for a primary password inside the app.
5. Transfer the request by the operator's protected channel to the machine with the primary. There the operator checks the fingerprint, newcomer and scopes, and authorizes the specific certification, `DeviceGrant` and next `AccountHead` through GnuPG/gpg-agent. Return only public certificates and signed records. A plain `gpg --detach-sign` packet is not a replacement for the raw grant signature.
6. The serialized account-authority writer verifies/commits the signed head and consumes the invitation under the same durable enrollment operation before installation acknowledgment. Reject a result based on a stale/conflicting head; reconcile/re-sign rather than publishing two heads at one epoch. An expired ticket requires a new pairing authorization even if an old request eventually returns signed.
7. The newcomer verifies/persists its expected primary, certificate chain, grant and highest head before entering normal transport. Retry by the same operation ID yields the same installed outcome, not a second device.

Possession of a ticket alone does not identify the intended physical device. Reject replay, key/root substitution, expiry and concurrent consume races. The account journal must coordinate online and offline signing; carrying a backup primary to another host does not create a second independent authority writer.

**Gap:** request/response bundle encoding, offline GUI/CLI import and the enrollment transaction/retry protocol need implementation tests. This specifies the pending state and authority boundary; it does not assert an existing offline-signing command.

## Revocation, epochs and partition behavior

Serialize account changes in one durable authority journal. Adding/revoking a device, changing its delegated authority or publishing/revoking a Google binding advances `account_epoch` and signs a linked `AccountHead` with the GPG primary. Reissue surviving device grants in that epoch. Device key replacement advances its device epoch; seat key replacement advances key epoch; class/destination/mute changes advance grant epoch.

Persist high-water marks before acknowledging or acting. Reject lower epochs; equal epoch with different signed content is a **fork**. Suspend affected new control effects and retain both proofs for operator resolution. Recovering a stale journal or using a primary backup must not reset those high-water marks.

Carry applicable OpenPGP key/certification revocations **and** application heads/grant revocations. The former invalidates the cryptographic identity chain; the latter fences terminal authority without necessarily retiring a GPG key used elsewhere. An operational device can immediately deny its own local seats, but cannot forge a primary-signed revocation of another device. Root-level revocation may remain pending until the primary is available.

Exchange signed current heads/certificate updates on contact, before new authenticated effects and on reconnect. Helpers may cache public proofs but cannot approve an update. A learned revocation invalidates active authenticated authority, queued control work and stale capabilities; hold blocked work with its reason and retain evidence. No backdated message or old certificate restores current permission.

Revoking here is **not globally instant**. Peers learn it on contact; an isolated receiver cannot know an unseen update. The 30-day device expiry bounds acceptance by a receiver that never receives revocation, assuming its clock/high-water state is intact. A 24-hour seat expiry alone cannot bound a stolen device that can issue fresh children. Do not publish private grants or host metadata while gossiping public authority.

**Gap:** clock rollback, lost high-water state, root-signed forks and offline revocation delivery require implementation handling. Historical signature validity and current control authority are separate decisions; there is no serverless proof of globally current revocation knowledge.

## GPG recovery and primary replacement

Recovery follows the operator's **existing GPG backup and revocation practice**. Preserve the matching public certificate, protected GPG secret backup, needed passphrase/token recovery and a separately protected revocation certificate. Restore/import through GnuPG on a trusted machine; the terminal sees the public certificate/agent availability and verifies the full pinned fingerprint. It never asks for primary secret bytes or imports them into Keychain.

Paper fallback is external **paperkey**, not the former 24-word application seed. Paperkey extracts secret-key material and needs the matching public key to reconstruct it; the operator performs that sensitive export/restore outside the terminal. A paper copy does not replace the normal backup, protection/passphrase or a tested restore. [Paperkey author's format and recovery explanation](https://www.jabberwocky.com/software/paperkey/).

After primary access is restored, recover the latest verified public head/journal high-water state from surviving pinned devices or backups. Authorize a new device, revoke lost ones, reconcile the account head and acquire fresh grants before remote control resumes. If the latest state cannot be recovered, show possession restored / authority state unresolved; never restart at epoch zero.

Recovery restores **identity only**. It does not resurrect seat processes, history, byte rings, harness session credentials, device/seat secrets or grants. Those have separate retained data/resume mechanisms. Public heads/rosters are backed up separately from primary custody.

A stolen primary or backup is not cured by importing the same key elsewhere. Root revocation/replacement creates a new account and requires independently trusted contact re-verification; an old-root cross-signature alone cannot prove the attacker was excluded. Email, Google and an account server cannot reset that root.

**Gap:** no real-primary backup, revocation or paperkey restore was exercised by I2. Lost public-state continuity and conflicting primary-authorized heads require operator resolution; possessing the key alone does not select the legitimate fork.

## Google binding

This is deferred out of the first prototype. The pinned GPG primary remains the identity throughout this flow. Use the system browser and a Desktop OAuth client, authorization code with **S256 PKCE**, and a listener bound to a literal loopback IP on an available port before opening the browser. No embedded webview or hosted account callback is required. Preserve the exact redirect URI. [Google desktop flow](https://developers.google.com/identity/protocols/oauth2/native-app), [RFC 8252](https://datatracker.ietf.org/doc/html/rfc8252).

1. The operator starts binding on a primary-signing host. Create independent random `state`, PKCE verifier and `nonce_random` (32 bytes). Keep a bounded expiring attempt with expected root, redirect, state and verifier in private memory. Cancellation/logic loss invalidates it.
2. Compute exactly the decided nonce: `hex_lower(SHA-256(raw_root_pubkey_32 || nonce_random_32))`. Send that string as `nonce` with `scope=openid email`, `response_type=code`, configured client ID, state and `code_challenge_method=S256`.
3. Accept only the matching loopback callback/path, one bounded response, no duplicate parameters, and exact constant-time state match. Exchange the code over HTTPS using the same redirect, client ID and PKCE verifier; a desktop client secret is not a confidential trust anchor.
4. Verify the ID token with Google's trusted discovery/JWKS: allowed signature algorithm/key, issuer, this app's audience, authorized party where applicable, expiry/issued time, exact nonce, stable `sub` and email claims. Set verified-email status only when `email_verified` is true. Provider identity is issuer plus subject; matching email strings never merge roots. [Google token verification](https://developers.google.com/identity/gsi/web/guides/verify-google-id-token), [Google OpenID Connect](https://developers.google.com/identity/openid-connect/openid-connect).
5. Root-sign the decided binding record below only after validation. `nonce_random` is unpadded base64url of the actual 32 bytes, `id_token_hash` is SHA-256 of the exact compact JWT bytes, and `issued_at` is this verified binding's time. Commit its digest in `AccountHead.current_binding_ids`; explicit revocation moves it to `revoked_binding_ids`. Store the proof bundle separately: record, root signature, original provider ID token and required public issuer/audience context. A hash alone does not let another verifier validate Google's signature.
6. Discard OAuth access/refresh tokens when not needed for another explicitly authorized feature; identity binding does not request offline API access. Do not put the code, verifier or bearer tokens in logs/rosters. Sharing the email/ID-token proof bundle requires the operator's chosen visibility; it is not automatically broadcast to global channels.

Decided record shape, with symbolic values rather than live credentials:

```json
{
  "provider": "google",
  "subject": "provider sub",
  "email": "operator-chosen email matching the verified token",
  "root_pubkey": "raw root public key encoded as lowercase hex",
  "nonce_random": "base64url of 32 per-login random bytes",
  "id_token_hash": "sha256 of exact provider JWT bytes",
  "issued_at": 0
}
```

The root-signed record remains valid until rotation/revocation under the operator's decision, with yearly rebinding scheduled. Its lifetime is **not** an extension of the provider JWT's `exp`: a live login token must not be accepted after expiration ([JWT expiry, RFC 7519, section 4.1.4](https://datatracker.ietf.org/doc/html/rfc7519#section-4.1.4)). Preserve “bound at time” evidence separately from a fresh provider-verification result. A first-time verifier needs actual trusted provider proof, not a sender-supplied JWK or the hash alone.

The yearly job can mark rebinding due in the background. A fresh nonce/browser authorization and explicit primary-signing operation through pinentry/cache may need the operator; do not substitute a refresh-token result that lacks this attempt's nonce. Device enrollment and GPG/paperkey recovery continue to use keys, not the Google account.

**Gap:** long-lived peer validation after ID-token expiry/provider key rotation is not established by the decided record. Retain historical observations as such; obtain fresh binding when current provider proof is required. The branded Desktop client/consent configuration and real loopback/token/rebinding flow remain untested and unprovisioned here.

## Signed seat roster and Git provenance

The provenance root is the **same OpenPGP certification chain**, not a new SSH allowed-signers authority: account primary certifies device, device certifies seat. The public `SeatRoster` is an append-only device-signed index of those certificates and their metadata, accompanied by the exact certification proofs. Verify both its application signature and the OpenPGP issuer chain.

Each public entry binds `seat_id, seat_pubkey, key_epoch, character_name, harness_kind, role, valid_from, valid_until?, status`, its certificate fingerprint and an opaque reference to the private grant. Status means current/retired/revoked key provenance, not harness activity. Preserve predecessor hashes and historical certificates after routine rotation. Do not export destinations, private swarm membership, `where`/host details, cwd, harness conversation IDs or tokens.

For external GPG tooling, export a public-only certificate bundle and an **OpenPGP detached signature of the public manifest** from that authorized device. `gpg --verify` can verify the manifest signature and, if per-seat signing is later approved, a seat's OpenPGP commit signature. It cannot by itself evaluate terminal scopes, grant epochs or the intended account→device→seat authorization. Verify issuer certifications explicitly against the pinned primary and current/historical policy; do not alter the user's trustdb or label any same-email key trusted. [GnuPG verification options](https://www.gnupg.org/documentation/manuals/gnupg/Operational-GPG-Commands.html).

```text
commit/manifest OpenPGP signature → exact seat/device certificate
                                  → issuer certification chain
                                  → pinned Boxy primary fingerprint
                                  + roster metadata and applicable app grant policy
```

A roster proves terminal-issued key attribution, not which model wrote a patch or whether it had publication permission. A claimed commit time, author string or hosted badge is not independent evidence of signing time or the full chain.

**Per-seat Git signing remains ASSUMED and default-off.** Boxy's standing author/committer identity and GPG signing key/policy remain unchanged. The chain makes such provenance possible; enabling it still requires an operator decision and a seat-bound signer that cannot expose another key. This task changes no Git configuration and makes no commit.

**Gap:** OpenPGP leaf-certificate/Git interoperability, time-of-compromise policy, public bundle publication and hosted verification remain untested. No claim that `gpg --verify` alone implements this application's delegated trust policy.

## Evidence and acceptance

Only `.bboxy/specs/identity.md` is written by I2. No operator primary was signed with, exported, changed or backed up; no Git/GPG configuration, key trust, enrollment, revocation, Keychain item or Google account was modified.

| Actual command/read | Result and limit |
|---|---|
| `date`, `git status --short`, task I, I2 board/ICQ addenda, [[vision]], [[README]], current identity/messaging specs | Read on 2026-10-21; worktree clean before this rewrite. Current signed class/ULID/where decisions applied here only. |
| `gpg --version`; `gpg-connect-agent 'GETINFO version' 'HELP SIGKEY' 'HELP SETHASH' 'HELP PKSIGN' /bye` | Exit 0: GnuPG/agent **2.5.24**, libgcrypt **1.12.4**; native help documents to-be-signed data for PureEdDSA via `SETHASH --inquire`. Public metadata only. |
| `gpg --with-colons --fingerprint --list-keys 9B07E849480C68328CA20684844ECC0587DE7AB0` | Exit 0: full fingerprint/selected UID and algorithm 22 confirmed; no secret export or signing. |
| In-memory downloads of official `pgp-0.21.0.crate`, `sequoia-openpgp-2.4.2.crate` and `sequoia-gpg-agent-0.6.2.crate`; source/manifest reads | Exit 0; exact versions, feature defaults, licenses and package source lines above checked. No dependency installed or added. |
| Disposable Python/GnuPG Assuan probe, independent Python `cryptography` Ed25519 verifier | Exit 0: fresh temporary-home algorithm-22 key, **187-byte** ASCII domain+canonical-object fixture, **32-byte** public key, **64-byte** signature; original input verifies, one-byte-changed input rejects. No secret exported. |
| Probe lifecycle | Fresh 0700 temporary home, explicit fixture socket/keygrip, `gpg --quick-generate-key` with empty passphrase for this fixture only; `SETHASH --inquire` supplied `TBSDATA`, then `PKSIGN`. Fixture-only `gpgconf --homedir <temporary-home> --kill gpg-agent` exited 0; temporary home removed. Operator agent/key untouched. |
| Initial probe driver | Exit 1: expected inquiry at `PKSIGN` rather than `SETHASH`. Corrected the driver; final probe passed. This is not an observed cryptographic failure. |

The probe validates one GnuPG/raw-signature path, not the product codec, passphrase/pinentry UI, smartcard, older agent, Rust integration or the full certificate/grant chain. Its payload uses only an ASCII JSON subset; it does not validate a complete JCS implementation. The disposable secret was created/removed by GnuPG without being exported or printed.

Document verification on 2026-10-21: `ls .bboxy/specs/identity.md && grep -c '^## ' .bboxy/specs/identity.md` exited 0 with **11** sections. Python checks passed for eight local wikilinks, one JSON block, balanced fences, whitespace, retained grant fields and 15 domain/nonce/custody/phase contract assertions; `git diff --check` exited 0. These are document checks, separate from product acceptance.

Implementation acceptance still requires:

- Raw grant/message and OpenPGP certificate vectors reject substituted roots/keys, bad issuer chains, unknown fields, stale/forked epochs, overbroad classes/scopes and expired parents.
- Primary-absent/cancelled/unsupported-agent cases install no authority; no operation reads primary secret bytes or uses a default key. Packaged macOS 14 leaf custody and normal pinentry/cache behavior are verified.
- Enrollment/offline replay, stale-head signing, ticket race/expiry and lost replies produce at most one authorized installed device.
- Revocation is enforced after learning it, while partition/clock/state-loss tests expose the actual expiry limits.
- External GPG/paperkey recovery restores the exact fingerprint and public-state continuity; missing seats/history/grants remain missing.
- Optional OAuth verifies the exact root-bound nonce/provider proof and keeps provider JWT expiry separate from durable binding.
- Public roster/GPG verification detects a substituted leaf/issuer, forged metadata and private-policy leakage without changing the standing Git signing policy.

**Gap:** these product acceptance checks have not run. I2 delivers the revised spec and the bounded disposable interoperability result, not implemented identity/account functionality.

— Nyx Copperwing <EXECUTOR>
