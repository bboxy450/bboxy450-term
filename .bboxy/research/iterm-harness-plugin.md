---
id: "57DBCYK2R8HW3DEKTS202Y4KK9"
type: "research"
number: 77
title: "iTerm2 3.7.6 harness plugin — how the Session Status panel is fed"
created: "2026-10-21T19:11:41Z"
updated: "2026-10-24T00:10:38Z"
---
# iTerm2 3.7.6 harness plugin — how the Session Status panel is fed

2026-10-21 · Oracle subagent for Vesper Greywind <ORACLE> · task `tasks/2026-10-21-subagent-detection-research.md`.

[[vision]] · [[ux-sidebar]] · [[harness-touchpoints]] · [[harness-adapters]] · spec: [[subagent-detection]]

Screenshot interpreted: `runs/ref-iterm-subagent-detection.png`. Every fact below is `file:line — mechanism` or `command — result`. Binary facts are `path @offset — mechanism` (byte offset from `python3` search) or `strings <path> line N`. **Gap:** marks what could not be verified on this machine.

## Answer in one paragraph

There are **two independent channels**, not one. (1) **Hooks → `cc-status` → `it2` CLI → iTerm2 Python-API websocket → built-in function `set_session_status`**: gives the seat's own row its status word (working/waiting/done/error/idle), colors, a one-line detail, a background-task *count* and a turn-open flag. (2) **OSC 7501 "Program Status Protocol", emitted by Claude Code and Codex themselves on their own PTY** after the terminal answers a capability probe: gives per-task reports with `id`, `state`, `title`, `msg`, and that is what draws the nested child rows (`claude-code`, `Research iroh/quinn…`, …). The child rows in the screenshot are channel (2). Channel (1) never carries subagent names; `cc-status` only ever sends a number.

## 1. What is installed on this machine

- `/Users/feral/.claude/settings.json` hooks — `python3 … json.load` result: ten events each run `/Users/feral/.config/iterm2/cc-status` with no args: `SessionStart` (second entry, after the ICQ hook), `SessionEnd`, `UserPromptSubmit`, `PreToolUse`, `PostToolUse`, `PermissionRequest`, `Notification`, `Stop`, `StopFailure`, `SubagentStop`. **`SubagentStart` is not bound.** No matcher on any cc-status entry.
- `ls -la /Users/feral/.config/iterm2/` — `cc-status -> /Applications/iTerm.app/Contents/Resources/utilities/cc-status` (symlink, created Oct 21 09:49), `AppSupport -> ~/Library/Application Support/iTerm2`, `sockets/secrets` (unix socket, unrelated to status).
- `file …/utilities/cc-status` — Mach-O universal (x86_64 + arm64) **Swift** binary, 321 520 bytes. It is not a script. Links Foundation, libswiftCore, Dispatch only (`otool -L`). No websocket code of its own: it shells out to `it2`.
- `ls /Applications/iTerm.app/Contents/Resources/utilities/` — `cc-status`, `it2` (10.8 MB Swift CLI, `it2 --version` → `1.0.0`), `it2api`, `imgcat`, `it2*` shell helpers.
- `strings iTerm2 @86662` — `utilities/cc-status`; `@86665` `Onboarding: no dot dir available for cc-status symlink`; `@86666` `Onboarding: couldn't create cc-status symlink:` — iTerm2's onboarding (`sources/ClaudeCode/ClaudeCodeOnboarding.swift`, `@86601`) creates the `~/.config/iterm2/cc-status` symlink and merges hook entries into settings: `@86649` `mergeCCStatusHooks(intoSettingsURL:ccStatusPath:environment:)`, `@86800` ` A cc-status hook is added to ~/.claude/settings.json`.
- `strings iTerm2 @120955-120962` — the event list iTerm2 installs and health-checks: `StopFailure`, `Notification`, `SessionEnd`, `PreToolUse`, `PostToolUse`, `SessionStart`, `SubagentStop`, plus `@86581-86582` `UserPromptSubmit`, `PermissionRequest` (`Stop` is ≤15 bytes and inlined, see §9). Matches settings.json exactly. `@87648` `ClaudeIntegrationHealthMonitor.swift`, `@87662` `…find a working cc-status hook for every Claude Code event in …` — repair prompt when a hook is missing.
- `strings iTerm2 @86655` `can't run cc-status but runs more than it; leaving it`, `@87441` `mention cc-status but also run other programs, so Uninstall didn…` — uninstall only removes hook commands that are exactly cc-status.
- `/Applications/iTerm.app/Contents/Resources/claude_usage.sh:1-30` — **unrelated to status.** It is the "AI Usage" toolbar provider: runs `env -u ANTHROPIC_API_KEY claude -p /usage` (line 95), parses `Current session: N% used … resets …` lines with awk into `{"bars":[{label,short,fraction,detail}],"error":null}`. Fixture env `ITERM2_AI_USAGE_FIXTURE`. Keep for the usage-limits roadmap item, not for this spec.
- `~/.codex/hooks.json` — only the ICQ `SessionStart` hook. No cc-status. `~/.codex/config.toml:148-178` — `[hooks.state."/Users/feral/.codex/hooks.json:<event>:0:0"] trusted_hash` entries exist for `pre_tool_use`, `post_tool_use`, `pre_compact`, `post_compact`, `session_start`, `session_end`, `user_prompt_submit`, `subagent_start`, `subagent_stop`, `stop` — Codex trust records for hooks that **were** in hooks.json at some point and are gone now. `git -C ~/.codex log -p hooks.json` shows a single commit with the ICQ-only file. **Gap:** who installed and removed those ten Codex hooks (iTerm2 supports `cc-status --agent codex`, §3); unproven.
- Codex seats in the screenshot show `Idle` without any hook: Codex 0.163.0 emits OSC 21337 and OSC 7501 natively (§6).

