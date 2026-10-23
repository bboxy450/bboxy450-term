---
id: 01M5NK2SRD1QWWZW77NBRYENNW
type: decision
number: 50
title: "when MIT, when Apache: dual for what we own, Apache alone where upstream forces it, never copyleft; identity is not licensed"
status: decided
decided: 2026-10-23
operator:
  - "MIT and Apache licenses, when to use what"
  - "corporate friendly, hardcore developer first"
links:
  - { to: "[[licensing]]", rel: decides }
  - { to: "[[d34-licensing]]", rel: amends }
  - { to: "[[licenses-present]]", rel: constrains }
created: 2026-10-23T23:56:02Z
updated: 2026-10-23T23:56:02Z
---
# Decision 50: when MIT, when Apache

Decided by the operator's two lines, 2026-10-23; amends #34 ([[licensing]]) with the rule behind the map.

**Alternatives.** (a) MIT alone everywhere. (b) Apache-2.0 alone everywhere. (c) Dual MIT OR Apache-2.0 for every crate we own, Apache-2.0 alone where an Apache-only upstream lives inside the crate, no copyleft or source-available licence anywhere, no CLA. (d) A copyleft or source-available licence for the app to keep forks honest.

**Choice.** (c).

**Reason.** "Corporate friendly, hardcore developer first" is exactly what dual buys. The developer picks MIT: one paragraph, drop it in, GPLv2-compatible, no questions. The corporation picks Apache-2.0: the explicit patent grant and the contribution terms its lawyers already approved. Nobody has to ask us. Apache alone where upstream forces it (`bbt-core` carries `alacritty_terminal`, Apache-2.0 only): a derivative of Apache code cannot be offered under MIT, and we do not relicense other people's work. MIT alone drops the patent grant, which is the one thing corporate adoption asks about. Copyleft and source-available licences fail the first word of the operator's rule and are rejected for every crate, the app included: forks are kept honest by the brand ([[d52-brand-and-face]]) and by shipping faster, not by licence. No CLA: the signed commit is the contribution record we already keep.

**The rule, for any new crate.** Ours and clean: `MIT OR Apache-2.0`. Contains Apache-only upstream: `Apache-2.0`. Contains MIT-only upstream: still dual for our code, upstream headers kept verbatim, NOTICE lists it. Would need anything else: the crate does not get written until the Architect rules.

**Identity is not licensed.** The name `bboxy450`, the fox (`.bboxy/images/bboxy.png`, [[images-resolve]]) and the brand are the operator's identity, excluded from every code licence by a NOTICE line; a fork takes the code and changes the name and the face.

Owner: Corvin Duskmantle <ARCHITECT>.
