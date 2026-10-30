---
id: 01M5WG2WS221JWTJ6EQ04S78N0
type: interface
number: 350
title: "ICD: plugin ↔ host"
status: baselined
links:
  - { to: "[[interfaces]]", rel: index }
created: "2026-10-26T16:18:20Z"
updated: "2026-10-30T00:00:00Z"
---
# ICD: plugin ↔ host

| | |
|---|---|
| Side A | the contracts and the host (`bbt-plugin`, the knowledge host in `bbt-mux`) |
| Side B | plugin providers, bundled and external |
| The agreement | [[plugins]]; [[knowledge-base]] |
| Amended by | #249 a plugin is an MCP server; #255 visualization is ours; #278 P-0 rulings |
| What crosses | JSON Schema contracts with wire names; data only, never pixels. At start the host sends `initialize.params.root`, one UTF-8 absolute path, and nothing else of identity: no root, author, signer, device or account field ever appears on a `data_source` method's arguments (Kestrel, 2026-10-26; both sides of the conformance fixture assert it) |
| Timing | hook ceilings as data per contract; the handshake and each call are capped at 2 s by default |
| States and lifecycle | trust is a content-addressed copy; nothing executes before trust; plugins start disabled; one provider instance per root, the root bound at start. **Swap guard in production** (Kestrel, 2026-10-26): a proposed provider is selected for a root when its handshake succeeds and a bounded read over that root (a `list` page and a `resolve` of a known record, within the call caps) answers without error; otherwise the selection is refused with the provider's reason and the current provider stays. A full comparison of path and digest maps is **not** a production guard, since digests are provider data and a whole-root scan per swap is a global scan; it is acceptance evidence only, for the same-representation stub swap of [[knowledge-base]] §6 |
| Failure | a refusal names its reason in the app; providers without the signed writer capability reply `signed_writer_unavailable`; authorship is always the host's |
| Versioning | contracts are versioned in `bbt-plugin`; a provider declares the version it implements |
| Verified by | `plugin check` on its fixture store; contract conformance fixtures; promises G3, G4, D1 and up, N1 and up |
| Open | native GitKB commit has no generation-CAS argument; preflight references are checked, and the native commit transaction owns the final snapshot |

A change to this interface names both sides, the reason and the verification it needs, and lands only with both sides' owners agreeing. A question an Executor asks about this boundary is a missing line here: the answer is written into this table, not only sent.

Owner: Corvin Duskmantle <ARCHITECT>.

## Writable GitKB bridge (operator scope, 2026-10-28)

The host admits and signs the document write intent. The additive
`signed_record` string crosses `data_source_write` and is retained and echoed
byte for byte by a writable provider. No private key or caller-supplied author
crosses this boundary. GitKB can canonicalize its native UUID, timestamps and
status representation; an opaque host-signed intent is distinct from the
provider's canonical Markdown. A refusal or stale generation is explicit.
The descriptor advertises `signed_writer: true` for this optional bridge;
ordinary presentation identity fields never change execution behavior.
The GitKB adapter retains the opaque intent in `bboxy_signed_record`, maps the
same 128 bits between native UUID and portable ULID, and returns the native
UUID as `external_id`. Native YAML keeps its UUID, slug and native status;
`bboxy_status` carries the host's finer task lifecycle. Only the host's
evidence gate can produce `accepted`; native completion alone cannot.

Foreign providers own their numbering semantics; no `next_number` extension or
second global numbering ledger is required. GitKB's full slugs are first-class
document and task locators, within the selected root. Native identifiers remain
available for identity continuity across slug changes. Numeric family handles
must never resolve an ambiguity by ordering.

Each swarm captures its own cwd/root, plugin discovery, provider binding,
document/task view and message context. A completion admitted for one swarm
cannot populate another swarm's view after selection changes. Acceptance uses
two roots with the same slug, distinct providers, tasks and messages, including
switches with reads in flight and persisted selections after restart.
Slow provider writes and Git commits run as bounded IO work with their captured
root and actor; they cannot hold the logic owner and stall another swarm's
input, resize or status requests. Journaled unknown outcomes never redispatch
automatically.

`data_source_commit` is an optional additive operation: explicit document
locators and a commit message produce the provider's commit identifier and
committed document references. The host resolves/admit-checks them in the
captured swarm/root. Missing support is an explicit command refusal, not a
fallback to bundled Git. Existing eight-method providers remain conformant.
The commit identifier is a bounded opaque string: Git returns its actual hash,
GitKB its actual native commit UUID. Committed references identify the admitted
scope; clients resolve again for the post-commit generation.
The optional commit operation has a 15-second host ceiling, including its
reference preflight and native transaction. The original eight data operations
and production initialization keep their 2-second ceilings. Commits run off the
logic owner and never retry an unknown native outcome automatically.

