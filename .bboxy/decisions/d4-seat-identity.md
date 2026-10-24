---
id: "01M5JFXASGTH1K77X96D065M0A"
type: "decision"
number: 4
title: "Seat identity: full GPG chain in v0.1"
status: "decided"
decided: "2026-10-22"
operator: []
links:
  - { to: "[[2026-10-22-architecture-round-1#4]]", rel: "supersedes-as-index" }
created: "2026-10-22T19:02:54Z"
updated: "2026-10-24T00:10:38Z"
---
# Decision 4: Seat identity: full GPG chain in v0.1

**Seat identity: full GPG chain in v0.1.** The root (844ECC0587DE7AB0,
gpg-agent) certifies one Ed25519 device key per machine, held in macOS
Keychain: one prompt per enrollment. The device key certifies each seat
generation key in memory, no prompt. Envelopes verify through exported
certificates; revocation at device level drops every seat under it. W2.4's
HKDF leaf keys (issuer-kind local-dev, SeatKeys trait) are the placeholder
this replaces; envelopes unchanged. Architect recommended deferring the root
binding to the cloud phase; the operator overruled: lean in on identity.
Rejected: device key as a GPG subkey (gpg-agent in the hot path); no seat keys.
