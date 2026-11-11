# bboxy450-keychain-macos

Read-only generic-password lookup for macOS usage providers. Every query sets
`kSecUseAuthenticationUIFail` and runs with the process's login-keychain user
interaction switched off for its duration, so neither the data-protection
authentication UI nor the login keychain's "wants to access key" dialog can
appear or block; an item another application owns reads as unavailable.
The caller runs lookup on its bounded IO worker and supplies a byte limit.
No credential mutation, unlock, refresh or subprocess API exists.

Pure query, status and byte-bound tests never call the operator's Keychain.
Live OS authorization and timing remain outside those tests and formal models.

Canonical source: https://github.com/bboxy450/bboxy450-term.
Credit policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md.
