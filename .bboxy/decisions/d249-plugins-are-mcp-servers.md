---
id: 01M5NVMX3AHPF45S11WD4SYZ2Z
type: decision
number: 249
title: "a plugin is an MCP server; contracts are JSON Schema in bbt-plugin; tiers and hook ceilings are data"
status: decided
decided: "2026-10-24"
operator: []
links:
  - { to: "[[plugins]]", rel: decides }
  - { to: "[[d41-plugins]]", rel: amends }
  - { to: "[[mcp-control-plane]]", rel: builds-on }
  - { to: "[[2026-10-24-plugins-r3-mcp-lifecycle]]", rel: answers }
  - { to: "[[2026-10-24-plugins-r3-bbt-plugin-layering]]", rel: answers }
  - { to: "[[2026-10-24-plugins-r3-tier-table]]", rel: answers }
created: "2026-10-24T02:25:43Z"
updated: "2026-10-24T02:25:43Z"
---
# Decision 249: a plugin is an MCP server

Taken by the Architect on 2026-10-24 from Seraph's review of [[plugins]] revision 3 (#213 to #230); amends [[d41-plugins]]. The operator's design (a folder of binaries, each with a shape, a registry with timings) is unchanged; this decides what the binary speaks.

**Alternatives.** (a) Our own newline-delimited JSON-RPC protocol with MCP's framing but its own lifecycle, cancellation and shutdown, as revision 3 had it. (b) Adopt MCP's lifecycle messages inside our protocol. (c) A plugin **is** an MCP server over stdio: its tools are the contract methods, `initialize` carries the shape version, the contract versions and capabilities under `capabilities.experimental.bboxy450`, `tools/call` with `structuredContent`, a JSON-RPC error is a protocol error and `isError` a command error, `notifications/cancelled` is cancellation, shutdown is close stdin then `SIGTERM` then `SIGKILL`; `--shape` stays the offline descriptor read once at trust.

**Choice.** (c).

**Reason.** LESS IS MORE: the suite already speaks MCP in both directions ([[mcp-control-plane]], decision #3), so a second protocol with the same framing is a second thing to maintain, document and test. Every language's MCP SDK becomes a plugin SDK and the guide shrinks to "an MCP server with these tools, plus `--shape`". The two-level error taxonomy revision 3 asked for already exists in MCP. Rejected: (a) for the duplicate, (b) for carrying MCP's lifecycle without its SDKs.

**Decided with it, same review.** Contracts are JSON Schema documents in `bbt-plugin` with no Rust type from `bbt-proto`; a test in `bbt-proto` proves `Envelope` and `TaskVerification` serialise to them, so the kit stays below `bbt-proto` and nothing is defined twice (rejected: moving the envelope types down, two homes for one shape). The trust tier is a table with one row per contract and the hook ceilings are a table per event class, both as data in `bbt-plugin`, both refused at trust when a shape exceeds them (rejected: tiers as prose, budgets declared by the plugin without a ceiling). At trust the binary is copied content-addressed into the operator's store and only that copy is ever executed (rejected: trusting a path, which can be swapped after the digest check; macOS has no `fexecve`).

**Still true.** Nothing is briefed; the spec is draft until the operator and the Architect talk; P-0's acceptance is written in [[plugins]] §3.

Owner: Corvin Duskmantle <ARCHITECT>.