## 2. How a hook process finds its pane

`cc-status` has no `ITERM_SESSION_ID` string; it reads `TERM_SESSION_ID`.

- `cc-status.trace` (otool -tV arm64 + swift-demangle) `cc_status.resolveAddress([String:String]) -> Address?` — inline strings in order: `'TERM_SES'`+`'SION_ID'` → dictionary find → `firstIndex(of:)` + `subscript(PartialRangeFrom)` (strip through the `:`), then `'--sessio'` (`--session <uuid>`). Fallback branch: `'TMUX'`, `'TMUX_PAN'`, `tmuxAddressLooksWellFormed(tmux:pane:)`, `'--tmux'`, `'--tmux-p'`, then `'IT2_SOCK'` (`IT2_SOCKET` override) and `localIt2SocketExists(env)`.
- `env` in this very hook-equivalent process — `TERM_SESSION_ID=w0t7p0:8F4147F0-3E33-4B6F-A64B-F05E682F127B`, `ITERM_SESSION_ID` identical. The part after `:` is the session GUID `it2` uses; `it2 session list --json` lists `"id":"8F4147F0-3E33-4B6F-A64B-F05E682F127B"` with `title` `Bboxy450 GPU terminal in Rust (claude)` — the "Pane 1" row in the screenshot.
- `it2 session set-status --help` — `-s, --session <session>  Target session ID (default: active, or the pane named by --tmux)`, `--tmux <tmux>  Value of $TMUX`, `--tmux-pane <tmux-pane>  Value of $TMUX_PANE`, `--quiet-if-unresolved  Exit 0 without a message if the tmux pane can't be resolved. For callers that run on every event…`.
- So: the mapping is **env var inherited by the hook process** (Claude Code spawns hooks with the parent environment; docs §7). No PTY lookup, no pid walk. Inside tmux it degrades to `$TMUX`+`$TMUX_PANE` and iTerm2's `session_id_for_tmux_pane` builtin (`strings iTerm2 @80924`).

## 3. `cc-status` internals (Swift, recovered from symbols + disassembly)

Symbols: `nm -arch arm64 | swift-demangle` → module `cc_status`, types `Agent` (enum: claude / codex), `Address`, `StatusUpdate` (+ nested enum `TurnEnding`), `CurrentStatus`, class `It2Runner` (fields `address: Address`, `agent: Agent`). Functions and their order in `main()` (`cc-status.trace` lines 7-35):

