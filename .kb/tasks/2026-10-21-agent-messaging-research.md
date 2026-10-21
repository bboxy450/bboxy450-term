# Task: embedded agent messaging — research

Date: 2026-10-21. Owner: Oracle. Assignee: Nyx Copperwing <EXECUTOR>. Status: CLAIMED 2026-10-21.

## Goal

Specify how ICQ-style messaging lives inside bboxy450-term: agents and harnesses message each other locally and across machines, with NAT traversal, account-scoped identity, global channels and role-based priority.

## Scope

Research only. Read the `icq` crate source (`cargo install icq`; source at `~/.cargo/registry/src/*/icq-*` or `https://docs.rs/crate/icq/latest/source/`) and the Rust P2P stack. No code outside `.kb/`.

## Questions to answer

1. **Transport.** How does `icq` do direct QUIC → simultaneous NAT punch → E2E-encrypted relay? Which crates (`iroh`? `quinn`? custom)? `file:line` for the dial path and the relay fallback.
2. **Identity.** How does `icq` derive and store keys, and how does it bind a harness session to a key? What would "one user account across machines" require on top: a root account key signing device/session keys, or a directory service?
3. **Rooms.** How are `#channels` and private groups implemented: DHT, gossip, relay-hosted, or helper-hosted? What is the delivery guarantee and what is not?
4. **Native delivery.** How does `icq` push a message into a Claude Code or Codex session (the `/tmp/cc-socks/*.sock` path, `crossSessionInbound`)? What does our terminal, which owns the PTY, get to do that an external daemon cannot?
5. **Reuse boundary.** Three options, costed: (a) link `icq` as a library crate, (b) run the `icq` daemon and front it, (c) reimplement on `iroh`/`quinn`. Name what each one blocks us from doing on priorities and roles.
6. **Priority and roles.** Where in a message envelope would a terminal-assigned role (Oracle/Executor/Creativity) and a priority live so the receiver can order and the UI can badge? Signed by whom?

## Deliverables

- `.kb/research/agent-messaging.md` — pointers and measured facts.
- `.kb/specs/messaging.md` — recommended architecture, envelope sketch, identity model, open gaps.

## Acceptance

- Every question has an answer or an explicit **Gap:** line.
- Transport and identity answers each carry at least one `file:line` pointer from the icq source.
- The spec names the reuse option chosen and what it costs.

## Verification

```sh
ls .kb/research/agent-messaging.md .kb/specs/messaging.md
grep -c 'Gap:' .kb/specs/messaging.md
```

## Report back

On `#bboxy450-term`: files written, commands run with exit status, what was not checked.
