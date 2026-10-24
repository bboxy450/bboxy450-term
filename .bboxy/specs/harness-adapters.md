---
id: "51FPRZP3WWTHWZZJ453YR1C11Q"
type: "spec"
number: 86
title: "Spec: harness adapters — launch, observe, feed, wake"
created: "2026-10-21T19:51:15Z"
updated: "2026-10-24T00:10:38Z"
---
# Spec: harness adapters — launch, observe, feed, wake

2026-10-21 · converged for Vesper Greywind <ORACLE> from two research passes · status PROPOSED.

[[vision]] · [[harness-touchpoints]] · [[harness-touchpoints-exec]] · [[iterm-harness-plugin]] · [[subagent-detection]] · [[messaging]] · [[mux-daemon]]

Source tags used in every cell: **HT §n** = [[harness-touchpoints]] section n (Oracle subagent pass, local binaries + icq source, `(memory)` where marked there). **HTE «name»** = [[harness-touchpoints-exec]] section (Nyx Copperwing, installed artifacts + primary docs, no runtime test). **IHP §n** = [[iterm-harness-plugin]]. **SD (x)** = [[subagent-detection]]. **MSG** = [[messaging]] "Harness delivery and PTY fallback" and its "Delivery, rooms and restart behavior" evidence-state list. **MUX** = [[mux-daemon]]. Where HT and HTE disagree the reconciliation is written out in place and collected in §0.

## 0. Reconciliations

| # | HT said | HTE / other said | Spec decision |
|---|---|---|---|
| R1 | Hooks are the state channel; "hooks win when present" (HT §1.2) | OSC 7501 is primary, hooks secondary (HTE «Primary PTY observation», SD (b), [[vision]] “Harness touch points”) | 7501 is `reported` grade and wins. Hooks are `shim` grade. "Hooks win" means hooks beat PTY cadence heuristics, nothing more. |
| R2 | Codex has no system-prompt flag; role goes through `AGENTS.md`, a profile `model_instructions_file` (memory), or the first user message (HT §2.1) | `-c developer_instructions=…` is the documented additive surface; `model_instructions_file` replaces instructions and is a different operation; an initial user prompt is not system instructions (HTE «Codex CLI») | Role prompt = `-c developer_instructions=<text>` plus the machine's `AGENTS.md`. Never the first user message. `model_instructions_file` not used. Key name on 0.163 still unverified locally (§6). |
| R3 | Codex hook events = 10 names found in binary and trust records; no Notification/PermissionRequest (HT §2.2) | Official docs list permission, interrupt and subagent events too (HTE «Codex CLI») | Bind the 10 verified names. Parse permission/interrupt events if they arrive; do not depend on them. Gap. |
| R4 | `Notification idle_prompt` → waiting-on-input (HT §1.2 table) | cc-status maps it to idle (IHP §4); SD (d) idle; it arrives about a minute after the turn, conditional on user absence (HTE «Claude Code») | `idle_prompt` → idle, `turn_open=false`. Never a `waiting_kind`. Its absence proves nothing. |
| R5 | No `ESC ] 133` literal in Claude; none of the four harnesses emit OSC 7/133/633 (HT §1.5, §6) | Earlier SD (b.5) claimed "Claude Code emits it" | Not relied upon. OSC 133 comes only from the zsh seat's terminal-installed shell integration (§5). Runtime-composed emission by Claude stays a Gap. |
| R6 | pi and omp have extension lifecycle events; omp's title carries state (HT §3.2, §4.2) | Earlier SD (b.5) grouped pi/omp with zsh/unknown under the PTY heuristic; HTE confirms no 7501/21337 in pi/omp (HTE «Primary PTY observation») | pi and omp get their own adapter: a terminal-installed extension is their `reported` channel, omp's title is an `inferred` hint. The heuristic is for zsh and unknown only. |
| R7 | Composite rule: "at rest" + idle hook within the last few seconds → inject (HT §6 conclusion) | Quiescence and tty flags are negative checks only; a hookless harness gets operator handoff, no automatic submission (HTE «Interactive-input guard» 3, 5; MSG; MUX «Supervisor ↔ logic protocol») | HTE wins. HT's composite stays as the negative gate and as the `inferred` display state. It never grants typing. |
| R8 | **Gap:** is `agent_settled` exposed to extensions (HT §3.2, Open gaps) | `Pi/dist/core/extensions/types.d.ts:761` defines `agent_settled` in the extension event types (HTE «Pi 1.0.0») | Resolved: exposed. pi idle = `agent_settled`, not `agent_end`. |
| R9 | Codex emits 7501 (HT via [[vision]] “Harness touch points”) | Codex 0.163 reports one state for itself: no `id=`, `title=`, `kind=` adjacent to 7501 (IHP §7); no Codex task-ID encoder found (HTE «Primary PTY observation») | Codex subagent rows come from hooks `subagent_start`/`subagent_stop` only. 7501 gives the main row. |
| R10 | `Stop` → idle (HT §1.2) | `Stop` is a turn boundary, not a certificate that stdin belongs to the composer (HTE «Claude Code») | Both. Display state and composer readiness are two predicates (§3, §5). `Stop` sets the first, never the second. |
| R11 | icq receipts `Written` / `Queued` / `Dispatched` (HT §5) | Six evidence states: sender-stored, recipient-stored, native-queued, native-written, native-unknown, application-acknowledged (MSG «Delivery, rooms and restart behavior») | Adapter `wake()` returns a MSG evidence state. Mapping in §1 and §2. |
| R12 | `SessionEnd` → `exited` seat state (HT §1.2) | Seat states are exactly active, idle, waiting, errored ([[vision]]); process generation is mux state (MUX «Durable seat identity and launch mapping») | Process liveness is a mux field on the seat record, not a seat state. `SessionEnd` → idle with `harness_session_id` kept (SD (d)). |

## 1. Adapter trait

Rust-shaped. One `impl` per harness. The mux logic process owns the instances (MUX «Ownership and failure boundaries»: "native harness adapters" live in logic, not the supervisor).