1. `ProcessInfo.processInfo.environment`
2. `Agent.init(arguments:)` — inline strings `'--agent'`, `'codex'`: **`cc-status --agent codex` selects the Codex dialect; default is Claude.**
3. `readHookEvent() -> (name, json)?` — `FileHandle.standardInput.readDataToEndOfFile`, `JSONSerialization.jsonObject`, key `hook_event_name` (`'hook_eve'`+`'nt_name'`). Stdin is the hook payload.
4. `resolveAddress(env)` (§2). If it returns nil the program exits 0 silently (`--quiet-if-unresolved` semantics).
5. `statusUpdate(forEvent:json:address:agent:) -> StatusUpdate?` (§4).
6. `It2Runner(address:eventName:agent:)` — `init` checks `'SessionS'`/`'SessionE'` prefixes → `isSessionBoundary` field.
7. `It2Runner.send(StatusUpdate) -> CurrentStatus?` (§5).

Helpers (trace lines 90-630): `backgroundTaskCount(_:excludingID:)` reads `background_tasks` (cstring `@…` `'background_tasks'`), counts entries whose `status` is not in the terminal set (`completed`, `failed`, `cancelled`, `canceled`, `killed`, `stopped` — `strings cc-status`), excluding the entry whose id equals the current `agent_id`; `backgroundDetail(count:)` → `"1 background task running"` / `"N background tasks running"`; `storedBackgroundTaskCount` → `it2 session get-background-tasks`; `storedStatus` → `it2 session get-status`; `changeBackgroundTaskCount(by:)` → `--background-tasks-delta N --json`; `condense(_:limit:)`; `shortPath`; `shortURL`; `humanize` (mcp tool names); `applyPatchFiles` (Codex `apply_patch` bodies: `*** Add File:`, `*** Update File:`, `*** Delete File:`).

Detail text builders:

- `permissionDetail(json)` — `tool_name`/`tool_input`; `AskUserQuestion` → the `question` string; `ExitPlanMode` → `"Review proposed plan?"`; else `"Allow " + toolCallSummary`.
- `toolCallSummary(toolName:toolInput:)` — `Bash` → `"Bash: " + condense(command)`; `Read`/`Edit`/`Write`/`MultiEdit`/`NotebookEdit` → `"<Tool>: " + shortPath(file_path)`; `Grep` → `"Grep " + pattern + " in " + shortPath(path)`; `Glob` → `"Glob: " + pattern + " in " + path`; **`Agent` and `Task` → `tool_input.description`**; Codex `apply_patch` → `"Edit: " + files`; Codex `update_plan` → `"Update plan"`; `WebFetch` → shortURL(url); `WebSearch` → query; `mcp__server__tool` → humanize.
- `lastMessageDetail(json)` — `condense(last_assistant_message)`.

Colors (cstrings): idle dot `#00d75f` text `#888888`; working `#ff9500`; waiting `#5f87ff`; error `#ff3b30`; done `#00d75f`. `it2 session get-status` on this pane while this subagent ran — `{"status":"working","turn_open":null,"detail":null,"text_color":"#ff9500","background_tasks":5,"dot_color":"#ff9500"}`; `get-background-tasks` → `5` (the running subagents of the Oracle session, counted from the hook payload). **`detail` is null and nothing names a subagent**: proof that channel (1) does not carry the child rows.

## 4. `statusUpdate(forEvent:)` — event → it2 update (trace lines 1066-1185, inline-string order)

| Hook event | What cc-status reads | Update sent |
|---|---|---|
| `UserPromptSubmit` | — | `--status working`, `--turn-open true` |
| `PreToolUse` / `PostToolUse` | `tool_name`, `tool_input` | `--status working --detail "<toolCallSummary>"`; progress ring written to tty (§5) |
| `PermissionRequest` | `tool_name`, `tool_input` | `--status waiting --detail "<permissionDetail>"` |
| `Notification` | `notification_type`, `message` | `idle_prompt` → `--status idle` with background detail; `permission_prompt` → `--status waiting --detail condense(message)` |
| `Stop` | `last_assistant_message`, `background_tasks[]` | `TurnEnding`: `--status done --detail "<last message>"` or `"Interrupted"`; `--turn-open false`; `--background-tasks <running count>`; if tasks still run, the done status is deferred (`endingIfTasksFinished`, `onlyWhenFinished`, `--if-background-tasks` compare-and-set) |
| `SubagentStop` | `agent_id`, `background_tasks[]` | recount excluding `agent_id`, else `--background-tasks-delta -1 --json`; detail = backgroundDetail |
| `StopFailure` | `background_tasks[]` | `--status error` (+ background detail) |
| `SessionStart` | — | session boundary: reset (`--status idle`, `--turn-open false`, `--background-tasks 0`) **Gap:** exact arg set not read from bytes; inferred from `isSessionBoundary` + `It2Runner.init` |
| `SessionEnd` | — | session boundary: clear status **Gap:** same |

