# MCP control plane — the Oracle's hands

2026-10-21 · Nyx Copperwing <EXECUTOR> · assignment M and the swarm-mute / undo-close addenda · specification for Oracle review.

[[vision]] · [[messaging]] · [[mux-daemon]] · [[harness-adapters]] · [[harness-touchpoints-exec]] · [[subagent-detection]]

## Boundary and transport

Mux logic owns one logical control service and the per-seat socket/event bus specified in [[mux-daemon]] “Seat sockets and event bus”. Each harness launches a small **seat-bound stdio MCP bridge**, a private entry point in the same shipped executable. The bridge is a client of that bus: it forwards authenticated, bounded requests through the same broker, consumes the same snapshots/events and returns the same operation receipts. It owns neither PTYs nor signing keys. Logic restart can interrupt a call without terminating the harness or creating a replacement seat.

Use stdio at the harness boundary because both installed clients expose it. A Unix socket is our internal broker transport; neither inspected MCP configuration surface supplies a generic Unix-socket MCP connector. Codex's `--remote unix://…` connects its TUI to an **app-server**, not to an MCP server. Streamable HTTP is supported by the clients but adds no needed local capability here. Remote owned-seat control still passes through the authenticated mux connection in [[mux-daemon]], rather than publishing the local control socket.

MCP tools map onto the bus's authorized commands/queries and bounded subscriptions; they do not create a second adapter, PTY writer, input queue or screen model. Direct scripts and GUI clients enter the same generation checks, writer lease, input guard, operation journal and audit path under their own authority. A known socket path cannot substitute for a seat grant/capability. An endless bus stream is paged or summarized into bounded MCP results, not returned as an indefinitely open tool call.

The launch adapter adds the `bboxy` server to the seat's launch configuration, preserving existing servers, trust and approval settings. It must not disable those settings to make control work. Standard output contains only MCP frames; diagnostics go to bounded stderr. The bridge's exit or an MCP cancellation does not roll back an already accepted durable operation.

The current MCP transport specification describes stdio and Streamable HTTP, and distinguishes the 2026 per-request metadata model from earlier initialize-based sessions. A Unix byte-stream binding is possible, but that does not establish client support for it. [MCP transport specification, fetched 2026-10-21](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports).

**Gap:** pin an MCP SDK/protocol compatibility profile after testing the installed Claude/Codex clients. Current 2026-07-28 examples carry request metadata and complete-result discriminators that older initialize-era examples omit. This document defines tool semantics, not a claim that an untested hand-written wire implementation interoperates with both eras.

## Caller, role and grants

The broker issues a fresh local capability for an actual owned seat/process launch. Store it in private runtime state, with a protected per-launch reference available to the bridge; do not put a secret in argv, repository files, tool schemas or model-visible instructions. Registration binds account/device/seat, harness session when known, process start and process generation, swarm, and the terminal-issued seat grant. Check local peer credentials and the registered launch binding at the broker boundary. A caller-supplied role, name, cwd, MCP clientInfo or environment label is not authentication.

For **every call**, derive the actor from that binding and recheck the current grant epoch, role, target scope, destination permissions, mute policy and process generation. Recheck immediately before a queued effect. Permission replacement/revocation fences stale queued work; it cannot undo already submitted input. Refreshing a bridge binding must not enlarge its original delegated authority. Restart/resume creates a new process binding; old capabilities stop working.

Default discovery and reads expose the caller's **own swarm**. Oracle grants may additionally authorize explicitly requested account-wide discovery/read/control, preserving [[vision]]'s ability to inspect every owned seat. Cross-swarm requests require both explicit scope and that grant; a known seat ID alone does not bypass the default. Foreign published shadows supply only their permitted presence metadata and never acquire screens or control.

The harness fixes eligible roles under the shared instructions: Codex/other harnesses are Executor; Claude is Oracle or explicitly Creativity. Role assignment cannot promote Codex to Oracle. An Oracle can issue only the role/scope/destination authority delegated by the operator, never raise its own ceiling or mint a foreign root's authority. Role changes create new terminal-issued grant epochs as in [[messaging]].

