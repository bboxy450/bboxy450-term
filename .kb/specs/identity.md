# Identity and account

2026-10-21 · Nyx Copperwing <EXECUTOR> · task I · specification for Oracle review.

[[vision]] · [[messaging]] · [[mux-daemon]] · [[mcp-control-plane]] · [[harness-adapters]] · [[transport-and-identity]] · [[ux-pick]] · [[ux-prototype]]

## Identity and authority

An account is its **Ed25519 root public key**, encoded as the lowercase hex of the raw 32 bytes. Same root, same account; a different root is a different account. Email, Google subject, character name and avatar are metadata, never alternate roots or recovery credentials. Day one accepts an operator-entered email as self-asserted; Google adds a separately graded binding in v0.1. Apple stays deferred under [[vision]].

Use three signing levels, matching [[messaging]]: root → device → seat. Generate independent keys from the OS cryptographic RNG; no child derives another child's secret. Transport session encryption keys are separate from these long-lived authentication keys. A signed role proves terminal-issued authority, not a model's identity or permission to execute peer text.

The root resides on one **controller Mac** by default. Other enrolled devices receive their own device grant and never receive the root seed. Only an authenticated operator action can use the root for enrollment, device authority changes, recovery or Google binding. Oracle can exercise its delegated seat controls; no MCP role gets root export, arbitrary root signing or account recovery.

Review lens: key possession, delegated authority, rollback/expiry, recovery and provider/provenance evidence. Assumes an uncompromised root and the stated OS custody boundary. A stolen root or an unrestricted process inside that boundary breaks the account authority assumption.

## Reference mechanisms

These are reference mechanisms, not dependencies or equivalent identity models:

