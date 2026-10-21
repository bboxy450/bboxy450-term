# Specification ownership and consistency

2026-10-21 · Nyx Copperwing <EXECUTOR> · assignment S and the “seats are sockets” addendum · Oracle review required for the Gap lines below.

Seven technical specs reviewed; the UX spec is included as the eighth contract because technical specs depend on its accepted vocabulary and interactions. [[vision]] owns milestone scope: rendering, local messaging, MCP and visibility first; remote/cloud, cross-instance messaging and Google/global features follow later. Describing their contracts does not move them into the first prototype.

## Ownership and dependencies

| Spec | Owns | Depends on |
|---|---|---|
| [[terminal-core]] | Fork/parser, viewport model, terminal protocols, checkpoint boundary and renderer split. | [[mux-daemon]] for PTY/history; [[subagent-detection]] for OSC consumers; [[text-rendering]] and [[monorepo-weavie]] for rendering/build references. |
| [[mux-daemon]] | Supervisor/logic ownership, seat sockets/event bus, process generations, close lifecycle, rings/checkpoints, state streams and GetHistory. | [[terminal-core]], [[identity]], [[messaging]] and [[harness-adapters]]. |
| [[messaging]] | Signed envelopes, persistence/delivery evidence, role priority, destination grants, mute and published presence. | [[identity]], [[mux-daemon]], [[harness-adapters]] and [[ux-prototype]]. |
| [[mcp-control-plane]] | Role-filtered tools, bus-client control, grant enforcement, bounded reads/input, spawn caps and control audit. | [[identity]], [[messaging]], [[mux-daemon]] and [[harness-adapters]]. |
| [[identity]] | Root/device/seat grants, custody, enrollment, epochs/revocation, paper recovery, Google binding and signed seat roster (seat git signing remains assumed). | [[messaging]] for envelopes; [[mux-daemon]] and [[mcp-control-plane]] for process-bound capabilities. |
| [[harness-adapters]] | Launch/observe/wake adapters, observation mapping, input guard and per-harness integration gaps. | [[subagent-detection]], [[messaging]], [[mux-daemon]] and [[mcp-control-plane]]. |
| [[subagent-detection]] | Normalized seat/subagent activity, OSC/shim ingestion and restart reconstruction. | [[harness-adapters]], [[mux-daemon]] and [[ux-prototype]]. |
| [[ux-prototype]] | Accepted visual/interaction reference and lab behavior; supersedes [[ux-pick]] wherever they differ. | [[vision]], [[ux-flow]], [[mux-daemon]] for close semantics and [[messaging]] for mute/presence. |

## Mechanical corrections applied

- Swarm names/IDs and shadow terminology agree across technical specs. The envelope now names `swarm_id` at `messaging.md:75`. The two surviving old token names in UX are explicitly marked retired vocabulary; no token or lab code was renamed.
- Settled decisions replace obsolete gaps: identity owns custody/enrollment/revocation and the decided nonce; priority keeps Executor = Creativity and local-root-only Oracle authority; purge is GUI/operator-only with typed seat name and is absent from the MCP catalogue (`mcp-control-plane.md:157`). The catalogue now has 25 tools; older M verification counts are labeled historical.
- Detection now distinguishes GUI, logic and supervisor failure (`subagent-detection.md:162`). Claude 7501 task reports and Codex hook-only subagent lifecycle follow the accepted adapter evidence. Canonical activity is `waiting`; main working/blocked and StopFailure mappings agree. Resumed display provenance is historical until a live observation arrives.
- Hook-launch files survive close grace until process exit; missing off-PTY events are no longer described as automatically reconstructed by the next delta. Stale MSG/MUX line-number references now name sections. The malformed JWT citation in [[identity]] is repaired.
- Terminal search covers retained history, with evicted bytes unavailable (`terminal-core.md:17`). The shared executable can depend on term-core through daemon mode; its GUI rendering path does not depend on a daemon Term model.
- The declared `seat_focus` output schema admits its promised GUI/layout fields. Socket ownership, generation binding, bounded framing/subscriptions and guarded input are explicit at `mux-daemon.md:38`; MCP is a client of that bus at `mcp-control-plane.md:9`.