```rust
pub enum Harness { Claude, Codex, Pi, Omp, Zsh, Unknown }
pub enum Role { Oracle, Executor, Creativity }

/// Everything the supervisor needs to spawn the seat. No $HOME mutation.
pub struct LaunchPlan {
    pub command: portable_pty::CommandBuilder,     // argv + cwd
    pub env: Vec<(String, String)>,                // BBOXY_SEAT_ID, BBOXY_SOCK, BBOXY_ROLE, TERM_PROGRAM…  (SD (c))
    pub overlay_files: Vec<TempFile>,              // 0600 settings/mcp/extension files, per seat, retained during close grace and deleted after process exit
    pub harness_session_id: Option<String>,        // chosen up front when the harness allows (Claude --session-id, pi --session-id)
    pub answer_osc_probes: bool,                   // always true: ESC]7501;?ESC\ is echoed on every seat (SD (b.1))
}

/// Observe channels, declared in priority order per adapter.
pub enum ObserveChannel {
    Osc7501,                 // reported, PTY-ordered                 (IHP §7, SD (b.1))
    Osc7502Shim,             // shim, PTY-ordered, bboxy-hook         (SD (b.2))
    ShimSocket,              // shim, off-PTY fallback                (SD (b.4))
    NativeApi(NativeKind),   // reported, off-PTY: Claude sessions file, Codex app-server, pi/omp extension stream
    Osc21337, Osc9_4,        // inferred hints                        (IHP §6, HT §6)
    TitleGrammar,            // inferred: omp "π > | ⠋ | !"            (HT §4.2)
    PtyHeuristic,            // inferred                              (HT §6, HTE «PTY-only signals»)
    ProcessExit,             // supervisor SIGCHLD / master EOF       (HT §6)
}
pub enum NativeKind { ClaudeSessionsFile, ClaudeMessagingSocket, CodexAppServer, CodexNotify, PiExtension, OmpExtension, ZshIntegration }

pub enum Grade { Reported, Shim, Inferred }      // SD (a): reported > shim > inferred

/// A normalized observation. `at` is the supervisor record sequence for PTY-ordered channels,
/// wall-clock for off-PTY ones (MUX «Supervisor ↔ logic protocol»: record sequences order non-output events).
pub struct Observation {
    pub channel: ObserveChannel,
    pub grade: Grade,
    pub at: Seq,
    pub process_generation: u64,                   // MUX «Durable seat identity and launch mapping»: PIDs are never identity
    pub kind: ObservationKind,
}
pub enum ObservationKind {
    Main { state: WireState, waiting_kind: Option<WaitingKind>, detail: Option<String>, turn_open: Option<bool> },
    Task { agent_id: String, state: WireState, waiting_kind: Option<WaitingKind>, title: Option<String> },
    TaskCleared { agent_id: Option<String> },      // None = clear all
    HookEvent(ShimEvent),                          // raw 7502 / socket payload, adapter maps it
    Title(String),
    Exit { code: Option<i32> },
}
pub enum WireState { Idle, Working, Blocked, Done, Error }   // 7501 vocabulary (IHP §7); other channels are mapped onto it
pub enum WaitingKind { Permission, Question, Auth }

/// Native endpoint bound to one process generation. Stale after restart/resume/switch (guard point 1).
pub struct Attachment {
    pub seat_id: SeatId,
    pub harness_session_id: String,
    pub process_generation: u64,
    pub attachment_generation: u64,                // pi/omp bridge generation; bump on session switch/branch/tree (HT §3.3, HTE «OMP»)
    pub endpoint: Endpoint,                        // ClaudeSocket(path) | CodexAppServer(sock, thread_id) | ExtensionStream(conn) | None
}

/// MSG «Delivery, rooms and restart behavior» evidence states, verbatim names.
pub enum Evidence {
    SenderStored, RecipientStored, NativeQueued, NativeWritten, NativeUnknown, ApplicationAcknowledged,
}
pub struct WakeOutcome { pub evidence: Evidence, pub native_id: Option<String>, pub note: &'static str }

pub enum Priority { Next, Later, Now }            // Claude socket vocabulary (HT §1.4); mapped per harness in §2

pub trait HarnessAdapter: Send + Sync {
    fn harness(&self) -> Harness;

    /// Build the spawn plan: role prompt, per-seat overlays, env, session id, resume variant.
    fn launch(&self, seat: &SeatRecord, role: Role, cwd: &Path, resume: Option<&str>) -> LaunchPlan;

    /// Priority-ordered. The state machine (§3) uses this order plus Grade to resolve conflicts.
    fn observe_channels(&self) -> &'static [ObserveChannel];

    /// Called by the VTE for every OSC on this seat's PTY; returns zero or more observations.
    fn on_osc(&mut self, seat: &SeatRecord, osc: &Osc) -> Vec<Observation>;
    /// Called by the mux socket/native readers.
    fn on_native(&mut self, seat: &SeatRecord, ev: NativeEvent) -> Vec<Observation>;
    /// Pure mapping: observation → (seat delta, subagent delta). Table in §3.
    fn map_state(&self, obs: &Observation) -> StateDelta;

    /// Bind the native wake endpoint for the current process generation. Fails closed.
    fn attach(&mut self, seat: &SeatRecord) -> Result<Attachment, AttachError>;

    /// Deliver a peer message natively. Never types. Returns MSG evidence.
    /// Ambiguous result => NativeUnknown and the caller must NOT fall back to the PTY (guard point 2).
    fn wake(&self, att: &Attachment, msg: &Envelope, prio: Priority) -> WakeOutcome;

    /// Composer-readiness for the PTY fallback (§5). Display state is an input, never the answer.
    fn input_guard(&self, seat: &SeatRecord, arbiter: &InputArbiterView) -> Readiness;

    /// Supported resume command for a dead process generation (MUX «Durable seat identity and launch mapping»), e.g. `claude --resume <id>`.
    fn resume_command(&self, seat: &SeatRecord) -> Option<LaunchPlan>;
}

pub enum Readiness {
    Ready { ready_token: ReadyToken },             // bound to seat+session+generation+attachment generation
    NotReady(&'static str),                        // keep RecipientStored, show pending badge
    OperatorHandoff,                               // hookless/unknown: never automatic (guard point 5)
}
```

