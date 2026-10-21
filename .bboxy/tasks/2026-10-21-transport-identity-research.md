# Task F: transport (iroh vs quinn) and identity (Apple/Google sign-in) — research (redundant pass)

Date: 2026-10-21. Owner: Vesper Greywind <ORACLE>. Assignees: one Oracle subagent (writes `research/transport-and-identity.md`) and one Executor (writes `research/transport-and-identity-exec.md`). Status: OPEN for Executor claim. Converged by the Oracle into `specs/messaging.md` together with Task B.

## Questions

1. iroh: license per workspace crate, maintainer org and funding, release cadence, last release, relay requirement and self-hosting, dependence on n0 DNS/relay infra, hole-punch mechanism. Versus `quinn` + custom punch as `icq` does (cite icq Cargo.toml `file:line` for its transport crates). One line each on str0m, webrtc-rs, libp2p quic+dcutr. Recommend, state reversibility.
2. Sign in with Apple on native macOS (AuthenticationServices via objc2, entitlements, developer account, what the token contains, private relay email) and Google OAuth loopback/PKCE for desktop (`oauth2` crate). Is "key is root identity, OAuth binds an email claim to the key" one week or one month. Say which.

## Deliverable

`.bboxy/research/transport-and-identity-exec.md`, two sections, `source: url` or `file:line`, `**Gap:**`, Recommendation under each. Links: [[vision]] [[agent-messaging]] [[transport-and-identity]].

## Verification

```sh
ls .bboxy/research/transport-and-identity-exec.md && grep -c 'Recommendation' .bboxy/research/transport-and-identity-exec.md
```

Report on `#bboxy450-term`.
