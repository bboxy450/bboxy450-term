---
id: 01M5P8K1RQ8K0NR4FV0D3369JP
type: decision
number: 262
title: "launcher role defaults: Claude is oracle, Codex is executor and stays one, the operator overrides a Claude seat per seat through a new grant epoch"
status: decided
decided: "2026-10-24"
operator: []
links:
  - { to: "[[w2-5-mcp-and-seat-wiring]]", rel: decides }
  - { to: "[[messaging]]", rel: builds-on }
  - { to: "[[d18-executor-harness-override]]", rel: builds-on }
  - { to: "[[e2e-acceptance]]", rel: constrains }
created: "2026-10-24T06:11:54Z"
updated: "2026-10-24T06:11:54Z"
---
# Decision 262: launcher role defaults

Ruled by the Architect on 2026-10-24 from Aster's launch audit: the launcher labelled a Claude seat `executor` by default.

**Alternatives.** (a) Every seat defaults to `executor`, the operator relabels. (b) Defaults follow the harness rule of `AGENTS.md`: Claude `oracle`, Codex `executor`, shells none; Claude overridable per seat to any of the four roles (Executor by decision #18), Codex not overridable to an oracle-class role; overrides at spawn through the presets and later through the role chip as a new grant epoch. (c) Ask the operator the role on every spawn.

**Choice.** (b).

**Reason.** The roles are the structure ([[vision]], `AGENTS.md`): Claude supervises, designs and creates; Codex implements; a `ROLE:` header moves Claude between its roles and never makes an Executor an Oracle. A default that contradicts the law on the first screen teaches every new seat the wrong thing, and (a) did exactly that. (c) costs a question on every spawn for an answer the law already gives. The label is the grant's role, so a relabel is a real grant change with an epoch, visible in every envelope ([[messaging]]), never cosmetic.

**Rules.** In [[w2-5-mcp-and-seat-wiring]] §3, Role defaults. Implemented by Nyx or Sable at the Oracle's call; the e2e promise L1 (launcher) is read against it.

Owner: Corvin Duskmantle <ARCHITECT>.
