---
id: 01M5P2ETWHD1JWXCYQH5X2EF84
type: decision
number: 255
title: "visualization is always ours; plugins plug into data sources; everything pluggable with opinionated defaults"
status: decided
decided: "2026-10-24"
operator:
  - "I WANT VIZUALIZATION TO ALWAYS BE OURS, and we plug into data sourcse! like document bases, code hosting, git or what not, everything pluggable and with opionated defaults!"
  - "what do you mean in plugin spec that all plugins should be MCP? is that really sota way of doing that? my thinking was - you can return things via hook, but we have now sockets and event system for that"
links:
  - { to: "[[plugins]]", rel: decides }
  - { to: "[[d249-plugins-are-mcp-servers]]", rel: amends }
  - { to: "[[d41-plugins]]", rel: builds-on }
  - { to: "[[d12-audience-and-plugins]]", rel: builds-on }
  - { to: "[[mantras]]", rel: mantra }
created: "2026-10-24T04:24:45Z"
updated: "2026-10-24T04:24:45Z"
---
# Decision 255: visualization is always ours

Decided by the operator on 2026-10-24 after reading #249. The question behind it: does "a plugin is an MCP server" mean plugins are generic MCP servers and the app grows a plugin UI? No. #249 answers one question only, which framing, handshake, cancellation and shutdown the binary speaks over stdio; it is MCP because every harness we host and every language already speak it, so it is the one protocol we never document. This decision states what #249 left implicit and the operator made explicit.

**Alternatives.** (a) Plugins may contribute views, panels or widgets the app hosts. (b) Plugins are data sources and effects only; the app renders everything they return through its own views. (c) Plugins are generic MCP servers the app exposes as they come.

**Choice.** (b).

**Reason.** The operator's words. The cockpit's look is the product ([[vision]], decision #39, [[rim-glow]], the mantras "every pixel counts" and "agnostic but opinionated defaults"): a plugin that draws is a plugin that breaks parity, the invariants and the one look on every platform. Data is where the world differs: document bases, task systems, code hosting, git, buses, decision models. So the seam is there. (c) makes the app a shell for other people's tools; (a) makes it a browser.

**Rules.** A contract returns records, resources and results in the app's shapes, never pixels, markup or layout; `plugin check` refuses a shape whose result schemas carry a UI type. The built-ins are the opinionated defaults and the first providers of their own contracts (the task system, the local repo graph, the local document store); a plugin replaces a source or a sink, never a view. Messaging tree, roles, priority and signing are the app's; a plugin is an `event_sink` or, later, an `event_source`, never the bus. Hooks remain one of the ways a plugin returns things (a hook handler is a tool the host calls); sockets and the event system are the `event_sink` and `session` contracts, not a parallel plugin path. Named for later: `code_host` for pull requests, reviews and CI status; `event_source` for inbound buses.

**What it changes in [[plugins]].** §1 opens with "contracts first; MCP is only the wire" and this rule; revision 4a is otherwise unchanged. Mantra 25 in [[mantras]]: visualization is ours, plugins bring data.

Owner: Corvin Duskmantle <ARCHITECT>.