### Interactive-input guard — contract (verbatim, HTE «Interactive-input guard — proposed contract»)

1. Bind a ready indication to **seat ID + harness session ID + process start + attachment generation**. Reject stale indications after restart/resume/switch. ICQ's process/descriptor checks above supply a reference mechanism; screen text is not the identity channel.
2. Prefer native queue/RPC/extension delivery, even while the model is active, when its documented queue semantics permit it. Keep admission, dispatch and model-consumption evidence distinct. Do not fall back to typing after an ambiguous native submission: the first attempt may already have arrived.
3. For PTY fallback, require a current adapter/hook indication that the composer owns input, no outstanding modal/approval request, and no active tool/input handoff. Under the mux input arbiter, recheck generation, foreground group, buffered parser/render state and operator draft/keystrokes. Output quiescence and tty flags are negative checks, never the positive proof.
4. If eligible, render the peer message as literal data, retain original signed bytes separately, respect the advertised paste mode, serialize the paste/submit sequence, and journal a stable delivery ID before writing. A possible partial write remains unknown; terminal bytes alone are not a durable exactly-once delivery mechanism.
5. For a hookless harness with no explicit ready handshake, leave automatic submission pending for operator handoff. Do not fabricate certainty from a blinking cursor. This is the consequence of the proof gap, not a tested implementation.

`input_guard` implements 1 and 3; `wake` implements 2; the supervisor `WriteInput` path (MUX «Supervisor ↔ logic protocol») implements 4; `Readiness::OperatorHandoff` implements 5.

### Wake evidence mapping (R11)

| Adapter result | MSG evidence | Source |
|---|---|---|
| Claude socket: full newline-JSON frame written, no `type:"error"` reply | `native-written` | icq `Written`, "receiver_acceptance: unverified" — HT §1.4, HTE «Claude Code» |
| Claude socket: `peer_idle_notice` for our `msg_id` received | `application-acknowledged` | HT §1.4 `peerFeatures: notify_idle`; subscribe frame is a Gap |
| Codex `thread/queue/add` → `queuedSubmission{id, clientUserMessageId}` matching ours | `native-queued` | icq `Queued{queue_id}` — HT §2.4, HTE «Codex CLI» |
| Codex `turn/started` on that thread after our queue id drained (`thread/queue/changed`) | `application-acknowledged` | HT §2.4 notification list; correlation rule is a Gap |
| pi/omp extension: `sendUserMessage` returned on our stream | `native-written` | icq `Dispatched` = "native callback invoked, not queue acceptance" — HT §3.3 |
| pi/omp extension: our message id seen in `input`/`before_agent_start` | `application-acknowledged` | HT §3.2 event list; exact event is a Gap |
| Any adapter: socket error, timeout after write, generation changed mid-call | `native-unknown`; never retried automatically | MSG «Delivery, rooms and restart behavior», MUX «Supervisor ↔ logic protocol» |
| PTY fallback: supervisor receipt "fully PTY-written" | `native-written` | MUX «Supervisor ↔ logic protocol» |
| PTY fallback: receipt "partial/unknown" | `native-unknown` | MSG «Harness delivery»; MUX «Supervisor ↔ logic protocol» |
| Guard not satisfied | stays `recipient-stored` | MSG «Harness delivery»: "No ready-state evidence means keep the message queued" |

## 2. Per-harness table

### 2a. Launch and wake

