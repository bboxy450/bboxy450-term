---
id: "01M5JQX1D88678E3NFRRRBFMTC"
type: "decision"
number: 17
title: "Device-key custody has three tiers"
status: "amended"
decided: "2026-10-22"
operator: []
links:
  - { to: "[[2026-10-22-architecture-round-1#17]]", rel: "supersedes-as-index" }
created: "2026-10-22T21:22:33Z"
updated: "2026-10-24T19:13:30Z"
---
# Decision 17: Device-key custody has three tiers

**Device-key custody has three tiers (Architect, from ID1a evidence).**
The Data Protection Keychain returns `errSecMissingEntitlement` (-34018)
to an unsigned binary; it needs `keychain-access-groups` on a signed app
identity. So: tier 1 Data Protection Keychain in the shipped, signed app
(lands with packaging, PKG1); tier 2 the login keychain with the same
attributes for ad-hoc and dev builds, shown as "dev custody"; tier 3
memory for tests; one `DeviceKeyStore` trait, fall from 1 to 2 on -34018,
never a plaintext file. ID1a acceptance 4 amended. Team known the
same day: Wild Honey on the Porch, LLC (UQB3368A84), Apple Development
and Distribution certificates on this Mac, Developer ID Application
still pending, so tier 1 is verifiable locally with a Development-signed
bundle and distribution waits on that one certificate. Rejected: blocking ID1 on
packaging (velocity); a plaintext or sqlite secret file (identity.md
forbids it); requiring a team ID before any identity work.

**Amended by [[d280-keychain-regrant]] (2026-10-24):** the no-dialog default stays; one interactive access is permitted on an explicit operator act (`ctl identity unlock`, Settings › Identity › Unlock); new items carry an ACL keyed to the team-signed designated requirement so rebuilds keep access; legacy items migrate during that one access.
