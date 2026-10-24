---
id: 01M5QJHV1N24RBNHAX53KYTAV7
type: decision
number: 278
title: "plugin P-0 pre-code rulings: the checker runs headless on a fixture store and never touches production trust; five contracts at 1.0 and two describe-only at 0.1; one name per method; contract_method wire names"
status: decided
decided: "2026-10-24"
operator: []
links:
  - { to: "[[plugins]]", rel: decides }
  - { to: "[[d249-plugins-are-mcp-servers]]", rel: amends }
  - { to: "[[d257-knowledge-base-first-plugin]]", rel: builds-on }
  - { to: "[[knowledge-base]]", rel: builds-on }
created: "2026-10-24T18:25:15Z"
updated: "2026-10-24T19:04:44Z"
---
# Decision 278: P-0 pre-code rulings

Ruled by the Architect on 2026-10-24 from Kestrel's plan-first read of [[plugins]] before writing code; both were gaps the spec had left to a guess.

**Trust bootstrap.** `bboxy450-term plugin check <binary>` never touches the production trust store. It runs as a short-lived headless logic mode (library calls into the host code in `bbt-mux`, never UI, supervisor or PTY startup, no second production account writer), creates an isolated fixture account under a temporary root with a preseeded trust record and the content-addressed copy of the binary under test, and refuses an untrusted production file by name. Production trust bootstrap is P-1's operator-only verb (`ctl plugin trust <path>`), never a flag on the checker. Rejected: an opt-in flag that writes production trust from a test tool.

**Contracts at v1.** Five at 1.0 with concrete method tables: `data_source` (implemented by K-0), `task_system` (one method per W2.5d tool with the tools' names, `status` is the read and there is no `get`), `event_sink`, `decision_model`, `repo_graph` (`describe`, `changes_since`). Two at 0.1, describe-only: `session` and `provider`, whose real methods are named and not callable until their contract reaches 1.0, and the app refuses a shape claiming 1.0 for them. Rejected: unconstrained object schemas as placeholders; a `get` alias beside `status`.

**Wire names.** On the plugin's MCP server a tool is `<contract>_<method>`; the host catalogue name stays `plugin_<system>_<provider>_<method>`. `Envelope` and `TaskVerification` stay the app's strict, opaque shapes; additive-field tolerance is tested at the plugin boundary only.

Owner: Corvin Duskmantle <ARCHITECT>.

**Addendum (same day).** The four remaining method tables are pinned in [[plugins]] §1 as Kestrel proposed, with two precisions: `event_sink.deliver` echoes the envelope id as the dedupe key and carries a `reason` when not accepted, and `decision_model.choose` must answer with one of the criteria names it was given or fail as a command error. `event_sink` is host-driven and not a seat tool; `decision_model` is Oracle class and operator only; `session` and `provider` `describe` are read class and have no effects.

**Addendum 2 (same day).** `task_system` mutating requests and results carry `signed_record: string`, required, returned byte for byte; the plugin never produces or alters it. The 64-character tool-name limit applies to the host-qualified name the harness sees, `plugin_<system>_<provider>_<method>` (Codex allows 128, verified at `tools.rs:226`; Anthropic's pattern is the binding one), and a shape that would exceed it is refused at trust.
