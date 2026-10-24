---
id: 01M5PEVPDKGQ63VA7QF32SA27Q
type: decision
number: 265
title: "Codex seat trust: the native hook review stays, one operator consent persisted by hash; no bypass flag, no copied credentials; our MCP server gets the scoped approval override"
status: decided
decided: "2026-10-24"
operator: []
links:
  - { to: "[[w2-5-mcp-and-seat-wiring]]", rel: decides }
  - { to: "[[plugins-sota]]", rel: evidence }
  - { to: "[[harness-adapters]]", rel: builds-on }
  - { to: "[[e2e-acceptance]]", rel: constrains }
created: "2026-10-24T08:01:29Z"
updated: "2026-10-24T08:01:29Z"
---
# Decision 265: Codex seat trust

Ruled by the Oracle on the operator's delegation, 2026-10-24, from Sable's research on the Codex 0.163 documentation for hooks and auth; written up by the Architect. The e2e nights found the Codex trust prompt as a blocker for an unattended seat.

**Alternatives.** (a) Launch Codex with `--dangerously-bypass-hook-trust`. (b) An isolated `CODEX_HOME` per seat with the operator's `auth.json` copied in, so the prompt never appears. (c) Keep Codex's native hook review: the operator consents once, Codex persists the consent by the hook definition's hash ([[plugins-sota]] §6 and A4: trust is recorded against the hook's current hash, a changed hook is `Modified` until re-trusted); our `bboxy` MCP server gets the documented per-invocation approval override in the seat's launch argv, scoped to that one server; a change to our hook content is a new hash and a fresh operator approval, which the launcher states.

**Choice.** (c).

**Reason.** (a) trusts every merged hook source at once, a repository's own `.codex/` included, which is the attack the trust review exists for. (b) duplicates a credential into a per-seat directory, the operator's call to refuse, and it fights the wiring kit's rule that user files are read and never written ([[w2-5-mcp-and-seat-wiring]] §3). (c) keeps Codex's own boundary, costs the operator one review per hook content, and lets the seat run unattended afterwards because Codex persists the consent. The scoped approval for our server is the documented mechanism, not a workaround, and it stops at our server.

**Rules.** In [[w2-5-mcp-and-seat-wiring]] §3, the Codex row. The launcher shows, before the first Codex spawn after a hook change, that Codex will ask once and why. The e2e promise L3 (user config untouched) and L4 (first-turn `orient`) read against it. Rejected, as above: the bypass flag; the copied `auth.json`.

Owner: Corvin Duskmantle <ARCHITECT>.