Activity (`active/idle/waiting/errored`), lifecycle (`open/closed_grace/closed_retained`), observation grade (`reported/shim/inferred`), operation outcomes and message evidence are separate domains. Adapter enum spellings are mapped to the six messaging evidence strings; none means model receipt without application acknowledgment.

## Decisions left to the Oracle

**Gap S1 — canonical durable IDs.** `subagent-detection.md:17` and `harness-adapters.md:261` choose ULID; `messaging.md:73` and its swarm field choose UUID; `mux-daemon.md:67` defers to an identity model that does not select the encoding. Select the durable seat/swarm ID types and wire spelling; a harness conversation UUID is a different identifier.

**Gap S2 — two swarm-creation contracts.** `mcp-control-plane.md:60` creates an empty swarm while `mcp-control-plane.md:68` launches a template. `ux-prototype.md:32` specifies a third default. Decide whether the two tool operations remain distinct, their names and how the GUI default maps to them. The legacy tool name remains pending that decision; it was not silently collided with `swarm_create`.

**Gap S3 — positive composer readiness.** `harness-adapters.md:136` and `mcp-control-plane.md:143` require an explicit current readiness/control indication, but `harness-adapters.md:301` returns Ready from reported idle/done, turn state and negative checks. Define the positive composer-ownership proof per harness, or keep autonomous fallback pending. No new handshake or automatic submission was selected here.

**Gap S4 — hook attribution across PTYs.** `subagent-detection.md:121` and `harness-adapters.md:279` reroute a mismatched hook using its asserted `seat=`, while `mcp-control-plane.md:23` rejects environment labels as authentication. Hook/socket payloads do not bind a verified process/attachment generation. Decide authenticated attribution for nested/forwarded PTYs and SSH; inherited labels and a local socket path cannot prove remote ownership.

**Gap S5 — shipped hook packaging.** `subagent-detection.md:76` and `harness-adapters.md:267` specify a separate static executable; `mux-daemon.md:11` specifies one shipped executable with private process roles. Decide whether the hook is a private entry point or an explicitly allowed helper artifact; no new entry-point flag was invented.

**Gap S6 — launch overrides versus preserved policy.** `harness-adapters.md:164` and `harness-adapters.md:287` propose hook-trust bypass/trust-hash installation; the Claude row at `harness-adapters.md:163` forces inbound accept and a tool allow flag. `mcp-control-plane.md:15` and `messaging.md:142` require preserving effective policy. Decide supported launch-scoped installation and approval behavior; no home config or trust setting was changed.

**Gap S7 — observation order, freshness and restart time.** `harness-adapters.md:63` is documented as a supervisor sequence for PTY observations and wall-clock for off-PTY ones, while `mux-daemon.md:109` defines an ordered record cursor. `subagent-detection.md:26` and its subagent timestamps are process-local Instants, while `mux-daemon.md:134` persists derived activity. Define separate sequence/time domains, freshness comparison and restart-safe timestamp encoding; no conversion can be inferred from a type name.

**Gap S8 — nested-row model.** `subagent-detection.md:34` excludes pending/idle, but `harness-adapters.md:231` creates pending rows and `harness-adapters.md:75` allows Idle task observations. The transitions at `harness-adapters.md:235` and `harness-adapters.md:237` also write detail and map a paused report to “blocked”, absent from the declared Subagent fields/waiting-kind enum. Decide binding lifecycle versus activity and the normalized fields/mappings.

**Gap S9 — hook field projection.** `harness-adapters.md:271` specifies normalized `agent_id`/`last_message` fields, while `harness-adapters.md:233` and the SubagentStop row consume raw `tool_response.agentId`/`last_assistant_message`. Specify the shim's raw-to-normalized projection for each event so the authoritative join and completion detail survive the whitelist.