| Harness | Role prompt at launch | Other launch flags | Wake (native) | Priority mapping |
|---|---|---|---|---|
| **Claude Code** 2.1.304 | `--append-system-prompt <text>` (keeps default prompt; `--system-prompt` replaces) — HT §1.1, HTE «Claude Code». Snapshot default `on`: the prompt is frozen across `--resume` until compaction (HT §1.1, HTE). Second channel that also works on resume: SessionStart hook `additionalContext` — HT §1.2 (memory). | `--settings <0600 file>` overlay: `hooks.*` → `bboxy-hook`, `crossSessionInbound:"accept"`, `statusLine` optional — HT §1.1, SD (b.2). `--mcp-config <file>` + `--allowedTools mcp__bboxy__*` — HT §1.1. `--session-id <uuid>` minted by the mux, `--name <character name>` — HT §1.1. Resume: `claude --resume <uuid>`. **Never `--bare`**: no hooks, no socket — HT §1.1. | Unix socket `/tmp/cc-socks/<pid>.sock` (path also in `~/.claude/sessions/<pid>.json.messagingSocketPath`), newline JSON: optional `{"type":"auth","token":…}` then `{"type":"user","uuid","msg_id","priority":"next","message":{"role":"user","content":TEXT}}` — HT §1.4; icq omits `auth` (same-uid attribution) — HT §1.4, HTE «Claude Code». Requires effective `crossSessionInbound: accept` — HT §1.4. | `Next`→`"next"` (default, icq precedent), `Later`→`"later"`, `Now`→`"now"` (interrupts the running turn, help text — HT §1.4). `now` only on an explicit Oracle interrupt tool call, never automatic (MSG «Priority without deleting messages»). |
| **Codex CLI** 0.163.0 | `-c developer_instructions="<text>"` additive — HTE «Codex CLI» (docs; R2). `AGENTS.md` at `~/.codex/AGENTS.md → ~/.agents/AGENTS.md` carries shared instructions — HT §2.1. | `-c mcp_servers.bboxy.command=…` etc. — HT §2.1 (icq `src/launch.rs:78-97`). `-C <cwd>`. Hooks: `~/.codex/hooks.json` is home-level; per-launch attempt = inline `-c hooks.…` tables + `--dangerously-bypass-hook-trust` — HT §2.1, HTE «Codex CLI» ("hooks.json or inline tables"); Gap. `-c notify=["bboxy-hook","--agent","codex-notify"]` for `approval-requested`/`agent-turn-complete` — HT §2.3. `-c 'tui.terminal_title=[…]'` — HT §2.5. Resume: `codex resume <thread-id>` — HT §2.1. Preferred future shape: terminal-owned `codex app-server --listen unix://…` + TUI `--remote unix://…` — HT §2.1; Gap. | App-server over `~/.codex/app-server-control/app-server-control.sock` (or the socket the TUI holds, `lsof -U`): `initialize{experimentalApi:true}` → `initialized` → `thread/read{threadId}` (status `idle\|active`) → `thread/queue/add{threadId, clientUserMessageId, input:[{type:"text",text}]}` → `queuedSubmission` — HT §2.4, HTE «Codex CLI». CLI equivalent `codex queue --thread <UUID\|name> --message <text>` — HT §2.4, HTE. Thread id from `CODEX_THREAD_ID` via the shim's SessionStart `session_id`, or `thread/started` — HT §2.1, §2.4. | `Next`/`Later` → `thread/queue/add` (mux orders before submission; the queue has no priority field — HT §2.4). `Now` → `turn/steer` (method exists in binary — HT §2.4); not automatic. |
| **pi** 1.0.0 | `--append-system-prompt <text\|file>` (repeatable) — HT §3.1, HTE «Pi 1.0.0». Extension `before_agent_start` can append prompt sections — HT §7 matrix, HTE (`types.d.ts:693`). | `-e <path to bboxy extension>` per launch — HT §3.1. `--session-id <id>`, `-n <name>` — HT §3.1. Seat tools via the extension's `pi.registerTool` instead of MCP (home-level `~/.pi/agent/mcp.json` must not be edited) — HT §3.1, §3.3. `AGENTS.md` read by default — HT §3.1. | Terminal-installed extension connects **out** to `BBOXY_SOCK` on `session_start` with `{seat, session_id, generation}`; mux pushes `{op:"dispatch", message_id, text, deliver_as}`; extension calls `pi.sendUserMessage(text, {deliverAs, expandPromptTemplates:false})` — API HT §3.1 (`types.d.ts:1215-1223`), icq pattern HT §3.3 (`bridge.mjs:55`) with the socket direction reversed (§4). | `Next`→`followUp`, `Later`→`nextTurn`, `Now`→`steer` — `deliverAs` values HT §3.1. `steer` not automatic. |
| **omp** 18.4.12 | `--append-system-prompt`, `--system-prompt-template <handlebars>` — HT §4.1, HTE «OMP 18.4.12». | `-e` or `--hook <file>` (alias for an extension) — HT §4.1. `--config <overlay.yml>` repeatable — HT §4.1. `-n` name. Keep title on (no `--no-title`, `tui.titleState` default) because the title carries state — HT §4.1, §4.2. `--from-claude`/`--from-codex` reserved for the harness-switching future — HT §4.1. | Same extension, omp entry point; identity/generation refreshed on `session_switch`/`session_branch`/`session_tree` — HT §4.2, HTE «OMP». Use omp's own `FollowUpOptions` type for `expandPromptTemplates` (`SendUserMessageOptions` at `:418` lacks it) — HTE «OMP» Gap. | Same as pi — `sendUserMessage` delivery choices HTE (`agent-session-types.d.ts:418`). |
| **zsh** | none (no model) | `zsh -l` with `ZDOTDIR=<per-seat overlay dir>` whose `.zshenv`/`.zshrc` source the user's files then add precmd/preexec hooks emitting OSC 133 A/B/C/D and OSC 7. Mechanism: shell integration gives exact prompt/command boundaries — HT §6 (OSC 133 row), HTE «PTY-only signals» (prompt zones). Spec decision; no research doc tested it. | none native. PTY fallback only, under §5 zsh rule. | n/a |
| **unknown** | none | argv as given by the operator; env from SD (c) still injected; probe still answered. | none. `Readiness::OperatorHandoff` always — guard point 5. | n/a |

### 2b. Observe, title, subagent nesting