Conformance setup remains disposable. A provider that requests fixture
bootstrap receives an explicit fixture initialization only from the checker's
owned temporary account. GitKB imports the canonical seeds into a real private
GitKB workspace before contract checks. Production initialization never carries
this marker and the checker never redirects mutations to Harmony.
Only that disposable bootstrap initialization may use a 15-second cap: real
GitKB initialization measured 6.077 seconds, including its SQL checkpoint.
Production initialization and ordinary contract calls retain the 2-second cap.

## Integration-skill addendum — operator brief #63, 2026-10-29

This additive agreement joins the binary, MCP surface and workflow content described in [[plugins]] §3a. [[plugins-sota]] §B separates the current compatibility cut from the published Beta extension.

| Boundary | Agreement |
|---|---|
| Producer | A provider may add `Shape.skills: [{ name, description, content }]`; each `content` is its embedded canonical `SKILL.md`. Legacy shapes omit the field. |
| Bounds | At most 16 entries, 64 KiB UTF-8 content each, and 256 KiB serialized skill-array JSON; invalid metadata is refused before publication. |
| Consumer | The logic host resolves built-in skills and the skills of every enabled, checked plugin once for both fixed `skill_list`/`skill_read` tools and legacy `resources/list`/`resources/read`. |
| Scope and lifetime | An external read retains its captured root. Every list/read rechecks each plugin's live enable state, conformance pin, trust and trusted binary digest; origin is the candidate id and the pin its SHA-256, independent of root provider selection. A disabled or re-pinned plugin's URI is refused; focus never retargets it. |
| Content and authority | The trusted binary contains the skill bytes. Returned content keeps its origin; it is instructional data and grants no role, signing, custody or execution permission. External instructions are never automatically injected at harness launch. |
| Failure | Missing legacy skill metadata produces no external skills. Malformed metadata is refused. A plugin whose current authority fails is omitted and named in the `provider_error` array (`{origin, plugin_sha256, code, message}`; `null` when none failed) while host guides and other plugins remain available; a stale external URI is refused. Neither path substitutes another plugin or the current focus. |
| Verification | Bounded descriptors; equal tool/resource catalogs and bytes; enabled-and-checked admission; selection-independent identity; per-plugin errors; root isolation; stale generation/disable/trust/digest fences; unchanged authority. |
| Next interface revision | Native Beta `server/discover` plus `skills/list`/`skills/get`, complete file manifests and supporting files require an explicit revision. Preserve origin and canonical bytes across that mapping; do not advertise this support before it exists. |

The KB plugin in PR #76 is the first concrete producer. Its builder derives embedded instructions and resource content from the same source. Generic tool forwarding and standalone bundled stdio export are separate work.

## Live integration diagnostics — issue #78

`knowledge_status` is a read-only seat tool. It returns the captured swarm/root
and provider revisions, selected provider and digest, eligible choices, compact
plugin trust/enable/check states and provider refusals. It remains available
when the selected provider cannot answer. It uses the existing seat authority
and completion fences; it grants no trust, selection or execution authority.
The operator's scoped `ctl` controls still own activation. Fresh operation IDs
read current plugin state; an explicit retry retains its original snapshot.

Successful seat document, graph, task, skill and status reads attach source
context before producing both MCP text and structured responses. Retained
responses with disagreeing representations are refused; a new operation ID
reads the current source.

The bundled index reserves only root-level `plugins/` and `plugin-notices/`
for packaging. Their files and attribution remain intact, outside document
search and graph inventory. Identically named nested document folders remain
ordinary knowledge.

GitKB list/search/graph pages contain at most one source-aware record, including
continuation pages, to reserve time for native inventory/search/edge queries
within the unchanged two-second contract. The read-only Harmony probe runs
fresh sessions without retries,
checks first/continuation pages and exact native source parity, and retains
every operation's timing or failure before asserting its deadline.

## Hooks — G3, 2026-10-30

Agreement in [[plugins]] §3b. Both sides: the host (`bbt-mux` `plugins/hooks.rs`) and plugins (`bboxy450-term-plugin-sdk` `Plugin::on_hook`).

