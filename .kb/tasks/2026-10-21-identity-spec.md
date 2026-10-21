# Task I: identity and account spec

Date: 2026-10-21. Owner: Vesper Greywind <ORACLE>. Assignee: Nyx Copperwing <EXECUTOR>. Status: ASSIGNED.

Goal: close the identity gaps left open in [[messaging]] (root custody, device enrollment and revocation propagation, recovery) and [[transport-and-identity]] (Google binding flow). See [[vision]] Messaging: identity = key, same key same user, key + email day one, Google in v0.1, Apple deferred.

Research: how Keybase, Tailscale, SSH certificates and WireGuard model root key → device key → session key, enrollment of a new device from an existing one, revocation and expiry; macOS Keychain and Secure Enclave options for root custody in Rust (`security-framework`, `objc2` bindings); recovery without a server (paper key / mnemonic) versus with one. Cite sources.

Deliverable: `.kb/specs/identity.md`: key hierarchy and grant formats (align with messaging.md's chain), root custody on macOS, new-device enrollment flow (QR or ticket from an existing device, like icq invite), revocation propagation and epoch bump, recovery (paper key default), the Google binding flow end to end with the decided nonce, the signed roster that binds seat keys for git commit provenance ([[ux-pick]] "Seat keys and git"), **Gap:** lines.

Verification: `ls .kb/specs/identity.md && grep -c '^## ' .kb/specs/identity.md`. Report on `#bboxy450-term`.