| Harness | Observe channels, priority order (grade) | Title source | Subagent nesting source |
|---|---|---|---|
| **Claude Code** | 1. OSC 7501 main + `id=` task reports (reported) — IHP §7, SD (b.1); Claude emits only after the probe reply, ≤2 s window — IHP §7. 2. OSC 7502 from `bboxy-hook` (shim): 11 events SD (b.2); payload fields IHP §9. 3. `~/.claude/sessions/<pid>.json` `status ∈ idle\|busy\|waiting\|shell\|stopped\|exited` + `waitingFor` (reported, off-PTY, kqueue/poll) — HT §1.3. Also yields `messagingSocketPath`, `name`. 4. Messaging-socket `peer_idle_notice` per sent message (reported, per message) — HT §1.4. 5. OSC 9;4 (inferred; Claude writes `9;4;0` on exit/restore — IHP §7). OSC 21337 encoder exists but is gated off (`Mtn(){return!1}`) — IHP §6. 6. PTY heuristic (inferred) — HT §6. 7. Process exit. | OSC 0 with the session name (`--name`) — HT §1.5. No state in title (Gap HT §1.5). | 7501 `id=<sanitized agent id>:title=<Agent description>:state=working\|blocked` per running task, `state=clear:id=` on end, cap 32, main row id-less — IHP §7, SD (b.1). Shim: `SubagentStart{agent_id,agent_type}` (no description), parent `PreToolUse Agent{tool_input.description,tool_use_id}` → `PostToolUse` `tool_response.agentId` is the only hook-side join — IHP §9, SD (d). Optional third: watcher on `~/.claude/projects/…/<session>/subagents/agent-<id>.meta.json` — IHP §9; not v0.1. |
| **Codex CLI** | 1. OSC 7501 main report only, `app=codex`, states `idle\|working\|blocked` (reported) — IHP §7, HTE «Primary PTY observation»; whether Codex probes first is a Gap. 2. App-server notifications `thread/status/changed` (incl. `waitingOnApproval`), `turn/started`, `turn/completed{completed\|interrupted\|failed}`, server requests → `serverRequest/resolved` (reported, off-PTY) — HT §2.4, HTE «Codex CLI». 3. OSC 7502 via hooks `session_start session_end user_prompt_submit pre_tool_use post_tool_use stop subagent_start subagent_stop pre_compact post_compact` (shim; per-launch install is a Gap) — HT §2.2, IHP §1, SD (b.3). 4. `notify` program: `agent-turn-complete` → idle, `approval-requested\|async-question\|plan-mode-prompt` → waiting (shim) — HT §2.3. 5. OSC 21337 `status=Idle\|Working\|Waiting` (inferred hint) — IHP §6, SD (b.1). 6. OSC 9 desktop notify (ignored). 7. PTY heuristic. 8. Exit. | OSC 2 from `[tui].terminal_title` items — HT §2.5. Whether `run-state` is a valid item is a Gap. | Hooks `subagent_start`/`subagent_stop` only (R9). 7501 carries no task ids in 0.163 — IHP §7. App-server `item/started\|completed` for agent items: Gap. |
| **pi** | 1. Extension events over the `BBOXY_SOCK` stream (reported): `agent_start`/`turn_start` → active; `agent_settled` → idle (not `agent_end`, which can be followed by retry/queued work) — HT §3.2 (`docs/json.md:48`), HTE «Pi» (`types.d.ts:761`, `agent-session.js:671`); `ui_prompt_start`/`ui_prompt_end` → waiting(question) — HTE (`types.d.ts:766`); `tool_call` on an `ask`-style tool → waiting(question) — HT §3.2 (tool name stability is a Gap); `tool_execution_end.isError` → errored (transient) — HTE (`:831`); `session_shutdown` → idle, process gone. 2. OSC 9;4 only if the user's `terminal.showTerminalProgress` is on (inferred) — HT §3.2. 3. PTY heuristic. 4. Exit. No 7501 (HTE). | OSC 0 `"pi - <sessionName> - <cwdBasename>"` — HT §3.2 (`pi-tui/dist/terminal.js:421`). No state. | none known. Gap. |
| **omp** | 1. Extension events as pi, plus `approval requested/resolved` with session/tool-call ids, `session_stop`, `agent_end{willContinue}` (reported) — HTE «OMP» (`shared-events.d.ts:162-166`, `:95`, `extensions/types.d.ts:723`); `session_switch`/`session_branch` bump attachment generation — HT §4.2. 2. OSC 0 title grammar `π > label` idle, `π ⠋ label` working (80 ms spinner), `π ! label` attention (inferred, zero-install) — HT §4.2 (`title-generator.ts:724-835`). 3. OSC 1337 `SetUserVar=<session file var>=<base64>` → session file path (reported identity) — HT §4.2 (`:694`). 4. PTY heuristic. 5. Exit. No 7501 (HTE). | OSC 0 `π <state glyph> <label>`; label is `-n` name or an AI-generated session title — HT §4.2. | none known. Gap. |
| **zsh** | 1. OSC 133 `A` prompt start, `B` prompt end = accepting input, `C` command start, `D;<exit>` command end (reported, from our injected integration) — HT §6, HTE «PTY-only signals». 2. `tcgetpgrp(slave) == shell pid` (no foreground child) and termios `ICANON\|ECHO` set (inferred; verified only that a busy Claude seat shows `-icanon -echo` — HT §6, HTE). 3. Child process tree (inferred) — HT §6. 4. Quiescence (negative only). 5. Exit. | OSC 0/2 from the integration (`cmd` while running, `cwd` at prompt) or the user's own precmd. OSC 7 for cwd. | none. A foreground child is shown as `detail`, not a row. |
| **unknown** | 1. OSC 7501 if it ever appears (probe is always answered). 2. OSC 0/2 title text. 3. PTY heuristic (inferred): drawn with the hollow glyph — SD (b.5). 4. Exit. | OSC 0/2 verbatim. | none. |

## 3. State machine

Vocabulary: seat `state ∈ active | idle | waiting | errored` (SD (a); canonical value `waiting`), `waiting_kind ∈ permission | question | auth | none`, `turn_open`, `detail`, `status_source ∈ reported | shim | inferred`, `subagents: Vec<Subagent>` in start order, never resorted — SD (a). Wire → sidebar: `working→active`, `blocked→waiting`, `idle→idle`, `done→idle` with `detail=last message`, `error→errored` — SD (a).

### 3a. Events → seat state

| Event (channel, grade) | state | waiting_kind | detail | turn_open | Source |
|---|---|---|---|---|---|
| 7501 `state=working` (reported) | active | none | msg | true | IHP §7, SD (d) |
| 7501 `state=blocked:kind=k` | waiting | k | msg | true | IHP §7 (`dIo` map) |
| 7501 `state=done` | idle | none | msg | false | SD (d) |
| 7501 `state=error` | errored | none | msg | false | IHP §7 (API error) |
| 7501 `state=idle` / `state=clear` (id-less) | idle | none | keep | false | SD (d) |
| Codex `turn/started` (reported) | active | none | keep | true | HT §2.4 |
| Codex `turn/completed{completed}` | idle | none | keep | false | HTE «Codex» |
| Codex `turn/completed{failed}` | errored | none | keep | false | HTE «Codex» |
| Codex server request outstanding / `thread/status/changed waitingOnApproval` | waiting | permission (question for user-input requests) | keep | true | HTE «Codex» |
| Codex `serverRequest/resolved` | active | none | keep | true | HT §2.4 |
| Codex `thread.status systemError` | errored | none | keep | — | HT §7 matrix |
| Claude sessions file `status=busy` | active | none | keep | true | HT §1.3 |
| Claude sessions file `status=waiting`, `waitingFor=w` | waiting | map w like `dIo` (permission/question), else question | w | true | HT §1.3, IHP §7 |
| Claude sessions file `status=idle` | idle | none | keep | false | HT §1.3 |
| Claude sessions file `status=shell` | active | none | "shell" | — | HT §1.3 (Gap: what sets it) |
| pi/omp `agent_start`/`turn_start` | active | none | keep | true | HT §3.2 |
| pi `agent_settled`; omp `agent_end{willContinue:false}` then idle per `waitForIdle` | idle | none | keep | false | HTE «Pi», «OMP» |
| pi/omp `ui_prompt_start`, `ask` tool call, omp approval requested | waiting | question (permission for omp approval) | tool/prompt text | true | HT §3.2, HTE |
| pi/omp `ui_prompt_end`, omp approval resolved | active | none | keep | true | HTE |
| pi/omp `tool_execution_end.isError` | errored (transient: cleared by next active) | none | error | true | HTE |
| Shim `SessionStart` | idle | none | "" | false | SD (d) |
| Shim `UserPromptSubmit` | active | none | keep | true | SD (d) |
| Shim `PreToolUse`/`PostToolUse`, no `agent_id` | active | none | summary (IHP §3 rules) | true | SD (d) |
| Shim `PermissionRequest` | waiting | permission | "Allow " + summary / question / "Review proposed plan?" | true | IHP §3-4, SD (d) |
| Shim `Notification permission_prompt` | waiting | permission | message | true | SD (d) |
| Shim `Notification agent_needs_input` / `elicitation_*` | waiting | question | message | true | SD (d), IHP §9 |
| Shim `Notification idle_prompt` | idle | none | keep | false | R4 |
| Shim `Stop` | idle, or active if any subagent active | none | last_message | false | SD (d) |
| Shim `StopFailure{error}` | errored | `auth` if error ∈ {authentication_failed, oauth_org_not_allowed, cloud_credential_error} else none | error | false | IHP §7, §9 |
| Shim `SessionEnd{reason}` | idle; `harness_session_id` kept | none | reason | false | SD (d), R12 |
| Codex `notify agent-turn-complete` | idle | none | last-assistant-message | false | HT §2.3 |
| Codex `notify approval-requested\|async-question\|plan-mode-prompt` | waiting | permission / question / permission | keep | true | HT §2.3 |
| omp title `π ⠋` / `π !` / `π >` (inferred) | active / waiting(question) / idle | — | keep | — | HT §4.2 |
| OSC 21337 `status=Working\|Waiting\|Idle` (inferred) | active / waiting / idle | none | `detail=` | — | IHP §6 |
| zsh OSC 133 `C` / `B` / `D;n≠0` (reported) | active / idle / errored until next `B` | none | command / "" / exit code | — | HT §6 |
| PTY heuristic: bytes flowing / quiet 2 s at prompt / `?`-`[y/N]` line + raw + quiet 10 s (inferred) | active / idle / waiting(question) | — | keep | — | SD (b.5), HT §6 |
| Process exit (supervisor) | errored if exit ≠ 0, else idle; mux marks the process generation dead | none | exit code | false | HT §6, MUX «Restart and reattachment» |