**Gap S10 — hook payload limits.** `harness-adapters.md:267` accepts 64 KiB before filtering; `harness-adapters.md:279` rejects OSC frames over 4 KiB. The background-task array has no emitted aggregate bound and base64 expands it. Select a bounded emission/projection or explicit rejection/truncation rule; the stdin cap does not establish a valid OSC size.

**Gap S11 — shell integration milestone.** `terminal-core.md:66` defers OSC 133 parsing/semantic prompts, while `harness-adapters.md:302` requires OSC 133 B as the positive zsh input condition. Decide whether adapter parsing ships initially or automatic shell fallback remains unavailable until that integration exists.

**Gap S12 — seat movement and swarm grants.** `mcp-control-plane.md:58` is a layout operation without a grant transition, while `identity.md:42` binds a seat to `swarm_id` and `mcp-control-plane.md:27` uses swarm scope. Specify whether move is confined to the current swarm; if cross-swarm movement is intended, its grant/queued-work policy needs an Oracle decision.

**Gap S13 — control metadata producers.** `mcp-control-plane.md:57` requires a known `gui_id`/layout revision and `mcp-control-plane.md:59` requires a record revision. `mux-daemon.md:55` declares lifecycle/process fields but no record/layout revision or connected-GUI discovery contract. Name their authoritative owner and discovery path before tool callers can construct valid requests; no extra discovery tool/schema was invented.

**Gap S14 — off-PTY recovery.** `harness-adapters.md:283` permits dropped socket events with no spool; `mux-daemon.md:151` requires revalidation before new input. Define the adapter's current-state reattach/snapshot path, especially for pi/omp and missing hook lifecycle deltas. A parser checkpoint plus the next arbitrary hook event is not a complete live readiness proof.

**Gap S15 — Oracle rate cap versus worker progress.** `messaging.md:122` calls the 10/s bucket a starvation bound, while `messaging.md:120` disclaims bounded worker latency. A rate cap alone does not prove progress if receiver dispatch capacity is at or below the admitted Oracle rate. Decide whether the guarantee is only rate limiting or whether a worker-progress contract is required; the scheduler policy was left unchanged.

**Gap S16 — explicit interrupt control.** `harness-adapters.md:163` requires an explicit Oracle interrupt tool for native `Now`/steer, while `mcp-control-plane.md:53` has no interrupt/priority argument and the catalogue declares no interrupt tool. Decide the explicit control surface or keep that path unavailable; no tool or implicit interruption was added.

## Verification and scope

Document checker — exit 0: nine Markdown files; four parsed JSON examples; 199 resolved wikilinks; balanced fences/inline code and clean whitespace; 25 unique MCP tools, exactly four worker tools, no callable purge; eight ownership rows, 16 Gap lines and 54 in-range file:line citations. The required vocabulary grep returns two explicitly retired UX token references and zero matches after excluding those notes. Final diff whitespace checks are run against both the current worktree and the accepted identity-spec baseline. This sweep changes specifications only. Runtime acceptance criteria remain unrun; no renderer, harness, key, provider, transport or process-control behavior is certified by these edits. The Oracle's kittens event-model research was not present when the socket addendum was drafted and must be reconciled when available.

No source/runtime/configuration/UI file is in scope, and no commit is made by this Executor. Other sessions may commit these documents while the sweep runs.

— Nyx Copperwing <EXECUTOR>

## Oracle decisions on S1–S16 (Vesper Greywind, 2026-10-21)

Apply on next touch of each file; no separate pass unless noted.

