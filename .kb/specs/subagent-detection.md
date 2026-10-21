# Spec: seat state and subagent detection

2026-10-21 · derived from [[iterm-harness-plugin]] · owner Vesper Greywind <ORACLE>.

[[vision]] · [[ux-sidebar]] · [[harness-touchpoints]] · [[harness-adapters]]

Status: PROPOSED. Facts cited as `research §N` point into [[iterm-harness-plugin]].

## 0. Decision in one paragraph

Two channels, both on the seat's own PTY, no env lookup, no socket round-trip for the common path. **Primary: OSC 7501**, the Program Status Protocol Claude Code (≥2.1.295) and Codex (≥0.163) already speak when the terminal answers the probe `ESC]7501;?ESC\` — Claude carries its main agent and running subagents with `id`, `state`, `title`; the evidenced Codex 0.163 report covers only its main row (harness-adapters R9). **Secondary: a hook shim the terminal installs** that writes its own OSC (`OSC 7502`, our namespace) to `/dev/tty`, for harnesses whose 7501 is absent or stale and to catch events 7501 does not carry (tool detail, permission detail, session boundaries). A unix socket is the fallback only when `/dev/tty` is not ours (ssh, tmux, cloud). Unknown harnesses get a PTY heuristic. iTerm2's `it2`-over-websocket hop is dropped.

## (a) Seat state model

```
Seat {
  seat_id:      ULID            // durable, minted by the mux daemon at launch
  harness:      claude | codex | pi | omp | zsh | unknown
  role:         oracle | executor | creativity
  name, color                   // character name, [[ux-sidebar]] §3
  state:        active | idle | waiting | errored
  waiting_kind: permission | question | auth | none
  detail:       String?         // one line, ≤192 bytes, last tool call or last message
  turn_open:    bool
  status_source: reported | shim | inferred
  last_change:  Instant
  harness_session_id: String?   // Claude session_id / Codex thread id, for resume
  subagents:    Vec<Subagent>   // ordered by started_at, never resorted
}
Subagent {
  agent_id:     String          // harness id, sanitized [A-Za-z0-9_.+-]{1,32}
  description:  String          // Agent tool `description`, ≤192 bytes
  agent_type:   String?         // general-purpose, Explore, custom name
  state:        active | waiting | done | errored
  waiting_kind: permission | question | auth | none
  started_at:   Instant
  ended_at:     Instant?
  source:       reported | shim
}
```

Rules:

- Sidebar state words map from the wire: `working→active`, `blocked→waiting`, `idle→idle`, `error→errored`, `done→idle` with `detail` = last message (a seat is not "done", its turn is). [[ux-sidebar]] §2: a state change never moves a row.
- A subagent row stays listed for 30 s after `ended_at` with state `done`/`errored`, then collapses into the seat's count. iTerm2 drops it on `state=clear:id=` immediately and lists at most four; we keep order and show all, collapsing past 8 into `+N` (1000-seat budget, [[ux-sidebar]] §8).
- Swarm and tab roll-ups: `waiting` count bubbles up; `active` subagents count toward the seat's activity glyph.
- `status_source=reported` outranks `shim` outranks `inferred`. A lower source never overwrites a higher one younger than 5 s.

## (b) Ingestion channels

### b.1 OSC 7501 (reported) — Claude Code and Codex, no install needed

Grammar (research §7, verbatim from the Claude Code encoder):

```
ESC ] 7501 ; ? ESC \                              probe (harness → terminal)
ESC ] 7501 ; ? ESC \                              reply  (terminal → harness, echo)
ESC ] 7501 ; state=<s>:app=<name>[:id=<id>][:kind=<k>][:progress=<0-100>][:title=<b64>][:msg=<b64>] ESC \
ESC ] 7501 ; state=clear ESC \                    drop every report of this program
ESC ] 7501 ; state=clear:id=<id> ESC \            drop one task
s    ∈ idle | working | blocked | done | error | clear
k    ∈ permission | question | auth               (only with blocked)
id   = [A-Za-z0-9_.+-]{1,32}; absent = the main agent
title, msg = base64(UTF-8), ≤192 / ≤2048 bytes decoded
terminator: ESC \ or BEL; pairs split on ':'; '=' splits key/value once
```