Strings `'Subagent'`+`'Start'` exist in the binary (`smallstr` output `'Suba'`,`'Subage'`,`'Subagent'`,`'Star'`,`'Start'`), so the binary knows `SubagentStart`, but iTerm2 never binds it (§1). Codex dialect: same table keyed on the Codex hook names (`strings` shows `codex`, `apply_patch`, `update_plan`).

## 5. `It2Runner.send` — exact `it2` invocation and the tty side-effect (trace lines 1305-1388)

Argument build order: `--status`, `--dot-color`, `--text-color`, `--detail`, `--background-tasks` | `--background-tasks-delta`, `--expires-on progress-end` + `--then-status idle --then-dot-color #00d75f --then-text-color #888888 --then-detail …` (when `expiresOnProgressEnd`), `--json`, then address args (`--session <uuid>` or `--tmux … --tmux-pane … --quiet-if-unresolved`), then `--turn-open`, `--if-turn-open`, `--if-background-tasks`. Runs `it2 session set-status …` via `Process` (`resolveIt2()`: `<dir of cc-status>/it2` if executable, else `/usr/bin/env it2` — strings `'cc-statu'`, `'it2'`, `'/usr/bin'`, `'/env'`). Parses the `--json` reply into `CurrentStatus{status, background_tasks, turn_open}`; stderr `cc-status: failed to run it2` / `cc-status: it2 exited N for <event>`.

`writeProgressRing(for:)` — `agentTTYPath()` opens `/dev/tty` for writing (checks `S_IFCHR`), and writes **`ESC ] 9 ; 4 ; 3 BEL`** when the resulting status is `working`, else **`ESC ] 9 ; 4 ; 0 BEL`** (`python3` byte search: `@73221 b'\x1b]9;4;3\x07'`, `@73274 b'\x1b]9;4;0\x07'`). That is the OSC 9;4 progress protocol: indeterminate ring on the tab while working, cleared otherwise. `--expires-on progress-end` ties the `working` status to it: when Claude Code's own `OSC 9;4;0` arrives the status falls back to the `--then-*` values (`it2 session set-status --help`: "Expire this status by itself when an event happens. The only value is progress-end: the session reported through the progress protocol (OSC 9;4) that the operation running when the status was set has ended."). `strings iTerm2 @44565` `Not setting progress from program status: OSC 9;4 or a reset got there first`.

## 6. `it2` → iTerm2: transport and the builtin function

