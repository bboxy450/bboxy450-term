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

## Call and hook measurements — #61, 2026-10-31

Both sides: the host (`bbt-mux` `plugins/measure.rs`, `PluginHost::dispatch`, `DataSourceProvider::request_before`, hook delivery) and Settings (`bboxy450-term` `settings/registry.rs`, `bbt-ui` `settings/plugins.rs`).

| Boundary | Agreement |
|---|---|
| Recording | Every dispatched contract call, declared seat tool, hook delivery and selected-provider call is measured host-side: the whole call (depth 0) and its phases (depth 1: `validate`, `admit`, `authority`, `connect`, `plugin`, `result`, `recheck`, as they apply). A ring of 64 per plugin, 256 plugins at most, in memory only. |
| Outcomes | `ok`; `refused` (the plugin's own refusal, with its `code`); `failed` (host: deadline, transport, schema, authority, with its `code`); `skipped` (never dispatched: `provider_busy`, `stale_generation`, `plugin_disabled`, `queue_full`, …). |
| Trace | Only for a trusted shape declaring `trace: true`, the host sends `initialize.params.capabilities.experimental.bboxy450.trace: true` (else `false`) and reads `result._meta.bboxy450.spans: [{name, start_ms, ms}]` beside `structuredContent`: at most 32, names 1–64 characters, offsets in ms from when the plugin took the call. Each span must fit inside the host's `plugin` phase window (`start_ms + ms ≤ plugin.ms`) and is merged there at depth 2, in order; plugin spans never change the host's timing. Otherwise the span is dropped and counted as the sample's `span_skip {code, count}` (`trace_span_invalid`, `trace_span_outside_window`, `trace_spans_invalid`, `trace_spans_oversized`). A shape without `trace` contributes none, even if the plugin sends spans. SDK: `Manifest::trace()`, `Ctx::span(name)` (a no-op unless the host enabled trace). |
| Listing | `PluginRequest::List` entries carry `measurements: [{ago_ms, method, hook?, ms, outcome, code?, spans?}]`: the last minute, newest 32, spans only on the newest. Absent when nothing was measured; never in the account record. |
| Settings | Timelines, the summary (with named non-`ok` outcomes) and the flame graph come only from these samples; a trusted plugin without samples keeps "Call and hook measurements unavailable". The bundled in-process provider is not measured. |
| Verification | `tests/plugin_host.rs` `calls_and_hooks_are_measured_with_phases_outcomes_and_named_skips` (a fixture call held 1 s, a 300 ms budget cut, a busy skip) and `hook_deliveries_are_measured_and_superseded_ones_are_named_skips`; `knowledge_provider.rs` provider and oracle-seat listings; `measure.rs` ring/listing units; app projection and `bbt-ui` `tests/settings.rs` rendering tests. |

## Setup and authentication — #57, 2026-10-31

Both sides: the host (`bbt-mux` `plugins/setup.rs`, `PluginRequest::Setup`, `ctl plugin setup <candidate>`, Settings › Plugins "Run setup") and plugins (`bboxy450-term-plugin-sdk` `Manifest::auth`, `Plugin::auth_status`, `Plugin::auth_login`).

| Boundary | Agreement |
|---|---|
| Declaration | Shape `auth: {status_command, login_command}`: two distinct lowercase words (`^[a-z][a-z0-9-]{0,31}$`, never `serve`/`shape`), else `invalid_auth`. Each is an argument for the trusted pinned copy itself, `<copy> <word>`; the host never runs another program for a plugin. |
| Status | `<copy> <status_command>` with stdin closed, the scrubbed environment, `HOME`/`TMPDIR` under the plugin's data directory and that directory as cwd, its own process group, within 2 s. Exit 0 = logged in, 1 = not logged in, anything else or a timeout = failed. The first non-empty stdout line (≤ 200 characters, escapes removed) is the detail. |
| Login | Only when the status says not logged in: `<copy> <login_command>` in a **visible plugin login row**: a bare supervisor seat (`LaunchSpec.bare`, seat kind `plugin-login`, named "<plugin name> login", role chip PLUGIN LOGIN, key icon) in the asking window's swarm (`ctl plugin setup … --swarm`; unscoped: a known swarm, else its own group), placed and focused in the window like a ctl-spawned seat. The command is its session leader with the PTY as controlling terminal, `TERM=dumb` and exactly the scrubbed environment (the supervisor injects nothing: no seat ID, `TERM`, colour or inherited variable) with the data directory as cwd. The operator types into it (keystrokes, `ctl seat type`/`seat key`) and sees its output as a normal terminal. It has no harness adapter, MCP seat socket, messaging inbox, hooks, skills, role grant, provenance, attention or notifications; its OSC frames (7501/7502/21337, 7, 52, titles) are never observed and its terminal title never shown. It cannot be restarted (`plugin_login_not_restartable`). Budget 120 s: past it the login's whole process group gets SIGHUP, then SIGKILL after 250 ms; when the leader exits the supervisor SIGKILLs anything left in its group before reaping it. The row then closes (`ClosedRetained`) and the setup job receives the exit and the row's text (at most 4 KiB). A login whose setup job was lost with its logic owner is ended the same way, and its row closed, when the next owner attaches; nothing is recorded for it. Then the status runs again. Without a logic owner (the registry API alone) the login runs in a private PTY with the same environment, budget and group cleanup, its output kept as escape-free text. |
| Result | `SetupResult {sha256, state: logged_in \| logged_out \| login_refused \| failed, code?, login_exit?, detail, elapsed_ms}` stored at `plugins/setup/<candidate>` and listed as `Entry.setup` only for the current pin. Codes include `status_deadline_exceeded`, `status_exit_<n>`, `login_deadline_exceeded`, `status_after_login_failed`, `setup_spawn_failed`, `setup_pty_unavailable`, `login_outcome_unknown` (the logic owner stopped during the login); a failure's `detail` is its reason's first line. Setup is a journaled mutation; it never trusts, enables or selects anything, and requires a trusted, inspected pin whose shape declares `auth` (else `setup_unavailable`). A result whose pin changed during the run is refused `stale_plugin_completion`. |
| Settings | The auth field shows the recorded result in plain words ("logged in · <detail> · 12 ms", "login refused (exit 2)", "setup failed (<code>)", "setup not run"). "Run setup" is available for a trusted plugin whose shape declares auth; when a login is needed it opens the plugin login row in the window's swarm and focuses it. |
| Deferred | A device-flow URL or code the login prints is shown only in its row; Settings does not repeat it. `ctl plugin setup` waits for the login within ctl's own reply timeout; a longer login still completes and is recorded. |
| Verification | `PluginSetup`/`BrokenPluginSetup` (native and Java TLC; owner loss and orphan cleanup included); `bboxy450-term` `tests/plugin_login.rs` (the real app and ctl: a typed code completes the login in its row, the row closes, a SIGHUP-ignoring grandchild is ended; a wrong code is a refused login); `tests/plugin_setup.rs` (the private-terminal success, refusal, hang at a 1 s budget and missing-auth cases; a login seat past its budget; an orphaned login seat ended on reattach); supervisor `a_bare_terminal_gets_only_its_environment_and_ends_its_process_group`; `setup.rs` escape-stripping unit; SDK shape test; app label/request and `bbt-ui` controller/settings/seat-row tests. |

## Hook budget warning and skip agreement (#31, operator batch 2026-10-31)

The host registry owns one bounded persistent budget record per candidate,
keyed by activation generation and declared event/phase (bus subject included).
Wall time includes hook connection/admission and dispatch. A completed overrun
raises `Entry.hook_warnings`; an under-budget completion clears an unlatched
consecutive counter. Three consecutive overruns latch a skip for that event,
with one `account/control` publication on that transition. Later attempts are
named `hook_budget_skipped` measurements and preserve the block grant's failure
policy. The registry alert remains authoritative if the best-effort activity
stream is unavailable. Re-enable or re-trust advances the activation generation;
a late completion cannot affect its replacement, including A → B → A. Restart
retains the account record. Observational timeline ring eviction cannot clear a
skip. Settings reports measured time, budget and consecutive count, never the
plugin's requested work duration. The SDK G3 fixture attempts 10 s; production
ceilings and real cancellation remain enforced.

Host/consumer owners remain PluginHost and Settings. Verification:
`PluginHookBudget` / broken control with native/TLC parity, registry persistence
witnesses, the real host alert/skip test, and `make test-settings-plugins`.

## Transport 1.0 (#84, operator batch 2026-10-31)

Producer: a trusted SDK/MCP `bboxy450-term-transport-<provider>` declares
bounded `spawn_fields` (unique name, human label, `kind: "text"`, optional default).
Omitted kind defaults to text; unsupported kinds refuse as `invalid_transport_fields`. Consumer:
Settings Machines shows discovered transports and their state/fields; an enabled
transport is a Via choice `plugin:<provider>`. Machine preferences and launcher
fields retain names separately from labels. Changing Via clears another
transport's parameter overrides. Builtin ssh/mosh/et argv remains unchanged.

`transport_wrap({fields, command}) -> {argv}` is host-only, at transport tier.
The host validates names/defaults against the trusted shape and both DTO schemas;
no network or shell interpretation is required by the contract. The inner argv
is `/bin/sh -lc <existing quoted cwd/env/command composition>`, so remote context
has the same meaning as the builtins. Returned argv is validated before the
supervisor sees it. Malformed fields/commands refuse during trusted inspection.

The operator spawn operation is admitted before IO. Wrap runs on a blocking IO
worker, bounded at two seconds, while the logic owner remains available. Its
completion captures the root binding and connection authority. The owner fences
both before launching, revalidates the role, and uses the same original journal
operation. Pre-effect refusals are rejected; ambiguous supervisor outcomes stay
unknown and never authorize an automatic second launch. Losing a UI waiter does
not revoke an admitted owned launch. A busy/unavailable completion queue returns
an honest unknown outcome instead of waiting on the owner. Disable/re-pin
reconciliation removes superseded connections before acknowledgement.

Verification: `PluginTransportLaunch` / broken control and existing
OperationRecovery; the real SDK fixture in `tests/plugin_transport.rs` checks
literal fields through the owner and supervisor without network, a held wrap
versus status reads, disable in flight, and duplicate operation IDs. The SDK
example passes conformance, malformed declarations refuse at trust, builtin
remote-launch tests remain, and `make test-plugin-transports` captures Settings.

Explicit seat restart replays its retained, already-authorized wrapped argv,
matching built-in transport recovery. It does not run a new plugin callback.
A new spawn always resolves the current trusted activation and fields.


The transport admission deadline is captured before worker scheduling. An async
owner timer covers a saturated blocking pool as well as the plugin RPC;
expired queued work is canceled and a running worker's late result is retired.
The launch owner checks connection authority with a nonblocking lock attempt:
contention refuses `transport_unavailable`, and replacement refuses
`transport_changed`. A dropped GUI waiter does not revoke its owned admission.
`PluginTransportProgress` assumes weak fairness only for that timer and the
owner completion group, not for a plugin reply, blocking capacity or mutex
release. Its broken control removes timer fairness. Private runtime witnesses
hold the blocking pool or host mutex, retire late/duplicate completion and keep
status reads moving. Wall clocks, filesystems, signatures and journal durability
remain implementation coverage, without a Rust refinement claim.

## Seat attribution (task #5003, 2026-11-01)

Both sides: the host (`bbt-mux` `mcp/bridge.rs`, `seat_socket.rs`, `plugins/attribution.rs`, `PluginHost::dispatch`, `DataSourceProvider::request_traced`) and plugins (`bboxy450-term-plugin-sdk` `Ctx::attribution`; the GitKB plugin's `git-kb` child).

| Boundary | Agreement |
|---|---|
| Names | Exactly `SESSION_ACTIVITY_RESOURCE_ID`, `SESSION_ACTIVITY_RESOURCE_KIND`, `SESSION_ACTIVITY_PROVIDER`, `SESSION_ACTIVITY_CONTEXT_FILE`, `SESSION_ACTIVITY_SINK_COMMAND`, `SESSION_ACTIVITY_SINK_ARGS_JSON`, `ATC_SESSION_ID`, `GITKB_AGENT_ID` (`bbt_plugin::policy::ATTRIBUTION_ENV`): what `git-kb` reads to export document activity to ATC. Never tokens, registry paths or shim internals. |
| Validation | `policy::attribution_env`: each value nonempty, at most 1 KiB, no NUL or line break; `SESSION_ACTIVITY_SINK_ARGS_JSON` a JSON string array; at most 4 KiB in all (a name that would exceed it is dropped). Applied by the bridge, again by the host, and by the SDK. |
| Seat to host | `bboxy450-term mcp` captures the names from the environment its harness gave it, once at start, and sends them as `attribution` beside each `Action::Call` on the seat socket (absent when empty). |
| Host | Kept per seat and process generation in the logic worker's memory, never journaled. Every plugin `tools/call` made for that seat (declared seat tools, data_source reads, and both phases of a document or task effect) carries `params._meta["bboxy450/attribution"] = {"env": {NAME: value}}`. Calls made by the operator, the host, hooks, checks or another seat carry none. |
| Plugin | `Ctx::attribution()` returns the validated map for that call only, empty otherwise. Apply it to that call's own child processes, never to shared or background work. The GitKB plugin spawns one `git-kb` per command and adds the map to that child's environment only; its index builds use detached contexts and stay unattributed. |
| Verification | `bbt-plugin` `policy` attribution test; bridge capture test; `plugins/attribution.rs` scope test; `tests/plugin_attribution.rs` (seat data_source reads and effect reads, seat tools, an unattributed seat, the operator, two seats); SDK runtime accessor test; GitKB `backend.rs` fake `git-kb` environment test; `tests/conformance.rs` `attribution_reaches_the_gitkb_activity_sink` (real `git-kb` commit to a capture sink). |