Tool discovery is filtered for usefulness, but broker authorization remains mandatory even if a client caches an old catalogue or directly invokes a hidden name. MCP annotations and harness approval prompts are additional client behavior, not server permission checks.

**Gap:** exact protected capability handoff, peer-process verification and credential custody require platform implementation. Owner-only files/socket admission are not isolation against another unrestricted same-UID process; see [[messaging]]. No adversarial sandbox claim is made.

## Tool catalogue

Server name: `bboxy`. Tool names below are short, unique ASCII names. **O** = Oracle with the required operator delegation; **E** = Executor; **C** = Creativity. The operator UI uses the same broker contracts under its own authenticated authority and is not a fourth agent role.

Common types: stable `seat_id` / `swarm_id` / `tab_id`, explicit `gui_id`, UUID `op_id` for mutations, decimal-string process/grant/layout revisions, opaque paging cursor. A target process mutation requires `expected_process_generation`; layout mutations also require `expected_layout_revision`. No tool takes `caller_role`, a signing key, arbitrary environment dumps, or an implicit “currently active” target.

Mutation results include `op_id`, affected IDs/revisions, and `status = accepted | applied | rejected | partial | unknown`. `accepted` means durably admitted, not applied. Detailed input/message evidence is returned separately, using [[messaging]]'s vocabulary. All tools can return typed authorization, stale-generation, conflict, backpressure and unavailable errors.