- `strings it2 2385-2406` — unix socket + HTTP upgrade: `Failed to connect to iTerm2 socket at`, `Upgrade: websocket`, `Sec-WebSocket-Protocol: api.iterm2.com`, `Origin: ws://localhost/`, `x-iterm2-library-version: swift 1.0`, `x-iterm2-advisory-name: it2`, `x-iterm2-disable-auth-ui: true`, `x-iterm2-cookie:`. Socket: `~/Library/Application Support/iTerm2/private/socket` (`ls` shows `srw-------`). `it2 auth cookie` prints `ITERM2_COOKIE=… ITERM2_KEY=…` (`strings it2 2054`); `~/Library/Application Support/iTerm2/disable-automation-auth` exists on this machine so no cookie is needed. `it2 2367` `<socket path>,<server pid>,<session id>`.
- `strings it2 2225-2236` — the RPC it invokes: `iterm2.set_session_status(` with args `session_id:`, `status:`, `dot_color:`, `text_color:`, `detail:`, `background_tasks:`, `background_tasks_delta:`, `turn_open:`, `if_turn_open:`, `if_background_tasks:`, `expires_on:`, `then_status:`, `then_dot_color:`, `then_text_color:`, `then_detail:` (CodingKeys `strings it2 4952-5010`: `_status _dotColor _textColor _detail _backgroundTasks _backgroundTasksDelta _turnOpen _ifTurnOpen _ifBackgroundTasks _json _expiresOn _thenStatus _thenDotColor _thenTextColor _thenDetail`). Readers: `iterm2.get_session_status(session_id:)`, `iterm2.get_background_task_count()`.
- `strings iTerm2 @80936-80945` — server side: `set_session_status`, `registerSetSessionStatus()`, `sources/Language/BuiltInFunctions/SetStatusBuiltInFunction.swift`, error `SetStatus.ThenWithoutExpiresOn` "The then_ arguments only mean something with expires_on". `@80928` `get_session_status`, `@80923` `session_id_for_tmux_pane`. These are iTerm2 **built-in interpolated-string functions** invoked over the Python API's `invoke_function` RPC, not a protobuf message of their own.
- Storage: `it2 session get-status --help` — "turn_open is null until set-status --turn-open has been called"; "stored in memory" (`--background-tasks` help). Status state is in-memory per session, not a session variable (`it2 session get-var` is a different path).
- `strings iTerm2 @36554` `tabStatusUpdateForOSC21337Payload:`, `@96022` `VT100TabStatusUpdate` with fields `status`, `dotColor`, `textColor`, `detail`, `backgroundTasks`, `backgroundTasksDelta`, `turnOpen`, `expirationFallback`, `*Presence` (`@1587`, `@2709`, `@3882`, `@4047`, `@6369-6398`) — the same record is also settable by **OSC 21337** from the PTY.
- OSC 21337 grammar — `strings iTerm2 @81649-81665` (Session Status help text): `printf '\e]21337;status=Working;indicator=#ffa500\a'`; keys separated by `;`: `status=TEXT`, `indicator=COLOR`, `status-color=COLOR`, `detail=TEXT` (wraps up to 3 lines); colors `rgb:RR/GG/BB` or `#RRGGBB`; empty value clears; `\e]21337;status=;indicator=\a` clears all. Triggers ("Set Tab Status" regex trigger) are the third writer.
- Codex emits it: `codex @0xb3f1fa8` `\e]21337;status=…;indicator=…;status-color=;detail=…` and `@0xb8d8e4d` `\e]21337;status=;indicator=;status-color=;detail=` (clear), with its palette `Idle #00d75f Working #ff9500 Waiting #5f87ff` (same hex as cc-status). Claude Code has the encoder (`claude @0xd49286e` `VRs(e)`: `indicator=`, `status=` with `\` and `;` escaped, `status-color=`; `ojn` = clear) but it is gated off: `function Mtn(){return!1}`.

## 7. OSC 7501 "Program Status Protocol" — the channel that draws child rows

iTerm2 side:

- `strings iTerm2 @81666` — "**Program Status Protocol (OSC 7501):** A program can report that it is idle, working, waiting on you, done, or failed, and can report on several tasks at once. A session with more than one lists up to four of them, most urgent first, under its own row." That sentence is the screenshot: parent row = session, child rows = reports.
- `strings iTerm2 @90267` `sources/VT100/iTermProgramStatus.swift`; types `ProgramStatusReport`, `ProgramStatusReportBatch`, `ProgramStatusDisplayRecord`, `ProgramStatusController`, `ProgramStatusNotifier`, `ProgramStatusRecords.Kind`, `.State` (`iTerm2 @0x570c582`). Parser diagnostics (`iTerm2 @0x1aba05c`): `Discard OSC 7501 report of N bytes: too long`, `Discard OSC 7501 report: key … exceeds its limit`, `Ignore OSC 7501 report with missing or unknown state`, `Ignore OSC 7501 report with malformed id`, `Skip malformed OSC 7501 pair`. `@90269` `Keypress marked finished records seen` — a keypress in the pane marks done/error records as seen.
- Plumbing: `@36931` `terminalReportProgramStatus:`, `@36921` `terminalRemoveProgramStatusRecords`, `@30286` `screenReportProgramStatusReports:`, `@44563` `-[PTYSession screenReportProgramStatusReports:]`, `@13018` `applyTabStatusFromProgramStatus:`, `@33456` `setProgressFromProgramStatus:`, `@27539` `postProgramStatusNotificationWithDescription:`, `@44560` `PTYSession.TaskStatusChangedDescription`, `@29499` `restoreProgramStatus:` (survives session restore).
- Display vocabulary (`@90250-90266`): states `idle`, `working`, `blocked` (shown as waiting); blocked kinds render as `ProgramStatus.NeedsLogin` "Waiting for login", `NeedsAnswer` "Waiting for an answer", `NeedsApproval` "Waiting for approval"; `ProgramStatus.TitledDetail` "%1$@ … %2$@" = task name + what it is doing. Kind strings nearby: `permission`, `question`.
- Capability probe: iTerm2 holds the literal `\e]7501;?\e\` (`iTerm2 @0x570c582`, next to `ProgramStatusReport`). Claude Code's matcher (below) accepts any OSC 7501 reply whose payload starts with `?`, so **the reply is the probe echoed back: `ESC ] 7501 ; ? ESC \`**. **Gap:** whether iTerm2 appends anything after `?` — not visible in strings; echo satisfies Claude Code's matcher either way.
- Public docs: `WebFetch https://iterm2.com/documentation-escape-codes.html` — documents OSC 21337 (`indicator`, `status`, `status-color`) and OSC 9;4; **does not document 7501**. `https://iterm2.com/documentation-claude-code.html` → 404. The in-app help text is the only prose.