| Gap | Decision |
|---|---|
| S1 | **ULID** for every durable id: seat, swarm, tab, op, message. UUID nowhere. |
| S2 | Two tools stay distinct: `swarm_create` (empty) and `swarm_template` (launch a template). The GUI's default from ux-prototype is the GUI's choice of template, not a third contract. |
| S3 | Ready for injection requires a **positive** indication from the adapter's own channel (Claude: sessions file idle and no pending dialog; Codex: app-server status; zsh: OSC 133 B). Reported idle/done alone is display-only. Fix harness-adapters:301 to match :136. |
| S4 | A hook or OSC 7502 frame is attributed to the PTY it arrived on; that is authoritative. An asserted `seat=` is honoured only on the socket fallback and only with the seat's capability token. Env labels never authenticate. |
| S5 | One shipped executable; the hook is a private entry point (`bboxy450-term hook`) referenced by absolute path in the per-launch settings overlay. |
| S6 | Per-launch settings overlay only. Never edit user settings files or trust hashes. If a harness demands trust approval, surface it to the operator; no bypass. |
| S7 | The supervisor record cursor is the single order. Off-PTY observations are stamped with the cursor at ingestion by the logic process; wall-clock is metadata. |
| S8 | Nested rows show running subagents only. Pending is not a row; an Idle task observation ends the row. |
| S9 | The shim emits raw fields verbatim under `raw`; projection to normalized fields happens in the logic process. |
| S10 | OSC frames ≤ 4 KiB; larger payloads go over the socket fallback up to 64 KiB; background-task arrays truncated to 32 entries plus a total count. |
| S11 | OSC 133 parsing ships in term-core v0.1; the zsh ZDOTDIR overlay emits it; it is the positive zsh readiness signal. |
| S12 | `seat_move` is confined to its swarm. A cross-swarm move is close-grace in the source plus spawn in the target with a new grant. |
| S13 | mux-daemon owns `layout_revision` and `record_revision`, maintained by the logic process; `gui_id` is assigned at attach. Add the fields to mux-daemon. |
| S14 | No spool for off-PTY events. On reattach the adapter takes a snapshot from its channel (sessions file, app-server status, extension state query) and revalidates. |
| S15 | The Oracle token bucket is a cap, not a progress proof. Add a weighted fair queue: at least one worker dispatch per four Oracle dispatches when both are pending. |
| S16 | Add `seat_interrupt` to the catalogue, Oracle only, mapping to native Now/steer, with `op_id` and audit. 26 tools. |

## Oracle decisions on the UI proposals Nyx flagged (2026-10-21)

| Proposal | Decision |
|---|---|
| Audit lines in the swarm stream | Yes. The swarm stream shows audit entries from the durable audit log to readers authorized for that swarm; it is a projection, not a second store. mcp-control-plane: add `audit_read` scope = swarm members. |
| Listener count on seat rows | Yes, as telemetry from the socket broker: count of attached external clients per seat, no identities shown. |
| Interrupt and restarted signal types | Yes. `interrupt_sent` (one-shot, from `seat_interrupt`) and `restarted` (one-shot, on process generation change) join the signal table in ux-prototype; both are notices, not states. |
| First-run module | Must match identity.md: select and pin an existing GPG primary; never create a root or show 24 words as the root. The paper key is recovery material for the leaf chain only. |

## Oracle decision: `task_delegate` (operator, 2026-10-21)

MCP gets a first-class **delegate**, not just a message. Tasks, specs and epics are Markdown files with YAML frontmatter in the repo: [[work-items]]. `task_delegate{op_id, seat_id | spawn_spec, task: {title, brief_path | text, allowed_paths, acceptance, verify_cmd}, role?}`: the mux records a durable task bound to the seat, writes the role header and brief into the seat's harness through the adapter (same path as a message, oracle-class priority), tracks state `assigned → acknowledged → running → reported → accepted | rejected | reassigned` from the seat's events and its report, and exposes `task_status`, `task_list`, `task_accept`, `task_reassign`. A task is a row under the seat in the sidebar ([[ux-prototype]] to draw it; Lumen) and a line in the swarm stream. `spawn_spec` lets the Oracle delegate to a seat that does not exist yet: spawn + delegate in one op under the spawn cap. Oracle-class only, except `task_status`/`task_list` for workers on their own tasks. Nyx: add to mcp-control-plane.md on next touch; 30 tools.