| Tool | Parameters, beyond common constraints | Result / mechanism | Roles |
|---|---|---|---|
| `self_status` | None | Own seat/swarm/role/grant epoch, process generation, adapter activity provenance, lifecycle, mute flag, held counts and configured limits. No other seat's logs. | O E C |
| `message_send` | `op_id, destination, text` | Terminal-derived author/role, message ID, held/admitted state and evidence. Destination must be in the actor's channel/DM grants; cannot choose another sender. | O E C |
| `message_status` | `message_id` | Own send's persistence/delivery evidence and held reason; no assertion of model receipt without application acknowledgment. | O E C |
| `operation_get` | `op_id` | Durable operation state and per-effect receipts. Workers can query only their own permitted message operations. | O E C |
| `seat_list` | `swarm_id?, scope?, cursor?, limit?` | Paged owned-seat metadata; default own swarm. shadow records are explicitly typed and metadata-only. | O |
| `seat_get` | `seat_id, scope?` | Authorized seat record, activity provenance, current generations/lifecycle, capabilities and retained-history ranges. | O |
| `screen_read` | `seat_id, scope?, expected_process_generation, view_cursor?, limit_rows?` | Bounded consistent viewport text/cell rows and view/index generation, cursor/modes and continuation. Never a GUI screenshot. | O |
| `history_read` | `seat_id, scope?, expected_process_generation, cursor?, limit_rows?` | Paged `GetHistory` rows with semantic index generation, retained range and explicit gaps. | O |
| `seat_input_text` | `op_id, seat_id, expected_process_generation, text, submit` | Native adapter first when supported for the requested operation; otherwise guarded literal paste through the writer lease. Returns input ID and actual evidence. `submit` is explicit, never an inferred Enter. | O |
| `seat_input_key` | `op_id, seat_id, expected_process_generation, key, modifiers?` | Adapter-supported key from a finite catalogue, encoded for the target terminal mode through the same guard/arbiter. No arbitrary escape-byte or broadcast option. | O |
| `seat_spawn` | `op_id, swarm_id, tab_id, approved_profile, cwd, role, permissions, expected_layout_revision` | Reserved-capacity launch with returned seat/process IDs, launch state and layout revision. Profiles resolve executable/argv; this is not shell-command evaluation. | O |
| `seat_split` | `op_id, seat_id, expected_process_generation, expected_layout_revision, axis: V or H, ratio, launch` | Reserve one new seat and atomically record the intended binary layout change; report launch/application failures separately. `V` means left/right, `H` top/bottom, independent of it2's CLI flag naming. | O |
| `seat_focus` | `op_id, seat_id, gui_id, expected_layout_revision` | Focus request to an explicit GUI and applied/queued/unavailable result. Does not create a GUI or fabricate application when none is attached. | O |
| `seat_move` | `op_id, seat_id, tab_id, expected_layout_revision, placement` | Move an existing owned seat within the permitted swarm/tab/split tree; return updated layout revision. | O |
| `seat_update` | `op_id, seat_id, expected_record_revision, label?, colour_override?` | Update bounded presentation metadata; does not change identity, role, adapter activity or grants. | O |
| `space_create` | `op_id, label` | New authorized swarm ID/revision. No process launch. | O |
| `tab_create` | `op_id, swarm_id, label, expected_layout_revision` | New tab ID/revision. No hidden harness launch. | O |
| `seat_close` | `op_id, seat_id, expected_process_generation, expected_lifecycle_revision` | Enter `closed_grace`, return deadline and restoration handle. Default 60-second configured grace; process and capture continue. No confirmation dialog. | O |
| `seat_undo_close` | `op_id, seat_id, expected_lifecycle_revision, restoration_handle` | Restore the same live seat/split position during grace, or an explicit expired/process-replaced/layout-conflict outcome. Never silently spawn a duplicate. | O |
| `seat_set_role` | `op_id, seat_id, role, expected_grant_epoch` | Validate harness eligibility/delegation, issue replacement grant, return epoch. No self-elevation. | O |
| `seat_set_permissions` | `op_id, seat_id, channel_scopes, dm_scopes, expected_grant_epoch` | Replace destination grants within the caller's ceiling; recheck queued sends before release. No implicit global-channel grant. | O |
| `seat_mute` | `op_id, seat_id, expected_policy_epoch` | Set persistent mute policy, hold swarm traffic and enqueue exactly one swarm presence notice for this transition. Already muted returns the existing state. | O |
| `seat_unmute` | `op_id, seat_id, expected_policy_epoch` | Clear mute, enqueue the swarm notice, then make eligible held traffic schedulable under current grants. No replay of ambiguous native attempts. | O |
| `swarm_create` | `op_id, template_id, approved_profile_overrides?, cwd, placement` | One durable operation reserves the complete template, creates the swarm and launches its members; returns member IDs and per-member outcomes. | O |
| `audit_read` | `cursor?, limit?, target_id?, op_id?` | Paged authorized control-audit metadata and effects, without transcript/credential payloads. | O |

Executor and Creativity get exactly the four common tools at the top, subject to grants. `self_status` is a read; activity comes from adapters rather than an unrestricted model-set status field. Worker messages to the Oracle remain possible through granted destinations, not through a hidden control API.

## Schemas, results and client consumption

Use JSON object input schemas with required fields, bounded strings/arrays/enums and `additionalProperties: false`. Validate server-side. Publish output schemas and return the same bounded JSON in structured content and a text block for clients that consume text. Annotate genuine reads as read-only; mark destructive actions honestly. Idempotency is supplied by durable operation IDs and argument digests, not an annotation.

