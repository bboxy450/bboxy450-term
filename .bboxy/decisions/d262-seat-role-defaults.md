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
updated: "2026-10-28T23:44:21Z"
---
# Decision 262: launcher role defaults

## Operator amendment, 2026-10-28

The operator now requires any role on any harness and explicit editable role classes, including built-in roles. Harness/model names impose no class restrictions. The original choices below are starting defaults only. Settings owns role availability and default selection for every harness, including shells and custom harnesses; both GUI and MCP admission use the configured class and prompt. Existing grants remain authoritative: deleting or reclassifying an in-use role still requires a separate grant reissue. No model-name allowlist is introduced.

Verification: `make test-settings-roles`, `make test-settings-roles-e2e`, `make test-boundaries`, and `make test` (the latter on the committed source).

## Original decision, superseded where amended above

Ruled by the Architect on 2026-10-24 from Aster's launch audit: the launcher labelled a Claude seat `executor` by default.

**Alternatives.** (a) Every seat defaults to `executor`, the operator relabels. (b) Defaults follow the harness rule of `AGENTS.md`: Claude `oracle`, Codex `executor`, shells `shell` (a worker-class role with an ordinary grant and no role prompt; Nyx's shape question, same day: one more `Role` variant, no `Option<Role>`, no `Class::None`); Claude overridable per seat to any of the four roles (Executor by decision #18), Codex not overridable to an oracle-class role; overrides at spawn through the presets and later through the role chip as a new grant epoch. (c) Ask the operator the role on every spawn.

**Choice.** (b).

**Reason.** The roles are the structure ([[vision]], `AGENTS.md`): Claude supervises, designs and creates; Codex implements; a `ROLE:` header moves Claude between its roles and never makes an Executor an Oracle. A default that contradicts the law on the first screen teaches every new seat the wrong thing, and (a) did exactly that. (c) costs a question on every spawn for an answer the law already gives. The label is the grant's role, so a relabel is a real grant change with an epoch, visible in every envelope ([[messaging]]), never cosmetic.

**Rules.** In [[w2-5-mcp-and-seat-wiring]] §3, Role defaults, including the `shell` shape ruling and the role-to-class table: `oracle` and `architect` are `Oracle` class, `creativity`, `executor` and `shell` are `Worker`; the class comes from the table by role name at grant time and from nowhere else. Implemented by Nyx or Sable at the Oracle's call; the e2e promise L1 (launcher) is read against it.

Owner: Corvin Duskmantle <ARCHITECT>.