### 3b. Nested subagent rows

| Event | Subagent row | Source |
|---|---|---|
| parent shim `PreToolUse Agent` | create `pending{description, tool_use_id, agent_type}` | SD (d), IHP §9 |
| shim `SubagentStart{agent_id, agent_type}` | create/bind `active`, `started_at`; bind to the oldest `pending` of the same `agent_type` | SD (d) |
| parent shim `PostToolUse Agent` `tool_response.agentId` | authoritative `tool_use_id → agent_id` bind; set `description` | IHP §9 |
| 7501 `id=X:state=working:title=T` | upsert `active`, `description=T` (title is the Agent description, `…` descriptions dropped by the emitter) | IHP §7 |
| 7501 `id=X:state=blocked:kind=k` | waiting, k (`remote_agent needs_input` → question, `plan_ready` → permission, `paused` → blocked) | IHP §7 |
| child shim event carrying `agent_id` (`PreToolUse`, `PermissionRequest`, `Notification`) | active with detail / waiting | SD (d), IHP §9 |
| shim `SubagentStop{agent_id, last_assistant_message}` | done, `ended_at`, detail | SD (d) |
| 7501 `state=clear:id=X` | done if not already, `ended_at` | SD (d) |
| parent shim `Stop{background_tasks[]}` | listed agent with `status ∉ running` → done; active row absent from the list → done | SD (d), IHP §3 |
| Codex hooks `subagent_start`/`subagent_stop{session_id?}` | create active / done; no description (Gap) | HT §2.2, R9 |
| `SessionStart source=resume` | clear all rows; live adapter observations rebuild them (7501 task reports for Claude, hook events for Codex) | SD (e) |

Rows stay 30 s after `ended_at`, then collapse into the count; show all, collapsing past 8 into `+N` — SD (a).

### 3c. What wins on conflict

1. **Grade.** `reported` > `shim` > `inferred`. A lower grade never overwrites a higher-grade value younger than 5 s — SD (a). Within `reported`, the PTY-ordered channel (7501) wins over off-PTY channels (sessions file, app-server, extension stream) for the same instant, because PTY order is total with the harness's own output (SD (b.2) reason 3) and the sessions file is a coarser three-value projection of the same state machine (HT §1.4 `Uio` sub-kinds).
2. **Waiting is sticky.** Once any channel sets `waiting`, only a resolving event clears it: 7501 non-blocked report, `PostToolUse` or `PreToolUse` for the same `tool_use_id`, `serverRequest/resolved`, `ui_prompt_end`/approval resolved, `UserPromptSubmit`, or process exit. Quiescence never clears it. Reason: Enter during a dialog answers the dialog — HT §6 conclusion.
3. **Generation.** An observation whose `process_generation` is not the live one is dropped. Recovered display values retain historical provenance; only live observations establish a current grade — SD (e), MUX «Durable seat identity and launch mapping».
4. **Subagents.** 7501 `id=` reports are authoritative for existence and state; the shim supplies descriptions before 7501 has them and `agent_type`; `Stop.background_tasks[]` reconciles stragglers — SD (d). A row seen by 7501 is never deleted by a shim event, only marked done.
5. **Exit beats all.** Supervisor EOF/SIGCHLD overrides every channel; stale off-PTY readers (sessions file for a dead pid) are ignored — MUX «Restart and reattachment».
6. **Display ≠ readiness.** Nothing in 3a–3c produces a `ReadyToken`. That is §5 — R10, HTE «Primary PTY observation» ("An observed idle, done, clear, or blocked report is not permission to type").

## 4. The shim, seat identity, OSC 7502, socket fallback

Consistent with SD (b.2)–(b.4) and (c); restated here as the contract adapters depend on.

**Environment injected by the mux at launch** (hooks and extensions inherit the harness environment — IHP §9):

```
BBOXY_SEAT_ID=<ULID>            durable seat id
BBOXY_SOCK=<path>               mux unix socket (fallback for the shim; primary stream for pi/omp extensions)
BBOXY_ROLE=oracle|executor|creativity
TERM_PROGRAM=bboxy450-term  TERM_PROGRAM_VERSION=<semver>
```