| Boundary | Agreement |
|---|---|
| Declaration | `Shape.hooks: [{event, phase, budget_ms, subject?}]`. `subject` is required for, and only allowed on, event `bus` (phase `after`; a bus pattern whose first token is a public literal, never `inbox` or `plugin`). Budgets above the §1b ceiling are refused at trust. |
| Wire | One MCP `tools/call` of tool `hook` on the plugin's hook connection (a second process, separate from its call connection), arguments `{event, phase, budget_ms, subject?, seat?, payload?}`, result `{decision: "allow" \| "deny", reason?}` (reason at most 4 KiB). `event_sink` plugins use the same wire; their contract `deliver` stays a host-only contract method. |
| Operator grants | `PluginRequest::Trust {path, sha256?, grants?, tier?}` (GUI and `ctl plugin trust … --grant …`); re-trusting with changed grants is a new trust generation; grants outside the shape's request are refused at inspection (`grant_not_requested`); `payload_events: ["*"]` grants every declared payload event. Tool input reaches a payload-granted hook with each field bounded by an explicit truncation marker. |
| Grants | `payload` appears only when the trust record's `payload_events` contains the hook's event (`bus` for bus hooks; that grant needs the export tier). A `deny` decides only for a `before` hook whose record holds a `block` grant, through `PluginHost::decide`; the grant's `FailurePolicy` decides a timeout, transport failure or superseded activation. |
| Timing | Each call is bounded by `min(budget_ms, ceiling_ms)`; non-blocking delivery is queued (64 per root), dropped and counted when full, and never runs on the logic thread. |
| Lifetime | A queued item carries its activation generation; a disabled, re-enabled or re-trusted plugin receives nothing from older offers. |
| Outcomes | `plugin/<candidate>/hook/<outcome>` on the account stream: `allowed`, `denied`, `ignored`, `failed_open`, `failed_closed`, `failed`, `stale`, `dropped`. |
| Production veto | Gate file `<run>/seat-<seat>/hook-gate.json` = `{"events": {<name>: {"policy": "fail-open" \| "fail-closed", "ceiling_ms"}}}`, written by the seat root's `PluginHost` (absent when nothing blocks). Seat socket `HookDecide {event, wait_ms}` → `{"kind":"HookDecided","status","decision":"allow"\|"deny","reason"?}`. The shim waits at most `ceiling_ms − 250`; the host decides within `wait_ms = ceiling_ms − 350`. Deny = exit 2 with the reason on stderr (Claude Code `PreToolUse`/`UserPromptSubmit`/`PermissionRequest`, Codex `PreToolUse`/`UserPromptSubmit`); no reply = the gate's policy. |
| Verification | `PluginHook`/`BrokenPluginHook` and `PluginHookGate`/`BrokenPluginHookGate` (native and Java TLC); `tests/plugin_host.rs` hook tests; `tests/hook_gate.rs` (real shim process, seat socket, logic and plugin); `bboxy450-harness-wake-up` `tests/hook_sink.rs`; `bbt-plugin` policy and shape tests; SDK runtime hook test. |

## Host API — G7, 2026-10-30

Agreement in [[plugins]] §3c. Both sides: the host (`bbt-mux` `plugins/client.rs` reader, `plugins/host_api.rs`) and plugins (`bboxy450-term-plugin-sdk` `Ctx::host()`).

| Boundary | Agreement |
|---|---|
| Declaration | Additive `Shape.host_api: ["records.search" \| "records.fetch" \| "events.publish" \| "messages.send"]`; unknown or duplicate names are refused. The operator grants a subset at trust in `Grants.host_api`, and `messages.send` destinations in `Grants.messages_to`. `records.*` raise the effective tier to export, `events.publish` and `messages.send` to write. |
| Wire | The plugin writes `{"jsonrpc":"2.0","id":<its own id>,"method":"host/<name>","params":…}` on stdout. The host answers on stdin with `result`, or with `error {code, message, data: {code, message}}`: `-32601`/`unknown_host_api` for an unknown method, otherwise `-32000` with `data.code` (`host_api_not_granted`, `provider_changed`, `host_api_busy`, `deadline_exceeded`, `invalid_request`, `invalid_subject`, `payload_too_large`, `host_api_unavailable`, `response_too_large`). |
| Methods | `records.search`/`records.fetch`: data_source 1.0 `search`/`fetch` request and result for the plugin's own root, validated against the contract schemas. `events.publish {subject, payload?}`: published as `plugin/<candidate>/<subject>`, source `plugin:<candidate>`, payload an object of at most 64 KiB; returns `{id, subject}`. |
| Timing | At most 4 requests at once per connection, each within 2 s. A request is served on its own task while the reader keeps reading, so it may be sent during a host call to the same plugin. The SDK waits within the calling request's deadline and drops a late answer. |
| Lifetime | Each request rechecks that the connection's activation generation and trust record are still current. Requests are served only on `PluginHost` connections, not on the selected data_source provider's own connection. |
| Messages | `messages.send {destination, text, id?, priority?}` → `{id, destination}`: plain text to a destination exactly in `messages_to`, else `destination_not_granted`; `invalid_request` for threads/mentions/attachments. Sender `plugin:<candidate>` (ledger `Recipient::Plugin`, tree `Actor::Plugin`, row `AuthorKind::Plugin` with the shape name); envelope `IssuerKind::Host` signed by the host's key, `issued_by` `plugin:<candidate>`, seat = derived plugin ID, generation 0, role `plugin`, worker class, no door. Never a seat or operator key. |
| Verification | `PluginHostApi`/`BrokenPluginHostApi` (native and Java TLC); `tests/plugin_host.rs` host API tests; `tests/plugin_messages.rs` (logic end to end); `messaging/plugin_sender_tests.rs` (attribution, signature, impersonation); `client.rs` callback test; SDK runtime host test; `bbt-plugin` shape test. |