Claude Code side (emitter), `/opt/homebrew/Caskroom/claude-code/2.1.304/claude` (`claude --version` → `2.1.304`):

- `@0xde6962b` changelog 2.1.295: "Added Program Status Protocol (OSC 7501) support: terminals that implement it can show whether Claude Code is working, waiting on you, or done".
- `@0xd491db3` OSC table: `PROGRAM_STATUS:7501, TAB_STATUS:21337, ITERM2_PROPRIETARY:1337, ITERM2:9 …`.
- `@0xdbd981d` probe: `function PZo(){return{request:Ey(Uf.PROGRAM_STATUS,"?"),match:(e)=>e.type==="osc"&&e.code===Uf.PROGRAM_STATUS&&e.data.startsWith("?")}}` — sends `ESC ] 7501 ; ? ESC \` at startup alongside XTVERSION, `CSI ? u`, DECRQM 2026/1016 (`@0xdc13f04`, timeout `W1=2000` ms); on reply `u.answer("programStatus",true,"probe: OSC 7501 reply")`, otherwise `"probe: no reply to OSC 7501 ; ?"`. **No reply → Claude Code never emits 7501.**
- `@0xd492a56` encoder, verbatim:
  ```js
  var tt=2048,et=192,ot=32;
  function KRs(e){let n=e.replace(/[^A-Za-z0-9_.+-]/g,"-").slice(0,ot);return n.length>0?n:void 0}
  function I(e,n){let o=Mye(e.replace(U," "),n);return O.from(o).toString("base64")}
  function GTr(e){let n=[`state=${e.state}`,"app=claude-code"];
    if(e.id!==void 0)n.push(`id=${e.id}`);
    if(e.state==="blocked"&&e.kind)n.push(`kind=${e.kind}`);
    if((e.state==="working"||e.state==="blocked")&&e.progress!==void 0&&Number.isFinite(e.progress)){let o=Math.min(100,Math.max(0,Math.round(e.progress)));n.push(`progress=${o}`)}
    if(e.title)n.push(`title=${I(e.title,et)}`);
    if(e.msg)n.push(`msg=${I(e.msg,tt)}`);
    return Ey(Uf.PROGRAM_STATUS,n.join(":"))}
  var sjn=`${x}${Uf.PROGRAM_STATUS};state=clear${T6}`;
  function qRs(e){return Ey(Uf.PROGRAM_STATUS,`state=clear:id=${e}`)}
  ```
  Wire form: **`ESC ] 7501 ; state=<s>:app=claude-code[:id=<id>][:kind=<k>][:progress=<0-100>][:title=<base64 utf8, ≤192 bytes>][:msg=<base64 utf8, ≤2048 bytes>] ESC \`**. Pairs joined by `:`; `id` is `[A-Za-z0-9_.+-]{1,32}`; `title`/`msg` are base64 so `:`/`;` never appear raw. Clear-all `state=clear`; clear-one `state=clear:id=<id>`. (`Mye(t,n)` `@0xccabed3` truncates to `n` UTF-8 bytes without splitting a code point.)
- `@0xec2bb16` state mapping for the **main** report `kxe(h)`: `busy` or `idle`+queued → `{state:"working",progress,msg:workingDetail}`; `waiting`+`blockedOn` → `{state:"blocked",kind:dIo[blockedOn],msg}`; `lastTurnEnd==="completed"` → `{state:"done",msg:doneDetail}`; `null`/`"stopped"` → `idle`; API error → `{state:"error",msg:failure}`; auth errors (`authentication_failed`, `oauth_org_not_allowed`, `cloud_credential_error`) → `{state:"blocked",kind:"auth"}`. `dIo = {"permission prompt":"permission","sandbox request":"permission","worker request":"permission","goal proposal":"permission","input needed":"question","dialog open":undefined}`. So the on-wire state set is **`idle | working | blocked | done | error | clear`** and `kind ∈ {permission, question, auth}`.
- `@0xec2bdd1` **subagent reports** `mIo(h)`: for every task `H` in the task registry, `id = KRs(H.id)`, included when `H.type ∈ jbo` (agent task types) and `lut(H)` (still running) and `E0t(H)!=="main-session"`; row = `{id, title: lft(H.description), ...pIo(H)}` where `pIo`: `remote_agent` with `ultraplanPhase==="needs_input"` → `blocked/question`, `"plan_ready"` → `blocked/permission`; `status==="paused"` → `blocked`; else `working`. Sorted by id, capped at `uIo=32`. `lft` drops a description that contains `…`. **That is why each child row carries the Agent tool's `description` and reads `working`.**
- `@0xec2c5f9` emitter loop `Txe(h,R,L)`: runs only when `gx().now("programStatus")` (probe answered) and `!wn()`; keeps a `Map<id, lastOSC>`; the main report has id `""` (`SIo=""`, no `id=` on the wire); on every render it concatenates only the **changed** reports and `state=clear:id=X` for ids that vanished, writes them in one `H(Be)`; on unmount writes `state=clear`. `Nir` (`@0xdc27ea7`, restoreTerminalModes) also appends `state=clear` + `OSC 9;4;0` + OSC 21337 clear on exit/restore.
- The row named `claude-code` in the screenshot is the id-less main report: iTerm2 names it from `app=` when a session has more than one report ("under its own row").

Codex side, `/opt/homebrew/Caskroom/codex/0.163.0/bin/codex` (`codex --version` → `codex-cli 0.163.0`): `@0xb3f1fa8` `\e]7501;state=…:app=codex\e\`; `@0xb8d8e4d` `\e]7501;state=clear\e\`; state words present `idle working blocked`. Byte counts: `title=` 4, `kind=` 3, `msg=` 2 occurrences, none adjacent to 7501 — Codex 0.163 reports **one** state for itself, no tasks, no title. **Gap:** whether Codex probes before emitting (no `7501;?` literal found; it may emit unconditionally).

## 8. iTerm2's own model around the panel

- Panel: `strings iTerm2 @52512` `Toolbelt.Tool.SessionStatus`, `@1875` `iTermSessionStatusController`, `@81620-81700` help: rows show name, dot, status text, ⌘ shortcut; **sorted by priority** (default `waiting, done, error, working, idle`, user-configurable `StatusPriorities`), snooze moves a row to the bottom. This reordering is exactly what [[ux-sidebar]] rejects.
- `@87926` MCP tool text: sessions report `status (idle/working/waiting)`, `status_source` = `reported` ("the program announces its own status (accurate, and state watchers fire on exact transitions)") or `inferred` ("iTerm2 is guessing from indicators"), `kind (claude-code/shell/tui/other)`, `screen_last_changed`. `@87933` "the last cc-status detail message for Claude Code sessions". iTerm2 therefore keeps a per-session **hook-event log** fed by cc-status detail strings, used by its chat/MCP `get_screen_contents` ("For Claude Code sessions, returns a synthesized view of recent hook events").
- `@86574` `GlobalJobMonitor.swift` + `@71241` `ClaudeWatcher.swift` — process-name watcher (`jobName == "claude"`) that counts Claude sessions and, at a threshold, offers to install the status tool (`offerClaudeCodeStatusTool(sessionID:)`). Detection of "this pane runs Claude" is by foreground job name, not by hooks.
- No `subagents`, `meta.json`, `transcript_path`, `agent_transcript_path` strings anywhere in iTerm2 (`grep` on 243 339 strings) — iTerm2 never reads `~/.claude/projects`.

## 9. Claude Code hook payloads (docs + local transcript)

Docs: `WebFetch https://code.claude.com/docs/en/hooks` (redirect from docs.claude.com), three 100 kB windows.