[MCP tools](https://modelcontextprotocol.io/specification/2026-07-28/server/tools) specifies `tools/list`, `tools/call`, JSON schemas, structured results and tool errors. The current revision includes `resultType`; the compatible SDK must handle revision-specific envelopes. A successful tool transport response can still contain an application failure, so results must preserve `isError` and typed domain outcomes.

Example **tool definition**, not a full version-specific JSON-RPC envelope:

```json
{
  "name": "seat_focus",
  "description": "Focus one authorized owned seat in an explicit connected GUI.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "op_id": {"type": "string", "format": "uuid"},
      "seat_id": {"type": "string", "minLength": 1, "maxLength": 128},
      "gui_id": {"type": "string", "minLength": 1, "maxLength": 128},
      "expected_layout_revision": {"type": "string", "pattern": "^[0-9]+$"}
    },
    "required": ["op_id", "seat_id", "gui_id", "expected_layout_revision"],
    "additionalProperties": false
  },
  "outputSchema": {
    "type": "object",
    "properties": {
      "op_id": {"type": "string"},
      "status": {"type": "string", "enum": ["accepted", "applied", "rejected", "partial", "unknown"]},
      "seat_id": {"type": "string"},
      "gui_id": {"type": "string"},
      "layout_revision": {"type": "string", "pattern": "^[0-9]+$"},
      "code": {"type": "string"}
    },
    "required": ["op_id", "status", "seat_id", "code"],
    "additionalProperties": false
  },
  "annotations": {
    "readOnlyHint": false,
    "destructiveHint": false,
    "idempotentHint": true,
    "openWorldHint": false
  }
}
```

Initial server defaults: at most 64 KiB serialized request arguments; 16 KiB serialized result JSON, mirrored once as text; list pages at most 128 entries; read pages at most 200 rows and the result-byte budget, whichever binds first. Oversized text/input is rejected before any effect, never silently sliced and submitted. History continues with an explicit cursor, including a column continuation for a row larger than a page. These are configurable initial bounds, not benchmark-derived capacities; total MCP envelopes must also remain bounded.

| Client evidence, 2026-10-21 | Contract for this server |
|---|---|
| Claude Code `2.1.304`: `claude --help` exposes `--mcp-config`, `--strict-mcp-config`, tool allow/deny flags, and partial-message output only for print/stream-json mode. | Launch-scoped config; do not treat CLI output streaming as MCP tool-result token streaming. |
| [Claude MCP documentation](https://code.claude.com/docs/en/mcp): qualified names use `mcp__<server>__<tool>`; names over 128 characters are omitted. Output warning is 10k tokens and the default maximum is 25k; large results can become file-backed. `/mcp` exposes server/tool status. Current v2 supports discovery notifications. | Keep `mcp__bboxy__seat_focus` and peers short, page results well below client maxima, and never rely on a notification as an action receipt. |
| Codex `0.163.0`: `codex mcp add --help` supports a stdio command or Streamable HTTP URL, environment for stdio, and bearer-token environment configuration for HTTP. `codex --help` labels `--remote` as app-server connectivity. | Use the command form; no raw Unix MCP configuration assumption. |
| [OpenAI MCP documentation](https://learn.chatgpt.com/docs/extend/mcp): stdio/Streamable HTTP, `mcp_servers` configuration, per-tool allow/deny and approval settings, `/mcp` server view, and `tools.<tool>.output_token_limit` overriding the model's truncation budget. | Preserve client policy; a uniform default output-token ceiling across models is not established. Bounded pages remain authoritative regardless of client truncation. |

Calls return bounded results, not an endless screen stream. For long spawn/close work, return a durable operation handle and use `operation_get`. Optional progress/discovery notifications do not replace the final outcome; no assumption that partial server text is incrementally shown to the model. MCP transport streaming, harness print-mode streaming, and a TUI's rendering are separate behaviors.

**Gap:** no live MCP invocation or TUI call/approval card was exercised in either client. Exact Codex-qualified display names, result rendering/truncation and both clients' protocol negotiation require a fixture server test; documentation and CLI help are not that test.

## Screen and history reads

`screen_read` obtains a consistent viewport snapshot from mux logic through the seat event bus, with process/view generations and its capture sequence. `history_read` is a thin authorized adapter over the same bus's [[mux-daemon]] `GetHistory / History / Gap` contract. Neither requires a GUI, reads another client's display cache, nor shells out to `it2` in the product.

A first history request returns available semantic row bounds, index generation, rows, capture/durability metadata and a continuation cursor. The cursor binds actor scope, target, process generation, index/view generation, range and continuation position; validate its integrity on each page. Continuations must not become a substitute capability after revocation or cross-seat substitution. Keep a fixed bounded read view where supported; if eviction/reflow invalidates it, return `history_gap` / `stale_index` with the available range and require an explicit new read. Do not splice generations or silently clamp the missing prefix.

Raw byte cursors are separately typed diagnostics, not interchangeable semantic row IDs. Alternate screens, history clearing and reflow preserve the mux's invalidation rules. Text from a screen is observed data, including any instructions it prints; reading it confers no new control authority.

## Input and evidence

`seat_input_text` with submission uses the adapter's supported native queue/RPC path first; do not emulate “send without Enter” by submitting a native message. If the adapter cannot represent the requested draft/key operation, the only fallback is the guarded PTY path or explicit operator handoff. There is no automatic second route after an ambiguous native attempt.

Bind each attempt to seat, harness session, process start/generation and attachment generation. Under the mux input arbiter, recheck the writer lease, native/composer ownership, modal/tool handoffs and operator draft/keystrokes immediately before dispatch, following [[harness-adapters]]' interactive-input guard. A shell prompt, reported idle glyph or quiet output is not readiness. For a plain shell/unknown harness without an explicit control/readiness handshake, autonomous input remains pending for operator handoff.

The supervisor remains the sole PTY writer. The broker journals a stable input ID before a paste/key/submit sequence, uses literal input and the advertised paste/keyboard modes, and excludes interleaved operator/injected writes. Keys go through mode-aware encoding; no `--all` equivalent or broadcast-domain expansion exists. The public MCP caller receives no writer-lease secret.

Fully PTY-written, native-queued, native-written, partial/unknown and application-acknowledged remain different evidence. A disconnected/cancelled call is queried using its original `op_id` and input ID. Never repeat Enter or switch to typing after unknown native submission. This contract does not claim a PTY write and its durable receipt are atomic.

## Swarms, mute and close

Default session discovery is swarm-scoped. Global channels/DMs require explicit destination grants; joining a channel does not grant owned-seat control. [[messaging]] defines persistent mute state and terminal-authored presence notices. Muting does not stop the child, move its sidebar row, or change active/idle/waiting/errored. It holds incoming swarm delivery and outgoing swarm publication without deleting either. Direct operator control remains available through the authenticated operator path; ordinary peer text cannot declare itself that path.

Mute/unmute is an idempotent policy transition under `expected_policy_epoch` (the current `SeatGrant.grant_epoch`, named `policy_epoch` in the seat record and presence notice per [[messaging]]). Commit policy, updated grant reference and the notice's durable outbox entry together, then change scheduling. Unmute makes eligible, previously unattempted work schedulable under current grants/priority; it cannot recall or replay an earlier native attempt. The terminal publishes each transition to the whole swarm, with normal known-recipient evidence, not a claim that every model read it.

Close, mute and harness activity are orthogonal. `seat_close` uses [[mux-daemon]]'s retained restoration record and deadline; closing is reversible during grace without a confirmation dialog. A live grace-period process continues consuming the spawn budget. Undo addresses that exact close generation; an expired close is a later resume operation, not proof of original-process survival.

**Oracle decision on purge (Vesper Greywind, 2026-10-21):** `seat_purge` = skip the remaining grace, terminate the process, and move the retained ring, checkpoints and events to cold blob storage; the terminal never deletes them ([[storage]], never-delete). Deletion is an out-of-band operator action. Operator only, from the GUI with a typed seat name as confirmation; not exposed to any MCP role, Oracle included. The Oracle can close (grace) and undo-close; it never deletes history.

## Swarm templates and spawn caps

The operator configures a hard per-swarm process cap and a machine/account aggregate cap. Oracle can read these through status; none of these tools raises them. Missing cap configuration is an explicit launch-configuration failure, not unlimited capacity. Count running, starting, reserved and closed-grace owned processes; remote owned seats count against their swarm plus the remote machine's applicable cap. shadows and retained dead history do not count as live processes.

`swarm_create(template_id="oracle-2-executors", …)` resolves an operator-approved template containing one Claude Oracle and two Codex Executors, approved launch profiles, role prompts, initial destination grants and layout. It is one tool call and one durable parent operation, with deterministic member IDs/input launch request IDs. Validate the template, paths, roles, delegation and all budgets, then **atomically reserve the whole launch count** before any child starts. Concurrent spawn/split/template operations use the same reservation ledger.

Process creation is not a transaction across three OS children. Return each member's `reserved / launching / live / failed / unknown` state and a partial outcome if necessary. Query original launch IDs before retrying; never relaunch an ambiguous member. Release only reservations proved unused or processes proved exited. Do not kill successfully launched members as an implicit rollback.

A new Oracle template does not inherit more authority than its delegated template grants. Existing children are discovered after logic restart before allocating more capacity. A supervisor must not accept an unreserved create from a stale logic lease. Profile/cwd validation and argument-vector launching prevent this tool becoming an undocumented host-wide shell interface.

## Audit and failure recovery

Append a durable audit attempt before admitting an Oracle effect, and an outcome/receipt afterward. Include actor account/device/seat and process generation, grant/policy epoch, tool, operation ID, target IDs/revisions, request digest plus redacted parameter summary, admission/cap decision, timestamps and the observed effect/evidence. Record denials and mutation attempts, not just successes. Operator mutations enter the same ledger.

Store the ledger in private runtime state with retention/access policy, outside Git. Do not copy screen contents, message text, secrets or whole launch environments into audit by default. A digest is correlation, not encryption or evidence that low-entropy content cannot be guessed. `audit_read` returns only authorized bounded metadata.

If durable admission/audit storage fails, reject before starting an effect. Persist the operation/argument digest so repeating the same `op_id` returns its prior state; same ID with different arguments is a conflict. After crash/reconnect, reconcile supervisor/native receipts rather than converting missing outcome records into “not executed.” If effect certainty cannot be recovered, keep `unknown` and require explicit operator handling. Audit is not claimed tamper-proof against the machine owner or a compromised same-UID process.

## it2 evidence and mapping

Local installed evidence: iTerm2 `3.7.6`, bundled `/Applications/iTerm.app/Contents/Resources/utilities/it2` version `1.0.0`. Use this path, not another unrelated executable named it2. Commands below were help/version/plist reads only; no iTerm API connection, screen capture, pane input or lifecycle action was performed.

| Commands actually run | Result / boundary |
|---|---|
| Bundled `it2 --help`, `it2 session --help` and `it2 --version`; plist version read | Exit 0; root aliases/groups, 20 session verbs and the versions above. |
| `it2 session <verb> --help` for each of the 20 verbs listed below | All exit 0; target arguments and flags inspected. |
| `it2 {window,tab,app,monitor,profile} --help` | All exit 0; related control/event/profile trees inspected. |
| Detailed helps: window `list,new,close,focus,move,resize,fullscreen,arrange`; tab `list,new,close,goto,select,move,next,prev`; app `broadcast,get-focus`; monitor `output,prompt,activity,variable,keystroke`; profile `apply` | 24 commands, all exit 0. Profile parent help also lists `list,set,show`; no runtime behavior was tested. |
| `claude --help` / `claude --version`; `codex --help` / `codex --version`; `codex mcp --help` / `codex mcp add --help` | All exit 0; Claude 2.1.304 / Codex 0.163.0. Help inspection only, no model run or MCP configuration mutation. |

Complete session-verb mapping; a mapped concept is not a promise to clone every it2 feature:

| it2 session verb(s) / aliases | Observed interface and MCP decision |
|---|---|
| `list` / `ls` | `--json` and formatting; becomes `seat_list` with explicit scope/paging. |
| `read` | `--session`, `--lines` selects visible rows; becomes `screen_read`. It is not scrollback paging. |
| `capture` | `--output` and optional `--history` writes text; becomes `history_read` pages, without arbitrary output-file writes. |
| `send` / `send` | Literal text without newline; `seat_input_text(submit=false)` under guard. |
| `run` / `run` | Text plus carriage return; `seat_input_text(submit=true)` under guard. No command exit-status claim. |
| `split` / `split,vsplit` | `--session`, optional `--vertical` / profile; `seat_split` with our explicit V/H semantics. |
| `close` | `--session`, `--force`; `seat_close` uses our decided grace policy rather than an implicit forced termination. |
| `restart` | Targeted session restart; product recovery belongs to the mux/adapter generation-aware resume policy, not an unrestricted repeat-command tool. |
| `focus` | Positional full session ID; `seat_focus` uses explicit owned seat and GUI IDs. |
| `set-name`, `set-color` | Targeted label/tab colour; `seat_update` changes presentation only. |
| `get-status`, `get-background-tasks` | Stored status JSON/count; `seat_get/self_status` exposes normalized adapter provenance instead of treating it as composer readiness. |
| `set-status` | Status, colour, detail, task count, turn-open and expiry flags; reserve authoritative observations for adapter ingestion. No arbitrary MCP activity setter. |
| `get-var` | Named session variable read; expose approved metadata in `seat_get`, not all internal variables. |
| `set-var` | Arbitrary variable mutation has no initial MCP equivalent; explicit role/permission tools own authority changes. |
| `clear` / `clear` | Clearing the foreground screen is not a history deletion/control requirement here; no initial tool. |
| `copy` | Clipboard selection is outside this control catalogue. |
| `add-clipping`, `archive-clippings` | Session clipping lists are outside this catalogue; use the existing evidence records. |

Other root aliases `new,newtab` and window/tab new/list/select/close/move map conceptually to our swarm/tab/seat/layout tools. Application quit/hide/activation/theme/fullscreen/window geometry, broadcast domains, auth/config/profile administration, notifications and clippings are not exposed as arbitrary host-wide controls. `monitor output/prompt/activity/variable` informs the adapter/subscription boundary, not a perpetual MCP tool response; no keystroke recorder is added.

The [Python Session API](https://iterm2.com/python-api/session.html) supplies `async_activate`, `async_close`, `async_send_text`, `async_set_name`, profile/variable operations and `async_split_pane`. Structured scrollback reads use `async_get_line_info` with `async_get_contents(first_line, number_of_lines)` in a transaction; screen reads/streamers are viewport snapshots. `async_send_text(..., suppress_broadcast=True)` is available in Python, while the inspected native send/run help has no such flag. These are reference mechanisms, not a product dependency or runtime verification.

## Verification and implementation gaps

Original M document checks on 2026-10-21, before the purge decision and S sweep: `ls .bboxy/specs/mcp-control-plane.md && grep -c '^| ' .bboxy/specs/mcp-control-plane.md` exited 0 and returned 56 table rows. The Python document checker parsed all three JSON blocks across the MCP/messaging specs, checked fences/whitespace, 26 unique catalogue tools and coverage of all 20 it2 session verbs; final exit 0. Its first catalogue assertion exited 1 because it also matched the later it2 inventory; restricting it to the catalogue section corrected the check. `git diff --check` exited 0. These are historical document checks. Runtime acceptance remains future work:

- Forged/stale actor/grant, a cached hidden tool and a foreign Oracle cannot gain authority; cross-swarm defaults, destination scopes and revocation hold.
- Competing GUI/Oracle inputs, native-unknown outcomes and replaced process generations never cause a duplicate paste/Enter.
- Concurrent templates/splits/spawns cannot exceed caps; restart reconciles partial launches without duplicate children.
- Paged history reports reflow/eviction gaps; worker tools and shadow records cannot expose another seat's logs.
- Mute transitions commit notices and hold traffic; unmute neither drops held work nor retries unknown native attempts.
- Close retains the same process during grace, expires under logic outage and restores the recorded position; no MCP role can purge retained data.
- Both real harness clients connect to the fixture server, render bounded successful/error results and reconnect without duplicating mutations.
- An equivalent direct seat-socket request and MCP call traverse the same authorization/guard/journal path and expose matching operation evidence; neither bypasses the bus with an independent write route.

**Gap:** no MCP bridge/server, capability handoff, audit store or runtime acceptance test was implemented/run by this specification task. Input readiness remains dependent on the explicit adapter proof and its unresolved tests in [[harness-adapters]].

**Gap:** audit retention/storage ceilings, template/profile registration and platform launch-cap accounting need concrete implementation/configuration; no latency/throughput measurements or live remote-control claim is made.

Scope: this file plus the specifically requested swarm-mute amendment in [[messaging]] and close lifecycle amendment in [[mux-daemon]]. No runtime, harness config, UI, task-board or provider changes; no commits.

— Nyx Copperwing <EXECUTOR>

## Task tools (operator decision 2026-10-21, see [[work-items]])

| Tool | Parameters | Mechanism | Roles |
|---|---|---|---|
| `task_delegate` | `op_id, task_path, seat_id \| spawn_spec, role?` | Rewrite frontmatter `assignee`, `delegated_by`, `status: claimed`; send the `task.delegate` message through the adapter; with `spawn_spec`, reserve capacity, spawn, then delegate in one op. | O |
| `task_reassign` | `op_id, task_path, seat_id` | Rewrite `assignee`; `task.release` to the old seat, `task.delegate` to the new. | O |
| `task_status` | `task_path \| task_id` | Frontmatter plus the delivery evidence of the last task message. | O; E C for own tasks |
| `task_list` | `swarm_id?, status?, assignee?` | Index query over `.bboxy/`. | O; E C for own tasks |
| `task_accept` | `op_id, task_path` | `status: accepted`, `task.accept` to the seat. | O |
| `task_reject` | `op_id, task_path, reason` | `status: rejected`, `task.reject` to the seat. | O |
| `task_report` | `op_id, task_path, summary, commit?, verify_result?` | The responsible seat's report: `status: reported`, `task.report` to `delegated_by`, sidebar notice on the delegating seat. | E C for own task; O |

Catalogue total: 32 tools. Workers now have seven: the four from before plus `task_status`, `task_list`, `task_report` on their own tasks.

## Artifact tools (operator decision 2026-10-21, see [[work-items]] Artifacts)

| Tool | Parameters | Mechanism | Roles |
|---|---|---|---|
| `artifact_claim` | `op_id, kind, ref, digest?, task?` | Write `.bboxy/artifacts/<ulid>.md` with `produced_by` = caller seat, link from the task; the caller becomes responsible. | O E C |
| `artifact_release` | `op_id, artifact_id, to` | Hand responsibility to another seat; both seats get a notice. | O; E C for own artifacts |
| `artifact_list` | `swarm_id?, kind?, produced_by?, task?` | Index query. | O E C |
| `artifact_get` | `artifact_id` | Record plus provenance chain. | O E C |

Catalogue total: 36 tools. Workers: eleven.

## Orientation tools (operator decision 2026-10-21: "each agent can look up what its job is, what it is working on and what is next; the Oracle looks up who has what")

| Tool | Parameters | Mechanism | Roles |
|---|---|---|---|
| `orient` | none | The caller's own card from the execution graph: name, role {name,class}, swarm, where, current task (status, path, verify command), next tasks in its queue (claimed or assigned to it, ordered by `depends_on`), what it is blocked on, its subscriptions, who it may message (its edges in the communication graph), its unread count, its open artifacts. One call, one answer, enough to resume cold. | O E C |
| `graph_read` | `graph: execution \| communication \| task \| markdown \| artifact \| messaging \| provenance, swarm_id?, filter?` | The named graph from the index as nodes and weighted edges, bounded; `execution` is who has what and blocked on what, `communication` is who may talk to whom (the policy the Oracle sets). | O; E C for `task` and `markdown` scoped to their own items |
| `graph_set_edge` | `op_id, graph: communication, from, to, channel: dm \| channel, allowed: bool` | The Oracle edits the communication graph; written to the swarm object and enforced as may-send-to in grants. | O |

Catalogue total: 39 tools. `.bboxy/graph.md` is the hand-maintained ancestor of `graph_read execution` and `communication`; once the index exists the file is generated, not edited.
