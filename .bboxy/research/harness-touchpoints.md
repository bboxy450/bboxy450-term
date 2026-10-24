---
id: "1ST6RH8CYDHHBARXT2372ETDBR"
type: "research"
number: 81
title: "Harness touch points — inject and observe through an owned PTY"
created: "2026-10-21T19:25:45Z"
updated: "2026-10-24T00:10:38Z"
---
# Harness touch points — inject and observe through an owned PTY

Status: research, 2026-10-21. Written by a research subagent for Vesper Greywind <ORACLE>. Local binaries: Claude Code 2.1.304, Codex CLI 0.163.0, pi 1.0.0 (`@earendil-works/pi-coding-agent`), omp 18.4.12 (`@oh-my-pi/pi-coding-agent`), icq 0.20.2. Source paths below are absolute on this machine. Everything marked **(memory)** is from training knowledge and was not verified locally.

Correction to the brief: **omp is not OpenCode.** `omp` is "oh-my-pi", a fork of pi by can1357 (`/opt/homebrew/lib/node_modules/@oh-my-pi/pi-coding-agent/package.json` — `homepage: https://omp.sh`, `repository: github.com/can1357/oh-my-pi`, contributor Mario Zechner = pi's author). `opencode` 1.19.3 is a separate Mach-O binary at `/opt/homebrew/bin/opencode`; icq does not touch it. icq calls the harness `omp` (`src/harness.rs:107 — matches!(harness.as_str(), "pi" | "omp")`).

Terminology for the whole file: **inject** = get text or configuration into the harness; **observe** = learn active / idle / waiting-on-input / errored / title.

---

## 1. Claude Code

### 1.1 Inject at launch

| Touch point | Evidence | Mechanism |
|---|---|---|
| `--append-system-prompt <text>`, `--system-prompt <text>`, `--system-prompt-snapshot on\|off` | `claude --help` — listed | Role prompt per seat. Snapshot default `on`: prompt rendered on first request, reused verbatim on resume until compaction. Use `--append-system-prompt` to keep default prompt. |
| `--settings <file-or-json>` | `claude --help`; `icq src/launch.rs:200-229` writes a 0600 `settings.json` with `crossSessionInbound:"accept"` and passes `--settings=PATH` | Per-launch settings merged over user/project. Hooks, `crossSessionInbound`, `statusLine`, `model`, permissions all go here. |
| `--mcp-config <files...>`, `--strict-mcp-config` | `claude --help`; `icq src/launch.rs:207-229` writes `mcp.json` `{"mcpServers":{"icq":{"type":"stdio","command":BIN,"args":[...]}}}` | Per-launch MCP servers. The terminal's own MCP server goes here. |
| `--allowedTools`, `--disallowedTools`, `--tools`, `--permission-mode` | `claude --help`; `icq src/launch.rs:137,228` adds `mcp__icq__icq` | Pre-approve the terminal MCP tools so the Oracle seat is not prompted. |
| `--agent <name>`, `--agents <json>` | `claude --help` | Named agent definition with its own prompt — alternative role carrier. |
| `--plugin-dir <path>`, `--plugin-url` | `claude --help` | Session-only plugin: hooks + skills + commands in one directory. |
| `--add-dir` | `claude --help` | CLAUDE.md discovery dirs. |
| `--name/-n <name>` | `claude --help` — "shown in the prompt box, /resume picker, and terminal title" | Terminal can name the seat at launch; Claude echoes it in OSC title. |
| `--session-id <uuid>`, `--resume`, `--fork-session`, `--continue` | `claude --help` | Terminal chooses the session UUID at launch → durable seat id ↔ session id without parsing. |
| `--bare` | `claude --help`; `icq src/launch.rs:60-69` rejects it | Skips hooks, plugins, CLAUDE.md and **the messaging socket**. Never use for seats. |
| `--bg`, `claude attach <id>`, `claude logs`, `claude stop`, `claude agents` | `claude --help` | Claude's own background-session mux. Competes with / can be reused by our mux daemon. |
| `-p --input-format stream-json` | `claude --help` | Headless: JSONL user messages on stdin, `--output-format stream-json` events on stdout, `--include-hook-events`. Full inject+observe channel for non-TUI seats. |
| `--restricted`, `--setting-sources user,project,local` | `claude --help` | Lock a seat's config surface. |
| Skills dirs | binary strings: `.claude/skills/`, `.claude/agents/`, `.claude/commands/`, `.claude/hooks`, `.claude/plugins/` | `~/.claude/skills/` (user, here a symlink to `~/.agents/skills`), `.claude/skills/` (project). icq setup writes `skills/icq/SKILL.md` there (`src/setup.rs:131-134,192-224`). |

Env exported to children (observed in this session's `env`): `CLAUDECODE=1`, `CLAUDE_PID`, `CLAUDE_CODE_SESSION_ID`, `CLAUDE_CODE_MESSAGING_SOCKET=/tmp/cc-socks/<pid>.sock`, `CLAUDE_CODE_MESSAGING_TOKEN`, `CLAUDE_CODE_ENTRYPOINT=cli`, `CLAUDE_CODE_EXECPATH`, `CLAUDE_CODE_SESSION_ATTENDED=1`, `CLAUDE_EFFORT`, `AI_AGENT=claude-code_<ver>_agent`. A hook or MCP server spawned by the seat inherits these; that is how icq attributes itself (`src/harness.rs:236-266`, `src/focus.rs:7-15`).

### 1.2 Hooks (observe + inject)

Event names present in the binary (`strings` of `/opt/homebrew/Caskroom/claude-code/2.1.304/claude`, quoted-string counts): `PreToolUse`, `PostToolUse`, `PostToolUseFailure`, `PermissionRequest`, `PermissionDenied`, `Stop`, `StopFailure`, `UserPromptSubmit`, `SessionStart`, `SessionEnd`, `SubagentStart`, `SubagentStop`, `Setup`, `Elicitation`, `ElicitationResult`, `WorktreeCreate`, `WorktreeRemove`, `TaskCompleted`, `TeammateIdle`, `Notification`, `FileChanged`, `InstructionsLoaded`, `CwdChanged`, `PreCompact`, `PostCompact`, `ConfigChange`.

Config shape (local `~/.claude/settings.json`): `hooks.<Event>[] = {matcher?: string, hooks: [{type:"command", command, timeout?, statusMessage?}]}`. SessionStart matcher values: `startup|resume|clear|compact` (`icq src/setup.rs:14`). icq's own hook: `~/.claude/settings.json` → `SessionStart` → `'/Users/feral/.cargo/bin/icq' --home '/Users/feral/.icq' session-hook --target claude`.

Hook stdin JSON fields present in binary: `hook_event_name`, `session_id`, `transcript_path`, `cwd`, `permission_mode`, `tool_name`, `tool_input`, `tool_response`, `tool_use_id`, `stop_hook_active`, `notification_type`. `notification_type` values in binary: `permission_prompt`, `idle_prompt`, `auth_success`, `elicitation_dialog`. icq consumes exactly this: `src/main.rs:1284-1313` reads stdin ≤64 KiB, requires `hook_event_name == "SessionStart"`, takes `session_id`, then runs `icq start` with `CLAUDE_CODE_SESSION_ID` set.

Hook stdout JSON fields present in binary: `continue`, `stopReason`, `suppressOutput`, `systemMessage`, `hookSpecificOutput`, `additionalContext`, `permissionDecision`, `updatedInput`. **(memory)** `SessionStart` / `UserPromptSubmit` hooks that print plain text or `hookSpecificOutput.additionalContext` inject into the model context — a second role-prompt channel that also works on `--resume`. `PermissionRequest` hook can return `permissionDecision` and auto-approve; `PreToolUse` can rewrite `updatedInput`.

State mapping from hooks (hooks win when present, per vision):

| Hook event | Seat state |
|---|---|
| `UserPromptSubmit`, `PreToolUse`, `PostToolUse`, `SubagentStart` | active |
| `Stop`, `SubagentStop` (top level) | idle |
| `PermissionRequest`, `Elicitation`, `Notification{permission_prompt\|elicitation_dialog\|idle_prompt}` | waiting-on-input |
| `StopFailure`, `PostToolUseFailure` | errored (transient) |
| `SessionEnd` | exited |

Precedent on this machine: iTerm2 3.7.6 ships `/Applications/iTerm.app/Contents/Resources/utilities/cc-status` (Mach-O; strings: `get-status`, `set-status`, `notification_type`, `permission_prompt`, `PermissionRequest`, `waiting`, `--background-tasks`, shells out to `it2`). The operator's `~/.claude/settings.json` wires it to Notification, PermissionRequest, Pre/PostToolUse, SessionStart/End, Stop, StopFailure, SubagentStop, UserPromptSubmit — the same mapping as above, implemented by iTerm2.

### 1.3 Session registry (observe, no hooks needed)

`~/.claude/sessions/<pid>.json` — one JSON per live interactive process (`cat ~/.claude/sessions/*.json`):

```
{"pid":61194,"sessionId":"3544d975-…","cwd":"/Users/feral/mydev/bboxy450-term","startedAt":…,"procStart":"Wed Oct 21 18:05:48 2026","version":"2.1.304","peerProtocol":1,"peerFeatures":["notify_idle","reply_across_default_dirs","artifact_yield"],"kind":"interactive","entrypoint":"cli","pidDomain":"darwin","messagingSocketPath":"/tmp/cc-socks/61194.sock","name":"bboxy450-term-cb","nameSource":"derived","nameSince":…,"status":"busy","updatedAt":…,"statusUpdatedAt":…}
```

- `status` values observed live: `idle`, `busy`, `shell`. Literals in binary: `"idle"`, `"busy"`, `"shell"`, `"waiting"`, `"stopped"`, `"exited"`; record also carries optional `waitingFor` string. Derivation found in binary: `function Sxe(h){let R=VMo(h);if(R!==void 0)return{status:"waiting",waitingFor:R,working:!1};return{status:h.isLoading||h.deleg…` — i.e. a pending dialog/prompt → `waiting` + `waitingFor`, else loading/delegating → `busy`, else `idle`. So the sessions file already distinguishes **waiting-on-input** from idle. The terminal can `kqueue`/poll this file per seat pid — a hook-free state feed. **Gap:** what sets `shell` (probably `!` shell mode or a foreground Bash); not extracted.
- `name` is the display name (`--name` or derived); `nameSource: derived|…`.
- `claude agents --json` prints the same set (pid, cwd, kind, sessionId, name, status) for all live sessions — verified locally, returned 6 sessions with `idle`/`busy`.
- icq reads this file to attach from a hook: `src/native.rs:186-223` — path `$CLAUDE_CONFIG_DIR|~/.claude/sessions/<pid>.json`, checks `pid` + `procStart` match `ps -o lstart=`, takes `sessionId` and `messagingSocketPath`. Ancestor pid is found by walking `ps -o ppid=` (`src/native.rs:84-116`), nearest executable named `claude` (`src/native.rs:117-132`).

### 1.4 Cross-session socket (inject = wake)

- Path: `/tmp/cc-socks/<pid>.sock`, dir `0700`, socket `0600` (`ls -la /tmp/cc-socks`). Binary regex admits `/tmp/cc-socks(-N)?`, `/private/tmp/cc-socks…`, `$CLAUDE_CODE_TMPDIR`.
- Protocol: newline-delimited JSON. Binary contains the canonical example: `{ echo '{"type":"auth","token":"'"$CLAUDE_CODE_MESSAGING_TOKEN"'"}'; echo '{"type":"user","message":{"role":"user","content":"hello"}}'; } | socat - UNIX-CONNECT:…`. Message types in binary: `auth`, `user`, `peer_message_status`, `peer_idle_notice` (`[uds-messaging] peer_idle_notice not admitted: subscription for orig_msg_id=`). Fields: `uuid`, `msg_id`, `orig_msg_id`, `priority` (`"next"` ×53, `"later"` ×15 in binary; `'now'` interrupts the running turn per help text in binary).
- Auth is configurable: the listener logs `Inject messages (auth line ${authRequired?"REQUIRED":"optional"} here): …` — the `auth` frame is optional by default and required under some policy. **Gap:** which setting flips `authRequired`. Outbound frame types in binary: `type:"user"` ×44, `type:"error"` ×116 (error replies on the socket).
- Busy sub-kinds exist in the UI state machine: `Uio({working,spinnerMode,isCompacting,delegatedWorkRunning,waitingFor})` → `work:"compacting"`, `"subagent_running"`, `"turn_r…"` — richer than the three-valued `status` written to the sessions file.
- icq's write: `src/harness.rs:429-464` — `{"type":"user","uuid":ID,"msg_id":ID,"priority":"next","message":{"role":"user","content":TEXT}}` + `\n`, no `auth` frame (same-uid peer attribution; on macOS sleeps 150 ms before `shutdown()` so `Bun.ant.getPeerPid` can inspect the sender), result `Written{bytes}` with `receiver_acceptance: unverified` (`src/harness.rs:67-86`). Directory must be owned-private or root-sticky (`src/harness.rs:137-154`).
- Receiver policy: `crossSessionInbound: accept|hold|refuse` in settings (`icq src/setup.rs:276-286`; local user setting is `accept`). `--bare` has no socket (`claude --help`). Headless `-p` sessions need `--settings '{"crossSessionInbound":"accept"}'` (`src/setup.rs:164`).
- `peerFeatures: notify_idle` → a sender can subscribe to a `peer_idle_notice` after its message: an observe channel for "did the seat finish the turn I sent". **Gap:** exact subscribe frame — not extracted.
- `icq attach --harness claude --socket … --session-id …` registers an explicit endpoint (`icq attach --help`). `icq sessions` reports `native_ready` per registered seat (verified: 3 Claude seats, all `native_ready:true`).

### 1.5 Statusline and title (observe)

- `statusLine` setting: `{type:"command", command, padding?, refreshInterval?}` (binary schema `statusLine:()=>u({type:T("command"),command:o(),padding:E().optional(),refreshInterval:E().min(1)…`). **(memory)** stdin JSON includes `session_id`, `model`, `workspace`, `cost`, `context_window`, `transcript_path`, `output_style` — field names `cost`, `context_window`, `current_usage`, `output_style`, `transcript_path` are present in the binary. A statusline command is a per-render heartbeat the terminal can own.
- Title: raw `ESC ] 0 ;` occurs once in the binary; `useTerminalTitle`, `terminalTitle`, `terminalTitleFromRename` symbols; `--name` help says the name is "shown in … terminal title". So Claude emits **OSC 0** with the session name. No `ESC ] 2 ;`, no OSC 7, no OSC 133, no OSC 9;4, no OSC 1337 found. **Gap:** whether the title encodes state (busy/idle); not extracted.
- Other sequences present: `ESC [ ?2026h` (synchronized output), `ESC [ ?1049h` (alt screen), `ESC [ 6n` (DSR cursor query), `ESC [ ?25l`, `ESC [ c` (DA1), `ESC ] 52 ;` (clipboard), `ESC ] 8 ;` (hyperlinks). Bracketed paste `ESC [ ?2004h` not found as a literal (**Gap:** probably composed at runtime).

---

## 2. Codex CLI

### 2.1 Inject at launch

| Touch point | Evidence | Mechanism |
|---|---|---|
| `-c key=value` (dotted TOML override) | `codex --help`; `icq src/launch.rs:78-97` injects `mcp_servers.icq.command/args/env_vars/required/startup_timeout_sec/tool_timeout_sec/tools.icq.approval_mode` | Any `config.toml` key per launch, including MCP servers, `notify`, `features.*`, `tui.*`, `hooks` trust bypass. |
| `--enable/--disable <FEATURE>` | `codex --help`; `codex features list` shows `hooks stable true`, `daemon_auto_start stable true` | |
| `-p/--profile <name>` | `codex --help` | Layer `$CODEX_HOME/<name>.config.toml` — a role profile per seat. |
| `--dangerously-bypass-hook-trust` | `codex --help` | Run hooks without the persisted `hooks.state.*.trusted_hash` in `config.toml`. Needed when the terminal writes hooks.json for a fresh seat. |
| `-s/--sandbox`, `-a/--ask-for-approval`, `--approve-for-me`, `-C/--cd`, `--worktree`, `--add-dir` | `codex --help` | Executor sandbox + approval posture. |
| `codex resume <id\|name> [PROMPT]`, `codex fork` | `codex resume --help` | Resume by UUID or session name. |
| `--remote unix://PATH` | `codex --help` | TUI attaches to an existing app-server instead of embedding one — a mux hook. |
| `--no-alt-screen`, `--no-daemon` | `codex --help` | Inline mode preserves scrollback; `--no-daemon` isolates from the shared daemon. |
| System prompt | `codex --help` — **no `--system-prompt` flag** | Role prompt must go through `AGENTS.md` (`~/.codex/AGENTS.md → ../.agents/AGENTS.md` locally), a profile's `model_instructions_file` **(memory)**, or the first user message / `thread/start` params over app-server. **Gap:** verify `model_instructions_file` key name on 0.163. |
| Skills | `~/.codex/skills/icq -> ../../.agents/skills/icq` (icq `extensions/README.md`, `src/setup.rs:131-134`) | `~/.codex/skills/<name>/SKILL.md`. |
| MCP | `~/.codex/config.toml [mcp_servers.*]` (local: paperhands, node_repl, rea) | `codex mcp` subcommand or `-c mcp_servers.X.command=…`. |

Env exported to children: `CODEX_THREAD_ID` (binary ×7; icq relies on it `src/focus.rs:8`, `src/harness.rs:237`), `CODEX_HOME`, `CODEX_SANDBOX`, `CODEX_SANDBOX_NETWORK_DISABLED`, `CODEX_SESSION_ID` (×1), `CODEX_CI`.

### 2.2 Hooks (observe + inject)

- File: `~/.codex/hooks.json` (read locally). Same shape as Claude: `{"hooks":{"SessionStart":[{"matcher":"startup|resume|clear|compact","hooks":[{"type":"command","command":"…icq … session-hook --target codex","timeout":30,"statusMessage":"ICQ session guidance"}]}]}}`.
- Event keys from `config.toml [hooks.state]` trust entries (local): `session_start`, `session_end`, `user_prompt_submit`, `pre_tool_use`, `post_tool_use`, `pre_compact`, `post_compact`, `subagent_start`, `subagent_stop`, `stop`. PascalCase names in binary: `SessionStart`, `SessionEnd`, `UserPromptSubmit`, `PreToolUse`, `PostToolUse`, `PreCompact`, `PostCompact`, `SubagentStart`, `SubagentStop`, `Stop`. No Notification/PermissionRequest hook events found (binary `permission_request` ×8 only; **Gap:** whether it is a hook event).
- Trust: each `hooks.json:<event>:<i>:<j>` needs `trusted_hash = "sha256:…"` in `config.toml` (local file) — set via `/hooks` in the TUI, or bypass with `--dangerously-bypass-hook-trust` (`icq extensions/README.md`: "Codex requires `/hooks` trust").
- Handler types in binary: `HookHandlerConfig` internally tagged enum with `command`, `mcp_tool`, `prompt`, `agent`; fields `timeout`, `async`, `statusMessage`, `additionalContextLimit`, `eventName`, `matcher`, `timeoutSec`, `sourcePath`, `pluginId`.
- Hook stdin fields in binary: `hook_event_name`, `session_id`, `transcript_path`, `cwd`, `tool_name`, `tool_input`, `tool_response`, `stop_hook_active`. icq treats it identically to Claude (`src/main.rs:1290-1313`, target `codex` → `CODEX_THREAD_ID`).
- Hook output: `additionalContext` present in binary → SessionStart/UserPromptSubmit can inject context **(memory on semantics)**.

### 2.3 `notify` (observe, one-shot per turn)

- `config.toml`: `notify = ["<program>", "<extra args>…"]` (local: SkyComputerUseClient, `"turn-ended"`). Codex appends one JSON argument.
- Payload keys from binary (`hooks/src/legacy_notify.rs`): `type` ∈ `agent-turn-complete`, `approval-requested`, `plan-mode-prompt`, `async-question`; `thread-id`, `turn-id`, `cwd`, `client`, `input-messages`, `last-assistant-message`.
- Mapping: `agent-turn-complete` → idle; `approval-requested`, `async-question`, `plan-mode-prompt` → waiting-on-input. No "turn started" notify; pair with `user_prompt_submit` hook or app-server `turn/started`.

### 2.4 App-server (inject = wake, observe = everything)

- Shared daemon (`daemon_auto_start` stable, `codex agents` "Browse all agent sessions on the shared local app-server daemon"). Control locator: `~/.codex/app-server-control/app-server-control.sock -> /private/tmp/codex-daemon-501/<hash>` (symlink; `ls -la`). Daemon pid 2220 holds 8 fds on that socket (`lsof -U -c codex`). `~/.codex/app-server-daemon/settings.json` has `remoteControlEnabled:false`. Also `~/.codex/ipc/ipc.sock` (**Gap:** purpose).
- Transport: Unix socket carrying WebSocket, JSON-RPC-ish frames (`icq src/harness.rs:524-562`): `initialize{clientInfo,capabilities:{experimentalApi:true}}` → `initialized` → `thread/read{threadId,includeTurns:false}`; status `thread.status.type ∈ idle|active` required (`src/harness.rs:553-560`); other enum members in binary: `notLoaded`, `systemError`, `activeFlags`.
- Wake: `thread/queue/add{threadId,clientUserMessageId,input:[{type:"text",text}]}` → `queuedSubmission{id,clientUserMessageId,input}` (`src/harness.rs:591-626`). CLI equivalent: `codex queue --thread <UUID|name> --message <TEXT>` (`codex queue --help`). Other methods in binary: `turn/start`, `turn/steer`, `turn/interrupt`, `thread/inject_items`, `thread/queue/list|update|delete|reorder|start`, `thread/name/set`, `thread/start|resume|fork|list|loaded/list`, `process/spawn|writeStdin|resizePty`, `command/exec`.
- Observe notifications in binary: `thread/started`, `thread/status/changed`, `thread/closed`, `turn/started`, `turn/completed`, `hook/started`, `hook/completed`, `item/started`, `item/completed`, `item/agentMessage/delta`, `thread/queue/changed`, `thread/name/updated`, `thread/tokenUsage/updated`, `serverRequest/resolved`, `account/rateLimits/updated`, `warning`. `thread/status/changed` + `turn/started|completed` is the authoritative active/idle feed; approval requests arrive as server requests (`serverRequest/resolved` closes them).
- icq attaches a TUI thread by discovering the daemon socket among the ancestor `codex` process's open Unix sockets (`src/native.rs:133-185`, `lsof -p PID -U -Fn` on macOS, `/proc/PID/fd` on Linux), verifying each with `thread/read` (`src/session.rs:79-112`). Explicit override: `ICQ_CODEX_SOCKET` (`src/harness.rs:219-234`).
- `codex app-server generate-json-schema --out DIR` dumps the full protocol schema (`--help` verified) — run it when implementing.

### 2.5 Title and TUI sequences

- `[tui].terminal_title = [items…]` in `config.toml` (binary: `known terminal title item identifiers`, `Remove or replace the unknown entries in [tui].terminal_title`); emitted via `codex_tui::terminal_title::SetWindowTitle` as **OSC 2** (`ESC ] 2 ;` ×1 in binary; `]2;exceeded maximum length`). `[tui].status_line` items in local config include `run-state`, `approval-mode`, `context-used`, `task-progress` — **Gap:** whether `run-state` is a valid `terminal_title` item (would put state in the title); check `codex` config docs.
- Other sequences: `ESC ] 9 ;` ×1 (OSC 9 desktop notification), `ESC ] 0 ;` ×1, `ESC [ ?2004h` (bracketed paste), `ESC [ ?2026h`, `ESC [ ?1049h` (alt screen default; `--no-alt-screen`), `ESC [ ?1004h` (focus events), `ESC [ 6n`, `ESC [ >4;2m` (modifyOtherKeys), `ESC ] 8 ;`, `ESC ] 52 ;`.

---

## 3. pi (`@earendil-works/pi-coding-agent` 1.0.0)

Node script, `/opt/homebrew/bin/pi -> …/dist/bundle/cli.js`. Docs shipped at `/opt/homebrew/lib/node_modules/@earendil-works/pi-coding-agent/docs/`.

### 3.1 Inject

| Touch point | Evidence | Mechanism |
|---|---|---|
| `--system-prompt <text>`, `--append-system-prompt <text\|file>` (repeatable) | `pi --help` | Role prompt. |
| `--extension/-e <path\|builtin:name>`, `--no-extensions`, `pi install <source>` | `pi --help` | Extensions are the hook system. API: `pi.on(event)`, `pi.registerTool`, `pi.registerCommand`, `pi.sendUserMessage(text,{deliverAs:"steer"\|"followUp"\|"nextTurn"})`, `pi.sendMessage` (`docs/extensions.md:76-79`; `dist/core/extensions/types.d.ts:332,1215-1223`). User dir `~/.pi/agent/extensions/` (icq installed `~/.pi/agent/extensions/icq/{index.ts,host.mjs,bridge.mjs}`, manifest `~/.pi/agent/.icq-setup.json`). |
| `--skill <path>`, `--no-skills` | `pi --help` | `~/.pi/agent/skills/`. |
| MCP: `pi mcp add NAME -- cmd`, `~/.pi/agent/mcp.json`, `.pi/mcp.json` | `docs/mcp.md`; local `~/.pi/agent/mcp.json` exists | Same `mcpServers` format as Claude. |
| `--mode rpc` | `docs/rpc.md`, `docs/rpc-commands.md` | JSONL on stdin/stdout: `prompt`, `steer`, `follow_up`, `abort`, `get_state`, `set_session_name`, `switch_session`, `fork`, `bash`, … ; `extension_ui_request` records carry `setTitle`, `setStatus`, `notify` (`docs/rpc-extension-ui.md:8,146-155`). Full headless inject+observe channel. |
| `--session-id <id>`, `--session <path>`, `--name/-n`, `--session-dir`, `--no-session` | `pi --help` | Terminal picks the session id; `-n` names it. |
| `--tools/--exclude-tools/--no-tools` | `pi --help` | |
| Context files | `--no-context-files` disables `AGENTS.md`/`CLAUDE.md` discovery (`pi --help`) | AGENTS.md is read by default. |

Env exported to children: `PI_SESSION_ID`, `PI_SESSION_FILE` (`docs/environment-variables.md:26-27`; icq `src/focus.rs:12-13`), `PI_CODING_AGENT_DIR` overrides `~/.pi/agent` (`:81`).

### 3.2 Observe

- Extension events (binary strings in `dist/`): `session_start`, `session_shutdown`, `session_before_switch`, `session_before_fork`, `session_before_tree`, `session_tree`, `session_before_compact`, `session_compact`, `agent_start`, `agent_end`, `before_agent_start`, `turn_start`, `turn_end`, `tool_call`, `tool_result`, `user_bash`, `input`, `context`, `model_select`, `resources_discover`, `before_provider_request`. Mapping: `agent_start`/`turn_start` → active; `agent_end` → idle (but `docs/json.md:48`: wait for `agent_settled` before trusting idle); `tool_call` for an `ask`-style tool → waiting-on-input.
- JSON event stream (`--mode json` / rpc): `agent_start`, `turn_start`, `turn_end`, `agent_end{willRetry}`, `agent_settled`, `queue_update{steering,followUp}`, tool execution events, compaction, retry (`docs/json.md`).
- Title: pi-tui `setTitle` writes `ESC ] 0 ; title BEL` (`node_modules/@earendil-works/pi-tui/dist/terminal.js:421`); interactive mode sets `"${APP_TITLE} - ${sessionName} - ${cwdBasename}"` (`dist/modes/interactive/interactive-mode.js`). No state in the title. Progress: `ESC ] 9 ; 4 ; 3 BEL` active / `… 4 ; 0 BEL` clear (`terminal.js:8-9,425-431`), gated by setting `terminal.showTerminalProgress` (default false, `docs/settings.md:106`). Turning that on gives a hook-free busy/idle signal on the PTY.
- Hooks file: none. There is no `hooks.json`; extensions are the only lifecycle hook surface.

### 3.3 icq attachment (bridge)

- `extensions/pi.ts` → `registerHost(pi,'pi')` + `registerTool('icq')` (`icq extensions/pi.ts:5-12`).
- `extensions/host.mjs:38-63`: on `session_start` (and omp session switch/branch/tree) creates a bridge and runs `icq start` with `ICQ_HARNESS_DESCRIPTOR` env (`bridge.mjs:107-109`), closes on `session_before_switch|fork|tree|shutdown` (`host.mjs:73-76`).
- `extensions/bridge.mjs:12-81`: owner-private Unix socket at `$TMPDIR/icq-h-XXXX/sock` (dir 0700, sock 0600), newline JSON: `{"version":1,"op":"identity"}` → `{identity:{kind,instance_id,session_id,generation}}`; `{"op":"dispatch","identity","message_id","text"}` → calls `pi.sendUserMessage(text,{deliverAs:'followUp',expandPromptTemplates:false})` (`bridge.mjs:55`) → `{state:"dispatched"}`. Dedup by `message_id` + sha256 (`bridge.mjs:48-57`).
- Rust side: `src/harness.rs:40-46` `Attachment::Bridge`, `:334-354` framing, `:373-421` identity check then dispatch, receipt `Dispatched` ("native callback invoked, not queue acceptance", `extensions/README.md`). Descriptor validation `src/session.rs:178-206`, `src/focus.rs:40-48`.

---

## 4. omp (oh-my-pi, `@oh-my-pi/pi-coding-agent` 18.4.12)

Bun script, `/opt/homebrew/bin/omp -> …/dist/cli.js`; TypeScript source shipped under `…/@oh-my-pi/pi-coding-agent/src/`. Fork of pi: same extension API shape (`extensions/omp.ts` differs from `pi.ts` only in import path and zod schema).

### 4.1 Inject (delta from pi)

- `--system-prompt`, `--system-prompt-template <handlebars>`, `--append-system-prompt`, `--hook <file>` (alias path for an extension; `src/cli/flag-tables.ts:218-221` pushes into `result.hooks`), `-e/--extension`, `--trusted-extension`, `--plugin-dir`, `--config <overlay.yml>` (repeatable), `--profile <name>` (isolated auth/sessions/settings under `~/.omp/profiles/<name>/agent`; `icq src/setup.rs:64-94`), `--mode text|json|rpc|rpc-ui|acp` (`src/cli/flag-tables.ts:122-127`), `--no-ui`, `--auto-approve`, `--approval-mode always-ask|write|yolo`, `--max-time`, `--from-claude`/`--from-codex` (import sessions — relevant to the "harness switching" future item), `--cwd`, `--add-dir`, `--no-title` (sets `PI_NO_TITLE=1`, `src/main.ts:1872`).
- **ACP mode** (`src/modes/acp/{acp-agent,acp-client-bridge,acp-event-mapper,acp-mode}.ts`): Agent Client Protocol — a standard JSON-RPC agent↔editor protocol over stdio. The terminal could drive an omp seat as an ACP client. **Gap:** pi 1.0.0 has no acp mode dir (`dist/modes/` = interactive, json-event, print-mode, rpc).
- Config root `~/.omp/agent/` (local: `config.yml`, `mcp.json`, `extensions/`, `sessions/`, `terminal-sessions/`, `agent.db`); strings also reference `.omp/config.yml`, `.omp/mcp.json`, `.omp/agents`, `.omp/tools/`, `.omp/plugins/installed_plugins.json`. Env: `OMP_PROFILE`, `PI_CODING_AGENT_DIR`, `PI_CONFIG_DIR` (`icq src/setup.rs:65-70`), `PI_SESSION_FILE`, `PI_NO_TITLE`, `PI_NO_PTY`, `PI_TOOL_BRIDGE_URL/TOKEN/SESSION`.
- icq extension for omp is **not installed** on this machine (`~/.omp/agent/extensions/` holds only `freebuff-local.ts`); `icq setup --target omp` would write `extensions/icq/{index.ts,host.mjs,bridge.mjs}` (`src/setup.rs:135-148`).

### 4.2 Observe

- Extension events add `session_switch`, `session_branch`, `session_before_branch` (host.mjs registers them for omp only, `extensions/host.mjs:66-74`).
- **Title carries state.** `src/utils/title-generator.ts:724` `TerminalTitleState = "idle" | "working" | "attention"`; `:800-835` composes `π > label` (idle, user's turn), `π ⠋ label` (working, spinner frame every 80 ms, `:733-736,748`), `π ! label` (attention = agent blocked on ask/approval, `:751`), `π: label` when disabled (`tui.titleState`). Emitted as `ESC ] 0 ; … BEL` (`:635,640`). Drivers: `agent_start` → working (`src/modes/controllers/event-controller.ts:998`), `tool_execution_start` of `ask` or approval-prompting tool → attention (`:1744-1746`), approval resolved → working (`:1962-1968`), `agent_end` → idle (`:2136,2148,2178`), focus switch → idle (`session-focus-controller.ts:220`). Also `ESC ] 1337 ; SetUserVar=<TERN_SESSION_FILE_VAR>=<base64> BEL` reporting the session file path (`title-generator.ts:694`). An AI-generated session title (`generateSessionTitle`, smol model) replaces the cwd label when no `-n` name is set (`interactive-mode.ts:2346`).
- Parsing the omp title gives idle/active/waiting-on-input **with no hooks** — the only harness here that does this out of the box.

---

## 5. icq attachment summary (how the wake works per harness)

| Harness | Detect self | Endpoint | Wake frame | Receipt |
|---|---|---|---|---|
| Claude | `CLAUDE_CODE_MESSAGING_SOCKET`+`CLAUDE_CODE_SESSION_ID` env (`src/harness.rs:238-252`) or ancestor `claude` pid → `~/.claude/sessions/<pid>.json` (`src/native.rs:186-223`) | `/tmp/cc-socks/<pid>.sock` | `{"type":"user","uuid","msg_id","priority":"next","message":{role,content}}\n` (`src/harness.rs:436-438`) | `Written`, acceptance unverified (`:455-458`) |
| Codex | `CODEX_THREAD_ID` env + `~/.codex/app-server-control/app-server-control.sock` (`src/harness.rs:219-234`) or ancestor `codex` pid → `lsof -U` sockets verified by `thread/read` (`src/native.rs:133-185`, `src/session.rs:79-112`) | daemon Unix WebSocket | `initialize`→`initialized`→`thread/read`→`thread/queue/add` (`src/harness.rs:540-549,604`) | `Queued{queue_id}` (`:611`) |
| pi / omp | `ICQ_HARNESS_DESCRIPTOR` env set by the extension's child spawn (`extensions/bridge.mjs:109`, `src/session.rs:178-206`) | `$TMPDIR/icq-h-*/sock` owned by the extension (`bridge.mjs:17-19`) | `{"version":1,"op":"identity"}` then `{"op":"dispatch",identity,message_id,text}` (`src/harness.rs:404-409`) → `pi.sendUserMessage(text,{deliverAs:'followUp'})` (`bridge.mjs:55`) | `Dispatched` (`:410`) |

Registration trigger: Claude/Codex via `SessionStart` hook → `icq session-hook --target X` (`src/setup.rs:324`, `src/main.rs:1284-1313`); pi/omp via extension `session_start` (`extensions/host.mjs:65`). icq never types into a PTY; the PTY fallback in [[vision]] is ours to build.

---

## 6. PTY-level detection (harness without hooks)

All **(memory)** unless marked. Each signal: what it proves / what it does not.

| Signal | How | Proves | Does not prove |
|---|---|---|---|
| termios `ICANON`/`ECHO` on the slave | `tcgetattr` on the pty slave fd the terminal owns (or `stty -f /dev/ttysNNN -a`) | Line-mode (`icanon echo`) = a plain shell or a `read`-style prompt is waiting for a line. Verified locally: a **busy** Claude seat shows `-icanon -echo` (`stty -f /dev/ttys010 -a`). | Nothing for TUIs: Claude, Codex, pi, omp stay raw for their whole life, busy or idle. Only useful for the zsh seat and for harnesses that drop to cooked mode for a sub-prompt. |
| Output quiescence | No bytes from the slave for N ms after a burst | Candidate idle. Combined with a visible cursor (`ESC [ ?25h`) and no pending `ESC [ ?2026h` frame, strong hint the TUI is at rest. | Idle ≠ waiting-on-input; a spinner animation (omp title spinner at 80 ms, Claude/Codex spinners) breaks quiescence while busy, and a long tool call with no output looks idle. |
| Mode 2026 synchronized output | Track `ESC [ ?2026h` … `ESC [ ?2026l` pairs (Claude ×1, Codex ×2 in binaries) | Frame boundaries; a frame rate >0 means the TUI is redrawing (busy/spinner). | Idle TUIs may still redraw on a clock tick. |
| Alt screen `ESC [ ?1049h/l` | Mode tracking in the VTE | Harness is in TUI mode (Codex default, Claude present). `?1049l` = harness exited or dropped to shell (`!` / `$EDITOR`). | Nothing about turn state. |
| Bracketed paste `ESC [ ?2004h/l` | Mode tracking | Harness has an input editor armed (Codex ×2). Dropped during a sub-process. | Not emitted as a literal by Claude; unreliable as a state bit. |
| Focus reporting `ESC [ ?1004h` | Mode tracking | Codex wants focus events — the terminal must synthesize `ESC [ I` / `ESC [ O` on seat focus change. | — |
| DSR `ESC [ 6n` / DA1 `ESC [ c` | Request tracking | Harness is initializing or re-measuring (both binaries contain them); the terminal must answer or the harness stalls. | — |
| Cursor shape/visibility `?25l/h`, DECSCUSR | Mode tracking | Cursor hidden during streaming output; shown at an input box. Cheap "accepting keystrokes" heuristic for TUIs. | Dialogs (permission prompts) may also show a cursor; cannot separate idle from waiting. |
| OSC 0/2 title | Parse in VTE | Claude: session name (OSC 0). Codex: configured items (OSC 2). pi: `pi - name - cwd`. **omp: state** (`>` idle, spinner working, `!` attention) — verified in source. | Title is only as good as the harness's own state machine. |
| OSC 9;4 progress | Parse | pi (opt-in setting) `9;4;3` = busy, `9;4;0` = done. Codex has OSC 9 (desktop notify); omp inherits pi-tui progress (**Gap:** default). | Not emitted by Claude or Codex. |
| OSC 7 cwd, OSC 133 prompt marks, OSC 633 | Parse | None of the four harnesses emit them (byte grep). Only the zsh seat, if shell integration is installed, gives `133;A/B/C/D` prompt/command boundaries = exact idle/active for shells. | — |
| OSC 1337 SetUserVar | Parse | omp reports its session file path. | — |
| Keystroke echo probe | Inject a zero-width or `ESC [ 6n` and watch the response latency | TUI event loop is alive (responds) vs blocked. | Alive ≠ idle. |
| Child process tree | `ps -o stat=,ppid=` of the harness pid; count children | A running tool (`bash`, `rg`) = active; none + quiescent = idle. Works on every harness. | A pure model turn (no tool) spawns nothing and looks idle. |
| Process exit / SIGCHLD | pty master EOF | exited / errored (exit code). | — |

Conclusion for the fallback: for TUIs, no single PTY bit separates idle from waiting-on-input. Composite rule: quiescent ≥ 500 ms, cursor visible, no child processes, no 2026 frames → "at rest"; "at rest" after a known prompt-like redraw region or omp `!` title → waiting-on-input. Before injecting the PTY fallback, require "at rest" plus either an idle hook/notify within the last few seconds or an omp/Codex title that says idle; otherwise queue. Typing into a busy TUI is safe for Claude/Codex/pi/omp only because they buffer the editor, but Enter during a permission dialog answers the dialog — that is the hazard the gate exists for.

---

## 7. Matrix — harness × touch point

| Touch point | Claude Code | Codex CLI | pi | omp |
|---|---|---|---|---|
| System prompt flag | `--system-prompt`, `--append-system-prompt` | none; `AGENTS.md` / profile / first message | `--system-prompt`, `--append-system-prompt` | same + `--system-prompt-template` |
| Per-launch settings | `--settings json\|file` | `-c key=val`, `-p profile` | settings.json + flags | `--config overlay.yml`, `--profile` |
| Hooks file | `settings.json hooks.*` (26 events) | `~/.codex/hooks.json` (10 events, trust hash) | none (extensions) | none (extensions, `--hook` = extension file) |
| Context injection via hook | `SessionStart`/`UserPromptSubmit` stdout / `additionalContext` | `additionalContext` (**Gap:** confirm) | extension `before_agent_start` can replace/append prompt (`docs/extensions.md:103`) | same |
| MCP config | `--mcp-config`, `.mcp.json`, `claude mcp` | `-c mcp_servers.X…`, `config.toml`, `codex mcp` | `~/.pi/agent/mcp.json`, `pi mcp add` | `~/.omp/agent/mcp.json` |
| Skills dir | `~/.claude/skills`, `.claude/skills` | `~/.codex/skills` | `~/.pi/agent/skills`, `--skill` | `~/.omp/agent/…`, `--skills` glob |
| Session id control | `--session-id`, `--resume`, `--name` | `codex resume <id\|name>`, `thread/name/set` | `--session-id`, `--session`, `-n` | `-r`, `--session-dir` |
| Wake socket | `/tmp/cc-socks/<pid>.sock` JSONL (`auth`,`user`) | daemon Unix-WS `thread/queue/add`; `codex queue` | extension bridge socket → `sendUserMessage` | same |
| Headless stdin protocol | `-p --input-format stream-json` | `codex app-server --listen stdio://` | `--mode rpc` JSONL | `--mode rpc`, `--mode acp` |
| Turn start signal | `UserPromptSubmit` hook; sessions `status:busy` | `user_prompt_submit` hook; `turn/started` | `agent_start`/`turn_start` event | same + title spinner |
| Turn end signal | `Stop` hook; `status:idle`; `peer_idle_notice` | `stop` hook; `notify agent-turn-complete`; `turn/completed` | `agent_end` → `agent_settled` | same + title `>` |
| Waiting-on-input | `PermissionRequest`, `Elicitation`, `Notification{permission_prompt,idle_prompt}` | `notify approval-requested\|async-question\|plan-mode-prompt`; server request | `tool_call` on ask tool (extension) | title `!` (attention) |
| Errored | `StopFailure`, `PostToolUseFailure` | `thread.status systemError`; `warning` notif | `agent_end{willRetry}` / retry events | same |
| Exited | `SessionEnd`; pty EOF | `session_end`; `thread/closed`; pty EOF | `session_shutdown`; pty EOF | same |
| Hook-free state file | `~/.claude/sessions/<pid>.json` `status`, `claude agents --json` | `thread/read` over daemon | none | none |
| Title OSC | OSC 0, session name | OSC 2, `[tui].terminal_title` items | OSC 0 `pi - name - cwd` | OSC 0 `π <state> label` |
| Progress / notify OSC | none found | OSC 9 (desktop notify) | OSC 9;4 (opt-in) | inherits pi-tui (**Gap:** default) |
| Env to children | `CLAUDE_CODE_SESSION_ID`, `…_MESSAGING_SOCKET/TOKEN`, `CLAUDE_PID` | `CODEX_THREAD_ID`, `CODEX_HOME` | `PI_SESSION_ID`, `PI_SESSION_FILE` | same + `OMP_PROFILE` |
| icq adapter | `src/harness.rs:429-464`, `src/native.rs:186-223` | `src/harness.rs:524-626`, `src/native.rs:133-185` | `extensions/{pi.ts,host.mjs,bridge.mjs}`, `src/harness.rs:334-421` | `extensions/omp.ts` + same |

## Open gaps

- **Gap:** Claude: which code paths write `status:"waiting"` / `"shell"` to `~/.claude/sessions/<pid>.json`; `notify_idle` subscribe frame; whether OSC 0 title carries state; `auth` frame requirement when the sender is not same-uid-attributable.
- **Gap:** Codex: valid `[tui].terminal_title` item identifiers (is `run-state` among them); `permission_request`/`notification` as hook events; `model_instructions_file` key name; `~/.codex/ipc/ipc.sock` purpose; whether `thread/queue/add` into an `active` thread steers or waits (icq accepts both statuses, `src/harness.rs:553-560`).
- **Gap:** pi: whether `agent_settled` is exposed to extensions or only to the JSON stream; whether the `ask` tool name is stable for waiting detection.
- **Gap:** omp: OSC 9;4 progress default; ACP protocol version; whether `--mode rpc` matches pi's command set after the fork.
- **Gap:** PTY section is reasoning from terminal-protocol knowledge, verified only for termios on one busy Claude seat and for which literals exist in the binaries.

Links: [[vision]] [[agent-messaging]] [[terminal-core]]
