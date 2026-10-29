---
id: 01M5JQE3ZY6KYN1P0NA2M75X0E
number: 140
type: invariant
title: every commit on main carries a pinned verified signature
status: active
check: python3 -I scripts/verify-signatures.py "${SIGNED_RANGE:-HEAD}"
cost: an unsigned commit breaks the chain from artifact to key; releases inherit the gap
links:
  - { to: "[[provenance]]", rel: constrains }
  - { to: "[[identity]]", rel: constrains }
created: 2026-10-22T21:14:24Z
updated: "2026-10-29T22:09:42Z"
---
# Every commit on main carries a pinned verified signature

**Property (decision 220; operator amendment, 2026-10-29).** Every commit in
handed-off HEAD history has a cryptographically valid signature and the matching
exact author identity from one of these approved pins:

- Boxy `<bboxy450@gmail.com>` — `9B07E849480C68328CA20684844ECC0587DE7AB0`.
- Maksim Soltan `<code@maksim.sh>` — `B9ABDC578390946B71B6C20C3526057098BAC1AD`.

The operator explicitly approved Maksim alongside Boxy for new source commits
and author landing while integrating PR #76. The former exact exception for
`618df56faccffce46dfec3d1d525b8d115b5d35a` is now covered by Maksim's approved
pin. A key cannot sign for the other approved author; unapproved identities
remain refused. Landing retains its configured author/key ownership checks and
permits the approved source publishers `bboxy450` and `Gonzih`. Release signing
identities are separate from this source-commit policy.

The provenance chain is signed commits → signed tags → detached signatures on
published crates ([[provenance]], [[open-source]] §3); this verifies its first link.

**Check.** `python3 -I scripts/verify-signatures.py HEAD` on the clean candidate.
Exit 0 proves the complete requested commit set; exit 1 refuses missing proof.
The verifier uses bundled public keys in a fresh private keyring, with no agent,
network, operator private-key access or global trust/config mutation. A valid
`U` is accepted only under the explicit primary fingerprint and author pins;
an unapproved `G` is refused. Empty, unsigned, bad, unknown, revoked and expired
inputs refuse. Replacement objects and shallow history cannot hide ancestors.
Public-key source, fingerprint and SHA256 provenance is in
`scripts/verification-keys/provenance.json`. `make test-signatures` runs the real
history proof and throwaway signed/unsigned/invalid identity controls unattended.

**Cost of a violation.** An unsigned commit on main is an artifact nobody can
trace to a key; everything built on it, including a crate release, inherits
the gap. Fix the config, never paper over with `--no-gpg-sign`.

**Evidence.** `398 of 398 commits G at a9ab5c1` (Architect, 2026-10-22).

**Tiered by range.** Presubmit retains its existing explicit
`SIGNED_RANGE=<base>..HEAD` cut check. Normal Make CI clears that variable and
checks full HEAD, as do the night and watcher without a range. Both use the same
pinned policy. The full requested set is collected first, verified with two
bounded readers and checked for exact result-set equality; no ancestor is skipped.

Owner: Corvin Duskmantle <ARCHITECT>.