| System / primary source | Observed mechanism | Decision here |
|---|---|---|
| [Keybase per-user keys](https://book.keybase.io/docs/teams/puk) | Device-private keys and paper keys appear in a signature chain; a per-user secret is re-encrypted for members and rotated when a device is revoked. | Keep device secrets local, record public membership changes and provide paper recovery. Do not silently give every device our root secret. |
| [Tailscale Tailnet Lock](https://tailscale.com/docs/features/tailnet-lock) | Trusted signing nodes authenticate node keys distributed through the coordination service; its key authority tracks trusted signers. | Discovery is not authority. A helper may distribute signed grants/heads but cannot approve a device. This is not a claim that Tailscale itself uses our root/device/seat chain. |
| [OpenSSH certificates and KRLs](https://man.openbsd.org/ssh-keygen) | A CA signs public keys with identity/principals, options and validity; revocations are represented by lists/KRLs. | Include issuer, subject, constrained scope, serial/epoch and finite validity. A signed revocation still needs distribution. Our grants are not SSH certificates. |
| [WireGuard protocol](https://www.wireguard.com/protocol/) | Static peer authentication and ephemeral handshakes produce rotating traffic keys. It does not supply this application's account/seat authority chain. | Separate authenticated device identity from session encryption and from enrollment policy; a working encrypted tunnel alone does not authorize a seat role. |

## Signed formats and verification

Version 1 signs bounded typed payloads using strict Ed25519 verification and a distinct domain per record type. Encode payload objects with [JCS, RFC 8785](https://www.rfc-editor.org/rfc/rfc8785); reject duplicate/unknown fields, malformed keys, unsupported versions and out-of-range values before admission. Sign `UTF8("bboxy450-term/<type>/v1\0") || JCS(payload)`. The record ID is SHA-256 of those bytes; the detached signature and ID are outside the payload. This is the grant/identity encoding; it does not change [[messaging]]'s separately specified signed message tuple.

Public keys are lowercase 64-character hex; signatures are unpadded base64url of 64 bytes. IDs use their canonical types; counters/epochs are bounded unsigned decimal strings to preserve exactness across JSON implementations. Times are integer Unix seconds in the JSON safe-integer range. Scope lists have deterministic ordering and no duplicate elements. No record contains a private key, bearer ticket or provider access/refresh token.

| Record / signer | Required payload fields and meaning |
|---|---|
| `AccountHead` / root | `version, root_pubkey, account_epoch, previous_head_id, active_devices, revoked_devices, issued_at`. Each active entry names device ID, device public key and device epoch. A root-signed membership head, not a helper's assertion. |
| `DeviceGrant` / root | `version, root_pubkey, account_epoch, device_id, device_pubkey, device_epoch, serial, scopes, role_ceiling, not_before, expires_at`. Scopes distinguish device transport, issuing seat grants and publishing provenance. It cannot delegate root custody/recovery. |
| `SeatGrant` / owning device | `version, root_pubkey, device_grant_id, device_id, device_epoch, seat_id, seat_pubkey, key_epoch, grant_epoch, previous_grant_id, cohort_id, harness_kind, role, destinations, control_scopes, muted, not_before, expires_at`. Destination permissions and mute match [[messaging]]; scope/role cannot exceed the parent. |
| `SeatRoster` / authorized device | `version, root_pubkey, device_grant_id, device_epoch, roster_sequence, previous_roster_id, issued_at, entries`. Public provenance entries are defined below; no private cohort/destination policy is exported. |
| `GoogleBinding` / root | The decided fields `provider, subject, email, root_pubkey, nonce_random, id_token_hash, issued_at`; separate root signature and provider proof bundle. This does not grant device/seat permissions. |

Initial validity policy: device grants last at most **30 days**, seat grants at most **24 hours** and never beyond their parent. These are operator-configurable finite defaults, not measured capacities or provider rules. Root-approved renewal can keep the same device key; the owning device renews seat grants within its current ceiling. No issuer can backdate or extend a child beyond its own valid interval. Expiry blocks new authenticated effects; it does not kill a harness or delete stored messages/history.

A verifier pins the expected root, verifies every signature/domain and parent reference, matches the subject key to the actual authenticated peer, checks membership/device epochs, validity, scope and role eligibility, then applies current receiver policy. Codex/other harnesses remain Executor; Claude may be Oracle or explicitly Creativity. A signature cannot override that harness policy.

Stable seat ID and seat signing key can survive harness resume; process-bound MCP/native capabilities do not. Rotate a leaf key explicitly on compromise or replacement, recording a new key epoch. Every new process generation must reacquire its local capability/attachment under [[mux-daemon]] and [[mcp-control-plane]], even when it reuses the durable seat key.

Ordinary renewal or a mute-only update is not retroactive destruction of signed records. Admission/dispatch still requires current authority, clamped to the original signed grant: an old Oracle assertion cannot regain precedence after demotion. A still-authorized unchanged key may present its renewed grant alongside an immutable pending message; never rewrite a previously published envelope under its old ID. A revoked/replaced key cannot authorize new effects merely by claiming an old timestamp.

**Gap:** canonicalization/signature test vectors, record limits and the exact current-grant proof exchange must be implemented together with the messaging wire format. These formats are decisions, not an existing interoperable codec.

## Root custody on macOS

Keep the root's 32-byte seed as a **non-synchronizing Data Protection Keychain generic-password item**, scoped to our signed application, accessible when unlocked on this device, with operator user presence required. The GUI performs a specific root operation after displaying its purpose; the root is loaded only for that operation and owned secret buffers are cleared afterward. Root export/recovery display is an operator GUI action, never an MCP tool. This does not promise elimination of every process-memory copy or protection from a compromised application/OS.

[Apple's Secure Enclave documentation](https://developer.apple.com/documentation/security/protecting-keys-with-the-secure-enclave) describes generated P-256 keys and prohibits importing preexisting private keys. The decided Ed25519 root and recovery of the same seed therefore use software Keychain custody. A Secure Enclave P-256 key could protect a future wrapping layer; it would not make Ed25519 signing itself enclave-resident. No such wrapper is required in v0.1.

Set `kSecUseDataProtectionKeychain=true`, non-synchronizing storage, and `SecAccessControl` with `WhenUnlockedThisDeviceOnly + userPresence` explicitly; do not assume a convenience generic-password call selects this backend. Do not use a biometric-current-set-only policy that makes a changed biometric enrollment the sole recovery path. Device and seat seeds use separately scoped non-synchronizing Keychain items usable by the user-session logic process after first unlock; they need background signing without a root-presence prompt.

[Apple TN3137](https://developer.apple.com/documentation/technotes/tn3137-on-mac-keychains) distinguishes file-based and Data Protection keychains and limits the latter to user login contexts. Our macOS logic is a per-user service in that context, not a system LaunchDaemon. A locked/unavailable store returns unavailable and holds signing work; no plaintext-file fallback. Already loaded operational keys may continue signing while the session locks; this is not a claim that lock revokes every in-memory key.

| Rust source inspected locally | Mechanism / implementation choice |
|---|---|
| `security-framework-3.7.0/src/access_control.rs:31,56` | Protection modes and `SecAccessControl::create_with_protection`. |
| `security-framework-3.7.0/src/passwords.rs:33`; `src/passwords_options.rs:124,160` | Generic-password options, explicit access control and synchronization selection. |
| `security-framework-3.7.0/src/passwords_options.rs:202` | Arbitrary query insertion is crate-private; do not assume this high-level options API exposes every Data Protection attribute. |
| `security-framework-3.7.0/src/key.rs:328` | Software versus Secure Enclave generation; enclave is constrained to its EC key type. |
| `objc2-security-0.3.2/src/generated/SecItem.rs:1387,1526,1584` | Typed bindings for Data Protection selection, `SecItemCopyMatching` and `SecItemAdd`. Use explicit dictionaries through this narrow macOS storage module when the higher-level wrapper cannot express the attributes. |
| `security-framework-sys-2.17.0/src/access_control.rs:11` | `kSecAccessControlUserPresence` flag. |

Local crate root: `/Users/feral/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`. These are source observations, not a Keychain test. Keep the unsafe Core Foundation boundary small and do not interchange unrelated Rust wrapper types by assumption.

**Gap:** signed Developer ID packaging, access-group/entitlement requirements, fresh/upgrade Keychain behavior and user-presence prompts must be tested on the macOS 14 floor and current macOS. Non-macOS device custody needs its platform backend; root material must not be copied to a remote cloud host to avoid that work.

## New-device enrollment

Only a root-custody controller can complete a new root-signed device grant. Another enrolled device may help connect the parties, but cannot promote itself into an unrestricted issuer or transfer a root it does not hold. If the controller is lost, use paper recovery first.

1. The new device generates its device key locally and proves possession over a pending enrollment request. Its private key never leaves it.
2. On the existing controller, the operator creates a short-lived invitation. QR or protected ticket contains protocol/account ID, controller key/grant, invitation ID, endpoint hints and a random 256-bit one-use capability. Default expiry is ten minutes. Store only the capability verifier where possible; never put the ticket in argv, logs, MCP results or a public discovery record.
3. The new device pins the root/controller from the operator-transferred invitation and opens an authenticated encrypted pairing connection. A pairing capability admits only enrollment messages, not ordinary mux/MCP control. Helpers can carry bytes; they cannot approve the device.
4. The request binds invitation ID, new device public key, nonce/challenge and desired label to the new key's signature. The controller shows the full fingerprint and a short comparison code on both devices. The operator verifies the intended pair through that existing physical/private channel and approves the displayed scope.
5. Under a durable enrollment operation ID, consume the invitation once, root-sign the constrained `DeviceGrant` and updated `AccountHead`, and commit the new head before acknowledging. Root user presence covers this specific enrollment, not a general “sign whatever arrives” session.
6. Return the grant chain/head over the authenticated connection. The new device persists its key, expected root, grant and highest head, acknowledges installation, then joins normal device transport. Retry by original enrollment ID to obtain the same outcome; a lost reply must not create a second device.

A captured ticket is not proof that the requester is the intended physical device; possession proof plus operator comparison is necessary. Replayed/expired tickets, changed public keys, mismatched roots and repeated consume races reject explicitly. Ticket delivery may use a protected file/SSH stdin instead of scanning; this changes transport, not the approval or proof.

## Revocation, epochs and partition behavior

The root controller serializes membership changes in a durable journal. Adding/revoking a device or changing its authority publishes a higher `account_epoch` and root-signed `AccountHead` linked to its predecessor. Reissue grants for remaining authorized devices in the new epoch. Device key replacement also advances that device's epoch. A device advances a seat's grant epoch for role/destination/mute changes and its key epoch for key replacement.

On accepting a head/grant, persist its high-water mark before acknowledging or acting. Reject lower epochs; an equal epoch with different signed content is a **fork**, not “last packet wins.” Suspend new affected control effects and show the conflicting proofs for operator resolution. The controller must not restore a stale private journal and issue an independently competing head.

Gossip root-signed public heads/revocations over authenticated device links and contact discovery; a configured untrusted store may cache public proofs. Exchange current heads before new sessions/effects and on reconnect. Never publish private destination grants or invitation secrets as public discovery. A learned revocation invalidates active transport/application authority, queued control work and stale local capabilities; terminate/refuse their authenticated effects without deleting retained evidence.

A disconnected receiver cannot know an unseen revocation. Persist highest-seen state and require fresh, unexpired parent/child grants for new effects. Our 30-day parent default bounds how long a stolen device can continue obtaining acceptance from an isolated verifier that never learns the new head; a 24-hour seat grant alone does not bound it because that stolen device can issue more children until its parent expires. Receivers need a trustworthy local clock; extending a child cannot extend the parent.

When expiry/revocation blocks pending messaging, hold it visibly with its reason under [[messaging]]. No automatic approval downgrade, key substitution or timestamp-based resurrection. Existing stored signatures remain historical evidence, separate from permission to inject now.

**Gap:** no serverless gossip scheme can prove globally current revocation knowledge during partition. Clock rollback, lost high-water state and root-signed forks need implementation handling/tests; “immediate revocation everywhere” is not promised.

## Paper recovery and root replacement

Default recovery is a paper backup of the **same root seed**, not email/password reset. Encode the 32-byte random seed as 24 English BIP-39 words with checksum, labeled `bboxy-root-v1`, root public fingerprint and explicit format version. Here the words encode the raw entropy bytes; do **not** run the mnemonic through the wallet PBKDF2 seed derivation. [BIP-39](https://github.com/bitcoin/bips/blob/master/bip-0039.mediawiki) specifies both the 256-bit/24-word encoding and a separate seed derivation; the format label prevents confusing those operations.

Creation/reveal occurs only in the authenticated GUI. Require an operator re-entry check before claiming backup completion. No default clipboard export, agent-readable recovery file, screen log or remote upload. The paper's confidentiality is the root's confidentiality. Keep a backup of public signed membership/head/roster state separately; it is not a substitute for the seed.

To recover: install on a trusted Mac, enter the words in that GUI, validate checksum/format and recomputed fingerprint, recreate the same protected root item, then obtain the latest verified membership state from a surviving pinned device or public-state backup. Approve a new controller device and revoke lost devices with a higher epoch; publish/reconcile that head before resuming remote control. A surviving old controller must relinquish root custody through an operator-managed transfer; restoring the seed must not silently create two root writers.

Paper restores possession of a lost root. It does not remove an attacker who copied that root/paper, undo prior disclosure, recover a missing conversation, or prove which of two root-authorized forks is legitimate. If the root itself is compromised, changing to a fresh root creates a new account; an old-root cross-signature alone cannot prove the attacker was excluded. Existing contacts need an independently trusted re-verification.

A server could retain public heads or an end-to-end encrypted backup, improving availability; it still cannot recover the root without the recovery secret or another authority. No account server, escrow service or email-only reset is part of v0.1.

**Gap:** a paper seed alone contains no up-to-date membership high-water mark. If all devices/public-state backups are lost and peers cannot provide the latest signed head, possession is recovered but safe automatic membership continuation is unproven. Require state recovery/operator fork resolution; do not reset the epoch to zero.

## Google binding

The root remains the identity throughout this flow. Use the system browser and a Desktop OAuth client, authorization code with **S256 PKCE**, and a listener bound to a literal loopback IP on an available port before opening the browser. No embedded webview or hosted account callback is required. Preserve the exact redirect URI. [Google desktop flow](https://developers.google.com/identity/protocols/oauth2/native-app), [RFC 8252](https://datatracker.ietf.org/doc/html/rfc8252).

1. The operator starts binding on the root controller. Create independent random `state`, PKCE verifier and `nonce_random` (32 bytes). Keep a bounded expiring attempt with expected root, redirect, state and verifier in private memory. Cancellation/logic loss invalidates it.
2. Compute exactly the decided nonce: `hex_lower(SHA-256(raw_root_pubkey_32 || nonce_random_32))`. Send that string as `nonce` with `scope=openid email`, `response_type=code`, configured client ID, state and `code_challenge_method=S256`.
3. Accept only the matching loopback callback/path, one bounded response, no duplicate parameters, and exact constant-time state match. Exchange the code over HTTPS using the same redirect, client ID and PKCE verifier; a desktop client secret is not a confidential trust anchor.
4. Verify the ID token with Google's trusted discovery/JWKS: allowed signature algorithm/key, issuer, this app's audience, authorized party where applicable, expiry/issued time, exact nonce, stable `sub` and email claims. Set verified-email status only when `email_verified` is true. Provider identity is issuer plus subject; matching email strings never merge roots. [Google token verification](https://developers.google.com/identity/gsi/web/guides/verify-google-id-token), [Google OpenID Connect](https://developers.google.com/identity/openid-connect/openid-connect).
5. Root-sign the decided binding record below only after validation. `nonce_random` is unpadded base64url of the actual 32 bytes, `id_token_hash` is SHA-256 of the exact compact JWT bytes, and `issued_at` is this verified binding's time. Store the proof bundle separately: record, root signature, original provider ID token and required public issuer/audience context. A hash alone does not let another verifier validate Google's signature.
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

The root-signed record remains valid until rotation/revocation under the operator's decision, with yearly rebinding scheduled. Its lifetime is **not** an extension of the provider JWT's `exp`: a live login token must not be accepted after expiration ([JWT expiry, RFC 7519 `4.1.4](https://datatracker.ietf.org/doc/html/rfc7519#section-4.1.4)). Preserve “bound at time” evidence separately from a fresh provider-verification result. A first-time verifier needs actual trusted provider proof, not a sender-supplied JWK or the hash alone.

The yearly job can mark rebinding due in the background. A fresh nonce/browser authorization and root user-presence operation may need the operator; do not substitute a refresh-token result that lacks this attempt's nonce. Device enrollment and paper recovery continue to use keys, not the Google account.

**Gap:** long-lived peer validation after ID-token expiry/provider key rotation is not established by the decided record. Retain historical observations as such; obtain fresh binding when current provider proof is required. The branded Desktop client/consent configuration and real loopback/token/rebinding flow remain untested and unprovisioned here.

## Signed seat roster and git provenance

Maintain an append-only signed roster history per issuing device, with its root-signed `DeviceGrant` authorizing provenance issuance. Each public entry binds `seat_id, seat_pubkey, key_epoch, character_name, harness_kind, role, valid_from, valid_until?, status` plus a reference to its private seat grant. Status is a key/provenance status (current/retired/revoked), not a new harness activity state. Sign the manifest with the authorized device key and include its root proof and predecessor hash.

Do not export the private seat grant's channels/DMs, private cohort membership, harness conversation ID, cwd, tokens or host contact details in a public commit roster. The device's public provenance assertion binds the leaf key/name to the root; it is not a redacted runtime grant signature. A verifier pins the root, verifies device authority/epochs and the roster signature, finds the exact leaf key and checks key status. Keep historical signed entries after rotation so “Boxy's seat Nyx Copperwing” can be resolved for older commits.

Per-seat Git signing can use the leaf's `ssh-ed25519` public representation and SSHSIG's `git` namespace through a seat-bound signer. Signing access must be restricted to that seat and operator-approved purpose; no root/device private key is offered as the seat's Git key. This is the integration shape if that signing mode is approved, not a signer implemented by this document.

[Git's SSH signing configuration](https://git-scm.com/docs/git-config#Documentation/git-config.txt-gpgsshallowedSignersFile) supports allowed-signers files, validity windows and a separate revocation file. Generate a verification file only from verified root-bound roster entries. Git's revocation setting treats listed keys as untrusted even for old signatures; retiring a key for routine rotation and revoking a compromised key are distinct policies. A commit's author email, trailer and claimed timestamp alone cannot prove who held the key or when a compromise occurred.

```text
Provenance verification:
commit signature → exact seat public key → signed roster entry
                 → device provenance authority → pinned account root
```

A roster proves terminal-issued key attribution, not that a particular model wrote the patch, followed its role or had permission to publish it. Do not treat a GitHub badge or a name string as proof of the full chain.

**Gap:** [[ux-pick]] “Seat keys and git” is explicitly ASSUMED/operator-to-confirm; [[ux-prototype]] adds no contrary signing decision. This machine's standing Boxy identity and signed GPG commit policy remain in force. This task changes no Git config/author/signing key and makes no commits. Operator selection of per-seat signing versus operator signing plus seat provenance/trailers, signer custody and hosted verification integration remains open.

## Evidence and acceptance

Only `.kb/specs/identity.md` is written by task I. No keys/tickets were generated, no credential values or Keychain entries read, no OAuth login/client registration, enrollment, revocation, recovery, Git configuration or publication performed.

| Evidence command/read | Actual result / limit |
|---|---|
| Task I, current Git state, [[messaging]] grant/nonce decision, [[vision]], [[transport-and-identity]], [[ux-pick]], [[ux-prototype]] | Read locally; accepted decisions retained. |
| Local Rust source reads / symbol searches at the paths above | Exit 0 for inspected source; generated API availability is not behavior verification. |
| `git --version`; `ssh -V` | Exit 0: Git 2.54.0 (Apple Git-157), OpenSSH 10.3p1. No signing invocation. |
| Primary web sources linked above, plus Apple's documentation JSON payloads | Fetched on 2026-10-21; Apple HTML was JavaScript-only, JSON supplied the actual Enclave/Keychain text. Old Tailscale KB URL and OpenSSH raw certificate URL failed; current Tailscale docs and OpenBSD manual supplied those mechanisms. |
| Required document check | `ls .kb/specs/identity.md && grep -c '^## ' .kb/specs/identity.md`; execute before handoff. |

Implementation acceptance, not checks passed by this specification:

- Grant/signature vectors reject substituted roots, invalid signatures, child scope escalation, stale/forked epochs, expired parents and live authority from retired keys.
- Packaged macOS 14 app stores/loads exactly the intended Keychain items; locked store/cancelled presence cannot fall back to plaintext or an unrestricted root-signing session.
- Enrollment withstands ticket replay/race, substituted device/root and lost replies; retry yields one installed device/grant.
- Revocation fences active/queued effects after learning a new head; partition tests expose the configured expiry window and clock/state-loss limitations.
- Paper words reproduce the exact root, reject checksum/format mistakes and require public-state recovery; missing state and compromised-root cases are not mislabeled successful recovery.
- OAuth tests cover state/redirect/nonce mismatch, wrong audience/issuer/key, expired JWT, unverified email, reused callback, missing original proof and yearly rebinding.
- Roster verification resolves historical exact keys while detecting forged names, changed manifests, private-policy leakage and compromised-key revocation.

**Gap:** no runtime or cryptographic interoperability test, macOS 14 packaging/Keychain check, OAuth round trip, multi-device/partition exercise or paper restore was run. These remain implementation evidence requirements, not a claim of completed functionality.

— Nyx Copperwing <EXECUTOR>