Terminal duties: answer the probe on the same PTY **immediately** (Claude Code waits ≤2 s and never emits 7501 without a reply — research §7). Parse per research §7 iTerm2 diagnostics: discard a report over 4 KiB, unknown state, malformed id; skip a malformed pair, keep the rest. `app=` names the main row when a seat has subagents (`claude-code`, `codex`). Accept both OSC terminators and both `ESC ]` and `0x9d`.

Mapping: id-less report → `Seat.state/detail/waiting_kind`; `id=` report → upsert `Subagent{agent_id=id, description=title, state}`; `state=clear:id` → `ended_at=now`; `state=clear` → end all. `progress` is ignored in v0.1.

Also parse **OSC 21337** (`status=`, `indicator=`, `status-color=`, `detail=`, `;`-separated, `\;` escaped) and **OSC 9;4** (progress `st 0-4`) because Codex writes 21337 for its idle/working/waiting word and both harnesses write 9;4; treat them as `inferred`-grade hints under 7501.

### b.2 Hook shim (shim) — Claude Code

The terminal ships one static binary `bboxy-hook` (Rust, no runtime deps) and at seat launch writes a per-seat settings overlay passed with `--settings <file>` (Claude Code merges it; the user's `~/.claude/settings.json` is untouched, unlike iTerm2 which edits it — research §1). Events bound, all with `"command": "<path>/bboxy-hook"`, no matcher:

`SessionStart SessionEnd UserPromptSubmit PreToolUse PostToolUse PermissionRequest Notification Stop StopFailure SubagentStart SubagentStop`

(`SubagentStart` added over iTerm2's set: it is the earliest signal a subagent exists; the description arrives on the parent's `PreToolUse Agent`/`PostToolUse` with `tool_response.agentId` — research §9.)

`bboxy-hook` reads the JSON on stdin, keeps only these fields, and writes **one OSC to `/dev/tty`**, then exits 0 within milliseconds. Payload is JSON, base64-encoded so no byte can terminate the OSC:

```
ESC ] 7502 ; v=1:seat=<BBOXY_SEAT_ID>:ev=<hook_event_name>:d=<base64 json> ESC \
json = { session_id, agent_id?, agent_type?, tool_name?, tool_use_id?,
         description?,            // tool_input.description for Agent/Task
         summary?,                // cc-status-style one-liner (research §3 toolCallSummary rules)
         notification_type?, permission_tool?, reason?, source?, error?,
         last_message?,           // ≤2048 bytes
         background_tasks?: [{id,type,status,description}] }
```

Why an OSC on the seat's own PTY, not a socket: (1) the PTY *is* the seat identity — the byte arrives on the fd the mux already maps to `seat_id`, so no env lookup, no pid walk, no cookie (iTerm2 needs `TERM_SESSION_ID` → `it2` → websocket → builtin function, four hops and a 10 MB CLI, research §2/§6); (2) it survives ssh and `claude` running inside a remote shell, because the remote hook still owns a tty that flows back to our pane; (3) ordering with the harness's own output is total, so `Stop` cannot overtake the last `PostToolUse`; (4) it is the same path 7501 uses, one parser. Risk: a hook can run when `/dev/tty` is not ours (hook spawned detached, or inside tmux with a different client). Mitigation: `bboxy-hook` checks `isatty` and that `BBOXY_SEAT_ID` is set; otherwise it falls back to the socket (b.4). iTerm2's `cc-status` already does the tty write for its progress ring (research §5), so the tty is reachable from hook processes in practice.

The shim never writes to stdout (Claude Code would parse it) and never blocks (timeout budget on `SessionEnd` is 1.5 s, research §9).

### b.3 Hook shim — Codex

Codex hooks: `~/.codex/hooks.json` events `session_start session_end user_prompt_submit pre_tool_use post_tool_use stop subagent_start subagent_stop pre_compact post_compact` (research §1: trust records in `config.toml:148-178` name exactly this set). Codex requires each hook to be trusted (`hooks.state.<path>:<event>:<i>:<j>.trusted_hash`), so the terminal writes its hooks file **and** the matching trust hash, or launches Codex with a `--config hooks=…` override. **Gap:** whether Codex accepts a per-launch hooks file path; the Codex docs must be checked before implementation. Same shim binary with `--agent codex` (iTerm2 precedent, research §3), same OSC 7502. Codex already speaks 7501/21337 for its main row; the shim also supplies subagent lifecycle events, detail text and session boundaries (harness-adapters R9).

### b.4 Socket fallback

`$XDG_RUNTIME_DIR/bboxy450/mux.sock` (unix, `SOCK_STREAM`, newline-delimited JSON `{seat, ev, d}`), same payload as 7502. Used when `/dev/tty` is not a tty or the write fails. Socket path is injected as `BBOXY_SOCK`. The mux daemon owns the socket, not the GUI, so it keeps receiving through a GUI restart.

### b.5 Zsh and unknown harness — PTY heuristic (inferred)

Pi and omp use the reported extension channels in [[harness-adapters]] §2b (R6); their title/progress hints remain inferred. Zsh gets the terminal-installed OSC 133 integration there; Claude OSC 133 emission is not relied upon (R5). Unknown-harness display heuristics are owned by [[harness-adapters]] §5, retain `status_source=inferred` and never establish composer readiness.

## (c) Seat identity for hook processes

At launch the mux sets in the harness's environment:

```
BBOXY_SEAT_ID=<ULID>       # durable seat id
BBOXY_SOCK=<path>          # fallback socket
BBOXY_ROLE=oracle|executor|creativity
TERM_PROGRAM=bboxy450-term  TERM_PROGRAM_VERSION=<semver>
```

Hooks inherit the harness environment (research §9), so `bboxy-hook` always sees `BBOXY_SEAT_ID`. It puts it in the OSC anyway (`seat=`) so the mux can detect a mismatch (hook from seat A arriving on PTY B means a nested/forwarded tty) and route by the env value instead. Nothing in the chain parses `TERM_SESSION_ID`.

## (d) State transitions

Seat row (`shim` events unless marked `7501`):

| Event | Seat.state | waiting_kind | detail | turn_open |
|---|---|---|---|---|
| `SessionStart` | idle | none | "" | false |
| `UserPromptSubmit` | active | none | keep | true |
| `PreToolUse`/`PostToolUse` (no `agent_id`) | active | none | summary | true |
| `PreToolUse`/`PostToolUse` (with `agent_id`) | active | none | keep | true (+ subagent row active, detail=summary) |
| `PermissionRequest` | waiting | permission | "Allow " + summary / question / "Review proposed plan?" | true |
| `Notification permission_prompt` | waiting | permission | message | true |
| `Notification agent_needs_input` / `elicitation_*` | waiting | question | message | true |
| `Notification idle_prompt` | idle | none | keep | false |
| `Stop` | idle (active if any subagent active) | none | last_message | false |
| `StopFailure` | errored | auth for authentication_failed / oauth_org_not_allowed / cloud_credential_error, otherwise none | error | false |
| `SessionEnd` | idle, harness_session_id kept | none | reason | false |
| 7501 `state=working` | active | none | msg | true |
| 7501 `state=blocked kind=k` | waiting | k | msg | true |
| 7501 `state=done` | idle | none | msg | false |
| 7501 `state=error` | errored | none | msg | false |
| 7501 `state=idle`/`clear` | idle | none | keep | false |
| PTY quiet 2 s + prompt (inferred only) | idle | none | keep | — |

Subagent row:

| Event | Subagent |
|---|---|
| parent `PreToolUse Agent` | create `pending` with `description`, keyed by `tool_use_id` until an id is known |
| `SubagentStart agent_id` | create/bind `active`, `started_at=now` (bind to the oldest `pending` of same `agent_type`) |
| parent `PostToolUse Agent` `tool_response.agentId` | authoritative bind `tool_use_id → agent_id`, set `description` |
| 7501 `id=…:state=working:title=…` | upsert `active`, `description=title` |
| 7501 `id=…:state=blocked:kind=k` | waiting, k |
| child `PermissionRequest`/`Notification` with `agent_id` | waiting |
| child `PreToolUse`/`PostToolUse` with `agent_id` | active, detail=summary |
| `SubagentStop agent_id` | done, `ended_at=now`, detail=last_message |
| 7501 `state=clear:id=…` | done if not already, `ended_at=now` |
| parent `Stop` with `background_tasks[]` | any listed agent whose `status` ∉ {running} → done; any `active` subagent absent from the list → done (iTerm2 counts this way, research §3) |

## (e) Terminal restart

- The GUI reattaches to mux logic for snapshots/history without launching a harness. The supervisor owns PTY masters and rings; logic owns adapters, the hook socket and derived state, per [[mux-daemon]] “Ownership and failure boundaries”.
- Logic restart retains the same supervisor-owned children. Replacement logic restores a checkpoint and replays captured records; PTY hook output remains captured subject to the ring's existing backpressure. Off-PTY socket delivery can be unavailable; it has no hook spool.
- Supervisor/host restart is the new-process fallback: recover committed history, resume supported harness session IDs under the same durable seat ID and a new process generation, and revalidate every attachment/readiness proof. Closed seats follow the mux close lifecycle instead of automatic open-seat resume.
- `SessionStart source=resume` clears prior subagent rows; live adapter observations rebuild them (7501 task reports for Claude, hook events for Codex). Historical display provenance is not a fresh readiness indication. Recovery and off-PTY resynchronization limits follow [[mux-daemon]] and [[harness-adapters]].

## (f) What iTerm2 does that we skip, and why

| iTerm2 | We | Why |
|---|---|---|
| `cc-status` → `it2` CLI → websocket → Python-API builtin `set_session_status` (research §5-6) | OSC on the PTY, socket fallback | Four hops and a 10 MB CLI per hook event; the PTY already identifies the seat |
| Edits `~/.claude/settings.json` and symlinks `~/.config/iterm2/cc-status` (research §1) | per-launch `--settings` overlay, nothing in `$HOME` | A terminal must not mutate the user's harness config; two terminals would fight over the hook list |
| `TERM_SESSION_ID` parsing + tmux `--tmux/--tmux-pane` resolution (research §2) | `BBOXY_SEAT_ID` | One variable, injected by us, valid in tmux and ssh |
| Status priority sort + snooze (research §8) | stable order, state in place | [[ux-sidebar]] §2 |
| Background-task **count** only (`--background-tasks N`) (research §3) | full subagent list from 7501 + hooks | The count was iTerm2's pre-7501 design; 7501 made it redundant |
| Lists at most four task rows (research §7) | all, collapsing past 8 | Swarm scale |
| Progress ring `OSC 9;4;3` written by the hook (research §5) | none | Harness writes its own 9;4; the hook does not need to |
| `--expires-on progress-end` compare-and-set dance (research §5) | none | Ordering is total on one PTY, no race to defend against |
| Job-name watcher `GlobalJobMonitor`/`ClaudeWatcher` to detect Claude (research §8) | harness chosen at `Cmd+T` | We launch the harness; we know what it is |
| OSC 21337 tab status as a separate writer | parsed as a hint only | 7501 supersedes it |
| In-memory status lost on quit | seat table on disk in the mux | [[vision]] Persistence |
| MCP `status_source` reported/inferred (research §8) | kept, plus `shim` | Good idea; three grades instead of two |

## Open

- **Gap:** Codex per-launch hooks injection and trust hash (b.3).
- Pi/omp observation routing is resolved by [[harness-adapters]] R6 and §2b; their remaining runtime/event gaps stay in that spec.
- **Gap:** exact iTerm2 7501 reply bytes beyond the `?` echo (research §7). Claude Code's matcher accepts the echo; verify against Claude Code 2.1.304 in the first prototype by watching `--debug=terminal` for `probe: OSC 7501 reply`.
- **Gap:** whether the parent `Stop.background_tasks[]` lists in-process subagents with `type` = `local_agent` or similar; counted as running by iTerm2 (research §3) but the item schema in docs shows only `shell`. Capture one payload in the prototype.

**Wire evidence (task H, 2026-10-21, [[osc7501-capture]]):** Claude Code 2.1.304 proven: the native probe and every report are BEL-terminated; a same-payload ST-terminated echo was accepted; one Agent child produced `working` with `id` and `title`, then `clear:id`, then the main report `done`. Our parser must accept both BEL and ST, and emit the probe echo with BEL to match. Codex remains unproven on the wire: its startup trust chooser blocks a hookless fixture launch and no bypass is allowed (S6); needs the operator to trust the fixture directory once.
