---
id: 01M5QNE1QM9NGJ2BQT0V9TTJ7A
type: decision
number: 281
title: "identity custody is a cross-OS facade, bboxy450-keychain with per-OS backends, macOS Keychain first; an automation identity for unattended nights lives in a file-backed encrypted store on every OS, is a separate key with its own grant, and signs as automation, never as the operator"
status: decided
decided: "2026-10-24"
operator:
  - "keychain should be also supporting linux, windows and shit, so you know"
links:
  - { to: "[[id1-seat-chain]]", rel: decides }
  - { to: "[[d46-cross-os-facades]]", rel: builds-on }
  - { to: "[[d45-desktop-notifications]]", rel: builds-on }
  - { to: "[[d280-keychain-regrant]]", rel: builds-on }
  - { to: "[[d17-device-key-custody]]", rel: amends }
  - { to: "[[e2e-acceptance]]", rel: constrains }
  - { to: "[[cross-os-facade]]", rel: invariant }
created: "2026-10-24T19:15:36Z"
updated: "2026-10-24T19:23:03Z"
---
# Decision 281: custody is a facade; the automation identity has its own store

Decided by the operator on 2026-10-24 ("keychain should be also supporting linux, windows and shit, so you know"), folded with the Oracle's unattended-night finding: a locked Keychain holds signing, so an e2e night at 03:00 on a locked display stalls on custody.

**Alternatives.** (a) Keep custody macOS-only in `bbt-mux` with `security-framework`. (b) The pattern of #45 and #46: a facade `bboxy450-keychain` with per-OS backends, `-macos` (the Keychain, real, with the designated-requirement ACL of #280), `-linux` (Secret Service and keyring) and `-windows` (Credential Manager) as DO-RESEARCH-BEFORE-IMPLEMENTING dummies answering `Unsupported` by name, and `-file`, a file-backed encrypted store that works on every OS and is **real now**; and for unattended runs an **automation identity**: a separate device-class key with its own `DeviceGrant` from the root, scoped by the operator at provisioning (the e2e writes, `ctl` under the operator principal, nothing else), stored in the `-file` backend under a key file the operator provisions once (mode 0600, under the support directory), used only when the app runs with `--unattended`, and carried as `issued_by: automation` on everything it signs so the provenance never confuses it with the operator's hand. (c) Unlock the operator's Keychain item from the harness.

**Choice.** (b).

**Reason.** Custody is OS-specific by nature and #46 says every OS-specific thing is a facade; the exception for `bbt-mux`'s `core-foundation` and `security-framework` in `scripts/cross-os.sh` retires when `-macos` takes them. The automation identity is the honest answer to unattended nights: **a store a process can open by itself is a store any same-user process can open** (the same truth as #260 and the plugin trust record), so the key in it is a *separate* key with a *scoped* grant and its own label, never the operator's device key and never a copy of it; a night that runs on it leaves records that say automation signed them. (c) would make the operator's key self-opening, which is the one thing custody exists to prevent.

**Rules.** [[id1-seat-chain]] §5: the facade API (`open(backend)`, `get`, `put`, `remove`, `acl`, each with the one-shot interaction act of #280), the backends and their states, the automation identity's grant scope and label, `--unattended`. The five crates lead the layering order beside the notification family and depend on nothing of ours; licence rows dual, ours; brand names. The e2e night (`e2e-acceptance` §4) runs under the automation identity when the display is locked and says so in every record it writes. Rejected: a self-opening operator key; copying the device key into a file; a macOS-only custody.

Owner: Corvin Duskmantle <ARCHITECT>.

**Confirmed simpler (operator, same day, verbatim: "each agent can create its own sub gpg key from users key, or can have just a key without a passphrase, whatever design is simpler!").** The automation device key is a passphrase-less key file, mode 0600, root-granted and revocable by one signature; every seat already has its own generation leaf key from the device chain. The `-file` backend implies no passphrase and no key-wrapping key: the file is the secret, protected by its mode, the grant's scope and the label. Rejected: GPG subkeys per agent; a second identity system beside the Ed25519 device grants.