- Common fields: `session_id`, `prompt_id`, `transcript_path`, `cwd`, `scratchpad_dir` (v2.1.257+), `permission_mode`, `effort`, `hook_event_name`; **`agent_id`, `agent_type` present when the hook fires inside a subagent** ("Hooks from settings files … also run inside subagents. When a subagent calls a tool, tool events such as PreToolUse and PostToolUse fire the same configured hooks … and the input carries the agent_id and agent_type").
- `SubagentStart`: `agent_id`, `agent_type` only. **No description.**
- `SubagentStop`: `stop_hook_active`, `agent_id`, `agent_type`, `agent_transcript_path` (`~/.claude/projects/…/<session>/subagents/agent-<id>.jsonl`), `last_assistant_message`, `background_tasks[]`, `session_crons[]`.
- `Stop`: `stop_hook_active`, `last_assistant_message`, `background_tasks: [{id, type, status, description, command}]`, `session_crons`.
- `PreToolUse`/`PostToolUse` for `Agent`: `tool_input.description`, `tool_input.prompt`, `tool_input.subagent_type`, `tool_use_id`; PostToolUse `tool_response` for a background launch = `{status:"async_launched", agentId, description, prompt, outputFile, …}` — **the only hook-side join between a description and an `agent_id`**.
- `Notification.notification_type` ∈ `permission_prompt idle_prompt auth_success elicitation_dialog elicitation_url_dialog elicitation_complete elicitation_response agent_needs_input agent_completed quota_auto_resume_* plugin_notification`. `StopFailure.error` ∈ `rate_limit overloaded authentication_failed oauth_org_not_allowed account_on_hold billing_error invalid_request model_not_found server_error max_output_tokens cloud_credential_error unknown`. `SessionStart.source` ∈ `startup resume clear compact fork`. `SessionEnd.reason` ∈ `clear resume logout prompt_input_exit other`.
- Env for hook processes: inherits parent env (minus `OTEL_*`), plus `CLAUDE_PROJECT_DIR`, `CLAUDE_ENV_FILE` (SessionStart), `CLAUDE_EFFORT`, `CLAUDE_PLUGIN_*`. Observed in this process: `CLAUDE_CODE_SESSION_ID`, `CLAUDE_PID`, `CLAUDE_CODE_MESSAGING_SOCKET=/tmp/cc-socks/<pid>.sock` (+ token, value not printed), `CLAUDECODE=1`, `CLAUDE_CODE_ENTRYPOINT=cli`, `CLAUDE_CODE_CHILD_SESSION=1`.
- Timeouts: command hooks 600 s default, 30 s on `UserPromptSubmit`; `SessionEnd` budget 1.5 s (`CLAUDE_CODE_SESSIONEND_HOOKS_TIMEOUT_MS`). Exit 0 success, 2 blocking (events that can block), other = non-blocking error shown in transcript. Stdout parsed as JSON only if a single object.
- Local transcript layout: `~/.claude/projects/-Users-feral-mydev-bboxy450-term/<session>.jsonl` (record keys include `type`, `sessionId`, `isSidechain`, `agentId`…); `<session>/subagents/agent-<id>.jsonl` + **`agent-<id>.meta.json`** = `{agentType, launchedWithDefaults, description, toolUseId, spawnDepth, requestShape:"background", requestNonInteractive, finished?:true}` (`cat` of five files; `finished` appears only once done). Parent transcript `assistant` records hold `tool_use {name:"Agent", input:{description,prompt,subagent_type}}` and `toolUseResult {status:"async_launched", agentId, description, …}`. A file watcher on `subagents/*.meta.json` is a second, hook-free way to the same rows; iTerm2 does not use it.

## 10. Swift inline-string caveat (why `strings` lies here)

Swift stores strings of ≤15 UTF-8 bytes inline in registers; `strings` never sees `agent_id`, `tool_name`, `Stop`, `--agent`, `/dev/tty`… They were recovered by reassembling `mov/movk` immediates from `otool -tV` (`scratchpad/smallstr.py`, output in `cc-status.trace`). Any future audit of `cc-status` or `it2` must do the same; a plain `strings` pass will miss every short key.

## 11. Verification inventory

Verified from local files/binaries: §1, §2, §3, §5, §6, §7 (all code quotes), §8, §9 transcript layout. Verified from docs: §9 payload tables. Inference only: §4 SessionStart/SessionEnd argument sets, the Codex hook history (§1 last bullet), iTerm2's exact 7501 reply bytes beyond `?` (§7). Not attempted: a live capture of a hook payload (would require editing `~/.claude/settings.json`); a Ghidra decompile (`rea` has no provider installed: `GHIDRA_INSTALL_DIR` unset, no Java, no Hopper).