**`bboxy-hook`**: one static Rust binary, no runtime deps — SD (b.2). Invoked with no args for Claude, `--agent codex` for Codex hooks (iTerm2 `cc-status` precedent — IHP §3), `--agent codex-notify` for the Codex `notify` program (HT §2.3). Reads hook JSON on stdin (≤64 KiB, icq precedent HT §1.2), keeps the whitelisted fields, writes one OSC to `/dev/tty`, exits 0 within milliseconds; never writes stdout (Claude parses it — SD (b.2)); must finish inside the 1.5 s `SessionEnd` budget — IHP §9.

```
ESC ] 7502 ; v=1:seat=<BBOXY_SEAT_ID>:ev=<hook_event_name>:d=<base64 json> ESC \
json = { session_id, agent_id?, agent_type?, tool_name?, tool_use_id?,
         description?,            // tool_input.description for Agent/Task
         summary?,                // cc-status toolCallSummary rules (IHP §3)
         notification_type?, permission_tool?, reason?, source?, error?,
         last_message?,           // ≤2048 bytes
         background_tasks?: [{id,type,status,description}] }
```

Parser rules as for 7501: both terminators, both introducers, discard over 4 KiB or unknown `ev`, skip malformed pairs — SD (b.1). `seat=` is carried even though the PTY already identifies the seat: a mismatch (seat A's hook arriving on PTY B) means a nested or forwarded tty; the mux then routes by the env value — SD (c).

**Why the PTY and not a socket for the common path**: the byte lands on the fd already mapped to `seat_id` (no env lookup, pid walk or cookie — IHP §2/§6 needed four hops); it survives ssh because the remote hook's tty flows back to our pane; ordering with harness output is total; one parser for 7501 and 7502 — SD (b.2). Precedent that hook processes can open `/dev/tty`: `cc-status` writes its OSC 9;4 ring there — IHP §5.

**Socket fallback** `BBOXY_SOCK`: `$XDG_RUNTIME_DIR/bboxy450/mux.sock`, or the mux's private state directory on macOS where `XDG_RUNTIME_DIR` is unset (MUX «Ring records, checkpoints and eviction»: runtime files live in private application state directories). `SOCK_STREAM`, newline JSON `{seat, ev, d}` with the same `d` payload — SD (b.4). Used when `isatty(/dev/tty)` fails, `BBOXY_SEAT_ID` is unset, or the tty write fails — SD (b.2). Owned by the mux logic process so it keeps receiving through a GUI restart — SD (b.4); on a logic restart the supervisor keeps PTYs and captures PTY hook output subject to bounded backpressure; the socket listener returns with logic (MUX «Restart and reattachment»). If both routes are unavailable, hooks exit 0 in <50 ms with no spool. Off-PTY state resynchronization remains a Gap; a later hook delta is not promised to contain full state — SD (e).

**pi/omp extension**: the same `BBOXY_SOCK`, long-lived connection, opened by the extension on `session_start` and reopened on omp `session_switch`/`session_branch`/`session_tree` with a bumped generation; closed on `session_before_switch|fork|tree|shutdown` — pattern HT §3.3 (`host.mjs:38-76`), direction reversed so the mux never discovers `$TMPDIR/icq-h-*` style sockets. Frames up: `{seat, session_id, generation, ev, d}` for every lifecycle event in 2b. Frames down: `{op:"dispatch", message_id, text, deliver_as}` → `sendUserMessage`; reply `{message_id, state:"dispatched"|"rejected", reason?}`. Dedup by `message_id` on the extension side — HT §3.3 (`bridge.mjs:48-57`).

**Codex shim install**: per-launch inline `-c hooks.<Event>=[…]` plus `--dangerously-bypass-hook-trust`, or a terminal-written `hooks.json` with computed `hooks.state.*.trusted_hash` in a profile — HT §2.1, §2.2, SD (b.3). Gap until exercised.

**Credentials**: the shim inherits `CLAUDE_CODE_MESSAGING_TOKEN` (HT §1.1). It is never put into 7502, a record or a log (MUX «Durable seat identity and launch mapping»). If Claude's `auth` frame turns out to be required (HT §1.4 Gap), the shim forwards the token once over `BBOXY_SOCK` on `SessionStart` and the mux keeps it in memory only.

## 5. PTY fallback

The PTY is the last resort and the only path for zsh. Everything here is gated by the guard contract in §1. Display state from §3 is an input, never the proof.

**Order of attempts** for a message to a seat (MSG «Harness delivery»): native `wake()` → if `NativeUnknown`, stop and show the message with its unknown outcome (guard 2, MSG «Delivery, rooms and restart behavior») → if native is unavailable (no attachment for the live generation), evaluate `input_guard` → `Ready` → supervisor `WriteInput` with a stable input id (MUX «Supervisor ↔ logic protocol») → `NotReady` → stays `recipient-stored`, row badge → `OperatorHandoff` → row badge with a paste affordance; the operator's keystroke is the submission.

**Readiness per harness** (`input_guard`):

| Harness | Positive evidence required (all) | Negative checks (any fails → NotReady) | Verdict otherwise |
|---|---|---|---|
| Claude, Codex, pi, omp | `reported`-grade `idle` or `done` for the live generation, younger than the harness's own refresh (7501 is re-emitted on every render — IHP §7; sessions file `updatedAt` — HT §1.3; app-server `turn/completed`; `agent_settled`); `turn_open=false`; no outstanding server request / `ui_prompt` / `waiting` (§3c rule 2); attachment generation unchanged (guard 1) | `tcgetpgrp(slave) != harness pid` (a tool or `!` shell owns input — HTE «PTY-only signals»); operator draft bytes since last prompt in the arbiter (guard 3); unclosed mode 2026 frame (HTE: unknown render state); bytes in the last 500 ms (HT §6 composite, negative only — R7); alt-screen just toggled (HT §6: `?1049l` = dropped to shell/editor) | `Ready` |
| zsh | OSC 133 `B` seen after the last `C`/`D` from our integration (prompt is accepting input — HT §6); `tcgetpgrp(slave) == shell pid`; termios `ICANON` and `ECHO` set (HT §6: line mode = shell or `read` prompt) | password prompt pattern: `ECHO` off (HTE: raw editors and password prompts both disable echo) → NotReady; operator draft since `B`; generation mismatch | `Ready` |
| unknown | none exists | — | `OperatorHandoff` always (guard 5) |

**Write sequence** (guard 4): journal `{input_id, seat, generation, message_id}` → if mode 2004 is on, `ESC[200~` + literal text + `ESC[201~`, else literal text with CR/LF normalized to CR → one `CR` → supervisor receipt. Full receipt → `native-written`; partial/unknown → `native-unknown`, never an automatic second Enter (MSG «Harness delivery», MUX «Supervisor ↔ logic protocol»). Operator keystrokes and injected input are serialized through the arbiter; a write that began is not interrupted by a higher-priority message (MSG «Priority without deleting messages»).

**Hazard the gate exists for**: typing into a busy Claude/Codex/pi/omp TUI is buffered by their editors, but Enter during a permission dialog answers the dialog — HT §6 conclusion. Hence rule 2 in §3c and the `turn_open=false` requirement.

**Unknown-harness heuristic** (display only, `status_source=inferred`, hollow glyph — SD (b.5)): `active` while bytes arrive; `idle` after 2 s quiet with a visible cursor, no mode 2026 frame open and no child processes; `waiting` when the last line ends in `?`, `[y/N]`, `(Y/n)` with raw mode and 10 s quiet; `errored` on exit ≠ 0 — SD (b.5), HT §6 composite. Stated limits, each from HT §6 / HTE «PTY-only signals»: idle ≠ waiting (no PTY bit separates them for a TUI); a long tool call with no output looks idle; spinners break quiescence while busy; termios flags say nothing for a raw-mode TUI; a visible cursor appears in dialogs too; child-process count misses a pure model turn; titles can be stale or spoofed; a DSR sent into the child's stdin is not a readiness poll; an ssh/tmux remote foreground process is not identified by the local process group. Therefore the heuristic never produces `Ready` — R7.

## 6. Gaps

- **Gap:** no live OSC 7501 probe/reply exchange captured; Claude's matcher accepts the echo, iTerm2's exact reply bytes beyond `?` unknown — IHP §7, SD Open, HTE «Primary PTY observation».
- **Gap:** whether Codex probes before emitting 7501 or emits unconditionally — IHP §7, HTE.
- **Gap:** Codex per-launch hook installation (inline `-c hooks.*` vs `hooks.json` + trust hash) — SD (b.3), HT §2.1/§2.2, HTE «Codex CLI».
- **Gap:** Codex hook event set beyond the 10 verified names (permission, interrupt documented, not in binary strings) — HT §2.2 vs HTE «Codex CLI» (R3).
- **Gap:** `developer_instructions` key accepted by `-c` on 0.163 locally (docs only) — HTE «Codex CLI»; HT §2.1 raised the `model_instructions_file` naming gap.
- **Gap:** whether `thread/queue/add` into an `active` thread steers or waits; the public app-server page does not document the method — HT Open gaps, HTE «Codex CLI».
- **Gap:** correlating a Codex queue id with the `turn/started` that consumes it (needed for `application-acknowledged`) — HT §2.4 list only.
- **Gap:** Codex `--remote unix://` TUI attach to a terminal-owned app-server — HT §2.1, untested in both passes.
- **Gap:** valid `[tui].terminal_title` items (is `run-state` one) — HT §2.5.
- **Gap:** Codex subagent descriptions: `subagent_start` payload fields, app-server agent item events — HT §2.2, R9.
- **Gap:** Claude `auth` frame: which setting flips `authRequired`; same-uid attribution sufficiency — HT §1.4, HTE «Claude Code».
- **Gap:** Claude `peer_idle_notice` subscribe frame — HT §1.4.
- **Gap:** Claude sessions file: what sets `status:"shell"`; `"waiting"` write paths — HT §1.3.
- **Gap:** Claude OSC 133/`SEMANTIC_PROMPT` emission at runtime (no literal in binary) — HT §1.5, SD (b.5) (R5).
- **Gap:** Claude system-prompt snapshot: does a changed `--append-system-prompt` on `--resume` take effect before compaction — HT §1.1, HTE «Claude Code».
- **Gap:** Claude `priority:"later"` semantics; `"now"` behavior beyond the help text — HT §1.4.
- **Gap:** pi `ask` tool name stability for waiting detection — HT §3.2.
- **Gap:** pi per-launch MCP (none found); extension `registerTool` as replacement untested — HT §3.1.
- **Gap:** omp `expandPromptTemplates` on `SendUserMessageOptions` vs `FollowUpOptions` — HTE «OMP 18.4.12».
- **Gap:** omp OSC 9;4 default; `--mode rpc` parity with pi; ACP version — HT §4 Open gaps.
- **Gap:** pi/omp subagent or task events for nested rows — neither pass found any.
- **Gap:** zsh seat shell integration via `ZDOTDIR` overlay is a spec decision with no research behind it; interaction with the user's own prompt hooks untested — this spec.
- **Gap:** `Stop.background_tasks[]` item `type` for in-process subagents — SD Open.
- **Gap:** check/write race between a ready indication and the tty write without a consumable handshake; user-draft preservation, modal transitions, process replacement, partial writes — HTE «Interactive-input guard», MSG «Harness delivery», MUX «Supervisor ↔ logic protocol».
- **Gap:** every runtime transition/injection claim in both passes is unexercised; this spec is a convergence of static evidence, not a compatibility certification — HTE «Verification and provenance ledger», HT Open gaps.

Links: [[vision]] [[harness-touchpoints]] [[harness-touchpoints-exec]] [[subagent-detection]] [[messaging]] [[mux-daemon]]

## Correction 2026-10-22 (W2.3, checked against installed Codex 0.163 and Claude 2.1.304 schemas)

Codex hook keys are launched in CamelCase; the decoder keeps snake_case aliases. Hook child IDs are independent of the parent `session_id` and may be full UUIDs. Implementation: `crates/bbt-mux/src/adapters/`; evidence `.bboxy/runs/w2-3-evidence.log`.
