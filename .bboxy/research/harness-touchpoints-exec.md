---
id: "39PX7HT3QVKSXD66THK0RHNZEK"
type: "research"
number: 79
title: "Harness touch points — independent Executor pass"
created: "2026-10-21T19:16:54Z"
updated: "2026-10-24T00:10:38Z"
---
# Harness touch points — independent Executor pass

2026-10-21 · Nyx Copperwing <EXECUTOR> · assignment D.

[[vision]] · [[harness-touchpoints]] · [[agent-messaging]] · [[iterm-harness-plugin]] · [[subagent-detection]] · [[2026-10-21-harness-touchpoints-research]]

## Evidence boundary

Installed artifacts inspected: Claude Code **2.1.304**, Codex CLI **0.163.0**, Pi **1.0.0** (`@earendil-works/pi-coding-agent`), OMP **18.4.12** (`@oh-my-pi/pi-coding-agent`). Claude/Codex versions came from `--version`; Pi/OMP versions came from their installed `package.json`, with executable symlinks resolved to those packages. All four `--help` commands returned 0. No model turn or synthetic input was started for this research.

Pointers use these local roots:

| Alias | Root |
|---|---|
| `ICQ/` | `/Users/feral/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/icq-0.20.2/` |
| `Pi/` | `/opt/homebrew/lib/node_modules/@earendil-works/pi-coding-agent/` |
| `OMP/` | `/opt/homebrew/lib/node_modules/@oh-my-pi/pi-coding-agent/` |
| `TUI/` | `Pi/node_modules/@earendil-works/pi-tui/dist/` |
| `SDK/` | `/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk/usr/include/` |
| `Man/` | `/Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk/usr/share/man/man3/` |
| `ClaudeBin` | `/opt/homebrew/Caskroom/claude-code/2.1.304/claude` |
| `CodexBin` | `/opt/homebrew/Caskroom/codex/0.163.0/bin/codex` |

ICQ is the published **0.20.2 registry package**, revision `6524c3b689dc70ff9d379192f8a57a62c0c047ef`; it is not the older `~/mydev/icq-connect` repository. Pi's readable JavaScript and shipped docs were inspected; OMP's shipped TypeScript declarations establish interface contracts, not tested runtime behavior. Claude's embedded JavaScript and Codex's binary string fragments were read at the offsets below; `@0x…` means original file byte offset, not a line or virtual address. Primary online documentation was fetched on this date; a documented feature is distinguished from a feature exercised in this installation. Proof-limit conclusions below are inferences from the interfaces, not live race tests.

## Touch-point matrix

| Harness | Launch / role context | Primary observation / supplemental lifecycle | MCP / skills | Input into an existing seat |
|---|---|---|---|---|
| Claude | `--append-system-prompt`, `--system-prompt`, `--settings`; print-mode structured I/O is a separate mode | OSC 7501 main/task reports; secondary JSON hooks for detail/boundaries; print-mode hook stream | `--mcp-config`, project `.mcp.json`; personal/project skills | ICQ's owned Unix socket; structured print input for a process launched in that mode; PTY fallback only with readiness evidence |
| Codex | initial prompt, `-c developer_instructions=…`; replacement instructions file is a different operation | OSC 7501 main status, OSC 21337 hint; secondary hooks; app-server events for that integration | `mcp_servers` in config; `.agents/skills` discovery | installed `codex queue` command; ICQ exact-thread queue adapter; app-server input methods in an app-server integration |
| Pi 1.0.0 | system/append flags, SYSTEM/APPEND_SYSTEM files, extension prompt sections | TypeScript/JavaScript extension events; JSON/RPC event stream | native MCP config; user/project/shared skills | extension `sendUserMessage`; JSONL RPC when launched in RPC mode; ICQ session-bound bridge |
| OMP 18.4.12 | system/append/template flags; SDK prompt override | extension and hook modules; RPC; approval events | capability-based MCP discovery; skills under configured resource roots | extension `sendUserMessage`; JSONL RPC; ICQ session/generation-bound bridge |

These are alternatives at different process boundaries. An RPC/headless mode is not the same instance as an already running interactive TUI unless its documented attachment mechanism connects them. MCP supplies tools when the agent calls them; having an MCP server configured does not itself push a user message or prove readiness.

## Primary PTY observation: OSC 7501 and Codex OSC 21337

For Claude/Codex status, use OSC 7501 as the primary observation channel per the Oracle's addendum. Hooks remain secondary for tool/permission details and session boundaries. [[iterm-harness-plugin]] supplied the search leads; the offsets below were independently read from the installed artifacts. [[subagent-detection]] is a proposed consumer spec, not evidence that its parser or state rules already work.

| Local evidence | Mechanism established statically |
|---|---|
| `ClaudeBin @0xd491da4`, `@0xdbd981d` | OSC table assigns PROGRAM_STATUS=7501 and TAB_STATUS=21337. `PZo` constructs the 7501 `?` probe and accepts an OSC 7501 reply starting with `?`. Echoing the probe satisfies that matcher; no live exchange was captured. |
| `ClaudeBin @0xd49286e`, helpers immediately before it, `@0xccabed3` | `GTr` encodes colon-separated `state`, `app=claude-code`, optional `id`, blocked `kind`, bounded progress, base64 title/message. Helpers set 192/2048-byte title/message limits, truncate through TextEncoder.encodeInto, and sanitize IDs to 32 characters. Clear-all and clear-one encoders are adjacent. |
| `ClaudeBin @0xec2b869`, `@0xec2bb16` | Main-state mapping distinguishes working, blocked, done, idle, error/auth. `mIo` derives running agent-task reports with sanitized ID and description; paused/remote-input tasks can be blocked. This is evidence for individual Claude task rows. |
| `ClaudeBin @0xec2c5d3`, `@0xec2c63f` | `o7` reads the settled programStatus capability. `Txe` gates writes on it, writes changed reports, and clears removed task IDs/unmounted reports. Static code establishes the gate; it does not show a running process receiving the probe. |
| `CodexBin @0xb3f1fa7`, `@0xb8d8e4c` | Literal fragments contain OSC 7501 `state=` / `:app=codex` and a clear report. Formatting bytes between fragments are not captured wire output. This scan does not establish a Codex task-ID encoder or its enablement condition. |
| `CodexBin @0xb3f1f6e`, `@0xb8d8e1b` | OSC 21337 fragments set `status`, `indicator`, `status-color`, `detail`, plus an empty-value clear. iTerm2's [escape-code documentation](https://iterm2.com/documentation-escape-codes.html#session-status) confirms its status/indicator/color fields and omission/clearing semantics. Treat it as a display hint beneath 7501. |

Claude's encoded payload shape is `state=<s>:app=claude-code[:id=<id>][:kind=<kind>][:progress=<n>][:title=<base64>][:msg=<base64>]`; clear is `state=clear[:id=<id>]`. The main-state mapping and adjacent clear encoder cover idle/working/blocked/done/error/clear. Parse OSC framing before fields; base64 text remains untrusted child data. Bound payload and decoded text, associate reports with the owning seat/process generation, and keep the authenticated peer identity/role channel separate. A PTY writer can imitate another app's status string.

**Gap:** no live wire/probe capture was run; Codex probing and individual subagent reports remain unproven. No literal `7501;?` occurred in the Codex binary, which does not exclude a constructed probe. A bounded scan of installed Pi/OMP `.js`/`.mjs`/`.cjs` output plus Pi TUI found no 7501/21337 protocol identifier; OMP's two `7501` hits were color-matrix decimal fractions. Dynamic construction or dependencies outside that scan remain possible. Use their documented lifecycle interfaces until a runtime capture proves OSC support. Live capture is the separate assignment, not completed by this pass.

An observed `idle`, `done`, `clear`, or `blocked` report is not permission to type: a modal, approval dialog, stale generation or operator draft can still own input. Neither 7501 nor 21337 provides a consumable composer-ready token. Keep the interactive-input guard below even when primary status works.

## Claude Code

**Launch/instructions.** Installed `claude --help` confirms system and append-system flags, settings JSON/file input, session/resume selection, MCP configuration, and print-mode `--input-format stream-json` / `--output-format stream-json`. `--include-hook-events` exposes lifecycle records in the output stream. These flags were inspected, not executed with a prompt. The same help describes system-prompt snapshots across resumes; changing a later launch flag must not be assumed to replace an already recorded prompt. [CLI reference](https://code.claude.com/docs/en/cli-reference).

**Secondary hook files and states.** [Hooks reference](https://code.claude.com/docs/en/hooks) documents user/project/local settings hook sources. `UserPromptSubmit` marks submitted input; `PreToolUse`/`PostToolUse` surround tool work. `PermissionRequest` identifies an approval gate. `Stop` is a turn stopping boundary, not a certificate that stdin belongs to the composer. `StopFailure` covers an API-error stop; tool failures have their own events. `Notification` idle/permission messages are conditional notifications: `idle_prompt` can arrive roughly a minute later and depends on user absence/background work. Do not use its absence as active-state evidence.

**MCP/skills.** [MCP documentation](https://code.claude.com/docs/en/mcp) gives project `.mcp.json` and launch `--mcp-config`; project trust and managed policy affect loading. [Skills documentation](https://code.claude.com/docs/en/skills) gives `~/.claude/skills/<name>/SKILL.md` and project `.claude/skills/`. This machine's shared guidance keeps actual skill sources under `~/.agents/skills/`; preserve its links instead of making another copy.

**ICQ attach → wake.** `ICQ/src/session.rs:113` finds the nearest Claude ancestor; `ICQ/src/native.rs:186` validates its process-bound session record and obtains the messaging socket. Environment attachment also exists (`ICQ/src/harness.rs:236`), with ambiguity checks. `ICQ/src/harness.rs:429` writes newline JSON containing stable UUID, user content and native `priority: next`. At `:455`, success proves a completed socket write, not acceptance/model receipt. The common readiness evidence at `ICQ/src/harness.rs:80` keeps those outcomes separate.

ICQ's launch wrapper sets `crossSessionInbound: accept` (`ICQ/src/launch.rs:200`), preserves other settings, and rejects `--bare` (`:66`). User settings here contain `crossSessionInbound: accept`, but that does not prove the effective receiving session's managed/local policy. No setting was changed in D.

**Gap:** neither the complete private socket contract nor effective inbound acceptance was exercised. Hook presence, `Stop`, or a ready socket does not prove an idle composer or that a previous queued message was consumed.

## Codex CLI

**Launch/instructions.** Installed help confirms `-c/--config`, `-C/--cd`, profiles and `--no-alt-screen`; `codex exec --json` is a noninteractive JSONL output mode. The [configuration reference](https://learn.chatgpt.com/docs/config-file/config-reference) describes additive `developer_instructions` and a separate `model_instructions_file` replacement. Use the additive surface for role context without treating an ordinary initial user prompt as system instructions.

**Secondary hooks, MCP, skills.** [Official OpenAI hooks documentation](https://learn.chatgpt.com/docs/hooks) lists SessionStart/End, UserPromptSubmit, tool, permission, stop, interrupt and subagent events. Hooks load next to active config layers as `hooks.json` or inline tables; project and hook trust affect execution. `SessionStart` and `UserPromptSubmit` may supply additional developer context. [MCP](https://learn.chatgpt.com/docs/extend/mcp) is configured in user `~/.codex/config.toml` or a trusted project's `.codex/config.toml`. [Build skills](https://learn.chatgpt.com/docs/build-skills) documents repository `.agents/skills` discovery up to the repository root. Keep shared sources in `~/.agents/skills` as required on this machine.

**Structured observation.** [App-server documentation](https://learn.chatgpt.com/docs/app-server) provides `turn/started`, `turn/completed` with completed/interrupted/failed status, thread status notifications, approval requests, user-input requests and their resolution notifications. Track outstanding request IDs to distinguish waiting from active work; completion of a tool item is not completion of its turn. `thread/status/changed` can carry `waitingOnApproval`. App-server transport and a terminal TUI are separate integration choices.

**Existing-session input.** Installed `codex queue --help` confirms `--thread <UUID-or-exact-name> --message <text>` and remote endpoint options. Only help was run. For the owned-socket mechanism, `ICQ/src/session.rs:79` verifies process ancestry and candidate endpoints; `ICQ/src/harness.rs:524` initializes its Unix WebSocket and reads the exact loaded/healthy thread. `ICQ/src/harness.rs:591` calls `thread/queue/add`, validates the returned client-message/queue IDs, and preserves ambiguous outcomes rather than replaying them. The fetched public app-server page did not document that queue method; its evidence is the installed CLI surface plus ICQ source, not a claimed public protocol guarantee.

**Gap:** no queue submission, event subscription, approval cycle or runtime hook test was run. This machine's Codex hook file currently configures only SessionStart; the broader documented event set is not thereby installed or trusted. Window-title bytes are display hints, not an authoritative thread-state API.

## Pi 1.0.0

**Launch/instructions/resources.** Installed help and `Pi/docs/configuration.md:18` expose user instructions plus SYSTEM/APPEND_SYSTEM files. `:30` lists project `.pi/` resources. The default agent directory is `~/.pi/agent`, overridable by `PI_CODING_AGENT_DIR` (`:9`). `Pi/docs/skills.md:59` describes user/project skills and shared `.agents/skills`; `Pi/docs/mcp.md:32` gives native user `~/.pi/agent/mcp.json` and project `.pi/mcp.json`. This version has native MCP support; an older “Pi has no MCP” statement would be wrong.

`Pi/dist/core/extensions/types.d.ts:693` exposes structured prompt sections before the agent starts. `Pi/docs/extensions.md:15` describes extension factories and `--extension`; these executable extensions provide lifecycle hooks rather than Claude-style settings command hooks. Project trust controls project resource loading (`Pi/docs/configuration.md:3`).

**State proof.** `Pi/dist/core/extensions/types.d.ts:705` / `:781` identify agent-loop and assistant-turn starts. At `:761`, `agent_settled` explicitly distinguishes final settlement from `agent_end`. `Pi/dist/core/agent-session.js:671` emits settlement after clearing the active-run flag; `:1038` defines session idle from run/compaction state. `Pi/docs/json.md:48` explains why retries/queued work can follow agent_end. Blocking extension dialogs have `ui_prompt_start`/`ui_prompt_end` (`Pi/dist/core/extensions/types.d.ts:766`), which identify UI waiting, not a universal permission policy. Tool error evidence is `tool_execution_end.isError` (`:831`); retain provider/retry events rather than declaring every transient error terminal. `:118` offers a title setter, not identity proof.

**Input/attach.** `Pi/docs/rpc.md:3` describes a long-lived JSONL subprocess; `Pi/docs/rpc-commands.md:20` requires an explicit steering/follow-up policy during streaming. A successful prompt response is admission, not completion (`Pi/docs/rpc.md:60`). Extension `sendUserMessage` accepts follow-up delivery and a prompt-expansion switch (`Pi/dist/core/extensions/types.d.ts:1222`). ICQ creates its bridge through `ICQ/extensions/pi.ts:6`, registers session lifecycle at `ICQ/extensions/host.mjs:38`, and validates its descriptor in `ICQ/src/session.rs:178`. Dispatch reaches `ICQ/extensions/bridge.mjs:55`; it requests follow-up delivery with template expansion disabled.

**Gap:** no Pi extension/RPC launch or event smoke test was performed. Session-level settled is stronger than a per-turn end, but still does not prove the terminal input editor has no user draft or modal UI. No generic Pi permission-request hook is assumed beyond the observed UI/tool extension contracts.

## OMP 18.4.12

**Launch/resources.** Installed help confirms system/append/template overrides, `--extension`, extra `--config` overlays, skill selection, and RPC/RPC-UI modes. `OMP/dist/types/cli/args.d.ts:28` and `OMP/dist/types/sdk.d.ts:128` expose those prompt surfaces. `OMP/dist/types/config.d.ts:40` defines user `~/.omp/agent/` and project `.omp/` resource roots. Hook-module discovery includes OMP/Pi roots and installed providers (`OMP/dist/types/extensibility/hooks/loader.d.ts:55`); MCP uses its capability loader (`OMP/dist/types/mcp/config.d.ts:35`). These declarations do not establish every provider's filename/precedence. Unlike current Pi, OMP's context contract says project trust is always true (`OMP/dist/types/extensibility/extensions/types.d.ts:329`).

**State proof.** `OMP/dist/types/extensibility/shared-events.d.ts:162` defines starts; `:166` adds `willContinue` to agent_end, so an end event need not mean settled. `:95` exposes `session_stop`, whose handlers can request continuation. `OMP/dist/types/session/agent-session.d.ts:281` declares run-state subscription; `:376` declares waitForIdle including deferred recovery; `:550` exposes post-prompt work. Approval requested/resolved events carry session/tool-call IDs (`OMP/dist/types/extensibility/extensions/types.d.ts:723`). Correlate them rather than guessing from a pause. Tool errors have `isError` (`:652`); title setting is available at `:155`.

**Input/attach.** `OMP/dist/types/modes/rpc/rpc-types.d.ts:24` declares JSONL prompt/steer/follow-up commands. `OMP/dist/types/session/agent-session-types.d.ts:418` declares `sendUserMessage` delivery choices. ICQ's OMP entry point is `ICQ/extensions/omp.ts:6`; the host refreshes identity/generation on switch/branch/tree (`ICQ/extensions/host.mjs:64`) and closes stale attachments before transitions. `ICQ/src/harness.rs:392` verifies bridge identity, then sends a stable-ID dispatch and checks its acknowledgment. This is an extension call acknowledgment, not model receipt.

**Gap:** ICQ's shared bridge supplies `expandPromptTemplates: false` (`ICQ/extensions/bridge.mjs:55`), but OMP's current SendUserMessageOptions at `:418` does not declare that field. Its separate FollowUpOptions at `:404` does. Whether the shared adapter actually disables expansion on OMP is unproven; do not infer it from Pi compatibility. No fix or runtime test was made. Exact MCP-provider precedence and OMP generic dialog waiting behavior also remain untested.

## PTY-only signals: what they prove

| Signal | Evidence / limit |
|---|---|
| OSC 7501 / 21337 | Artifact evidence above supplies primary application-reported status and supplemental display text. Reports can identify Claude tasks; they do not authenticate the writer, prove editor focus, or acknowledge a message. |
| Foreground process group | `Man/tcgetpgrp.3:49` and `SDK/sys/ttycom.h:121`: identifies a tty's foreground group when the query succeeds. A harness, shell, child tool or same-group descendant can still own input; an error means unknown. |
| ICANON / ECHO | `SDK/sys/termios.h:240` / `:247`: echo and canonical line discipline. Raw full-screen editors and password prompts can both disable echo. Flags do not identify the composer or authorize text. |
| Output silence / queue length | Silence only measures no recent output. `SDK/sys/ttycom.h:124` defines an output-queue count. Neither excludes network waits, suppressed output, computation or buffered redraw. |
| Cursor / DSR | [XTerm sequences](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html) define cursor reports. DSR travels application→terminal; CPR replies terminal→application. Sending query bytes into a child's stdin is not a readiness poll. The terminal already owns its parsed cursor state. |
| OSC title / cwd | [WezTerm shell integration](https://wezterm.org/shell-integration.html) and [pane metadata](https://wezterm.org/recipes/passing-data.html): OSC 0/2 supply titles, OSC 7 supplies a cwd URI. Child-supplied labels can be stale or spoofed; they are not authenticated lifecycle events. |
| Prompt zones | The same shell-integration document describes OSC 133 zones. A cooperating shell can mark prompt/input/output boundaries. That is evidence about that shell, not arbitrary nested harness prompts. |
| Bracketed paste | `TUI/terminal.js:120` / `:343` enable/disable mode 2004. [XTerm](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html) defines the 200/201 paste brackets. It distinguishes pasted text from keystrokes; it does not prove the correct recipient or whether Enter will submit. |
| Synchronized output | `TUI/tui-main-screen.js:241` / `:264` bracket rendering with mode 2026. This is a display transaction boundary, not a promise that the application is ready for input. A missing close is an unknown render state, not a prompt signal. |

The tty observations above are local. An ssh/tmux remote foreground process is not identified by the local ssh process group alone. **Gap:** no portable PTY-only signal proves that an arbitrary hookless harness is at its prompt; no live PTY, remote-process or synchronized-render test was run here.

## Interactive-input guard — proposed contract

1. Bind a ready indication to **seat ID + harness session ID + process start + attachment generation**. Reject stale indications after restart/resume/switch. ICQ's process/descriptor checks above supply a reference mechanism; screen text is not the identity channel.
2. Prefer native queue/RPC/extension delivery, even while the model is active, when its documented queue semantics permit it. Keep admission, dispatch and model-consumption evidence distinct. Do not fall back to typing after an ambiguous native submission: the first attempt may already have arrived.
3. For PTY fallback, require a current adapter/hook indication that the composer owns input, no outstanding modal/approval request, and no active tool/input handoff. Under the mux input arbiter, recheck generation, foreground group, buffered parser/render state and operator draft/keystrokes. Output quiescence and tty flags are negative checks, never the positive proof.
4. If eligible, render the peer message as literal data, retain original signed bytes separately, respect the advertised paste mode, serialize the paste/submit sequence, and journal a stable delivery ID before writing. A possible partial write remains unknown; terminal bytes alone are not a durable exactly-once delivery mechanism.
5. For a hookless harness with no explicit ready handshake, leave automatic submission pending for operator handoff. Do not fabricate certainty from a blinking cursor. This is the consequence of the proof gap, not a tested implementation.

**Gap:** even ready hooks plus a later tty write have a check/write race unless the harness participates in a consumable readiness/input handshake. The generic guard is not implemented; user-draft preservation, modal transitions, process replacement and partial writes need implementation tests before automatic fallback can be called reliable.

## Verification and provenance ledger

| Command/read | Actual result |
|---|---|
| `command -v claude`, `codex`, `pi`, `omp`; resolve executable paths | Exit 0; installed artifacts identified above. |
| `claude --version`, `codex --version`; four `--help` commands | Exit 0; versions/help inspected, not inference calls. |
| `codex app-server --help`, `codex exec --help`, `codex queue --help` | Exit 0; inspected interfaces only. No app-server or queue submission started. |
| Targeted `rg -n`, `nl -ba … | sed -n …p` on installed sources/docs/types | Exit 0 for evidence reads; exploratory no-match searches returned 1 and were not treated as proof. |
| Whitelisted local-config structure read | Exit 0; Claude has ten configured event names and user inbound policy accept; Codex hooks file has only SessionStart. Configured is not effective/trusted. Credential values were never printed or copied. |
| Primary online docs fetches | OpenAI hooks/app-server/config/MCP/skills, Claude docs, XTerm and WezTerm read. Old guessed OpenAI MCP/skills URLs returned 404; canonical current pages were subsequently fetched. POSIX web pages returned 403; installed macOS manual/header evidence was used instead. |
| Python read-only mmap/byte search and SHA-256 of `ClaudeBin` / `CodexBin` | Exit 0; offsets and fragments above independently reproduced. No target was launched for this scan. |
| REA `binary_session {}` | Successful status result, no target open. Ghidra/Hopper/IDA/Kuna were all unavailable or unconfigured. No decompilation, provider installation or configuration change was attempted; byte inspection above is not control-flow analysis of Codex. |
| Bounded `rg -l` for `7501\|21337\|PROGRAM_STATUS\|programStatus` under Pi/OMP installed JS and Pi TUI; Python hit inspection | Both commands exit 0; only OMP cli.js matched, and its two numeric hits were unrelated color fractions. Search coverage is limited to those roots/extensions. |
| Required file-existence / gap-count command from Task D's Verification section | Exit 0; file exists, count 8. |
| Python source-pointer/structure/whitespace check | Exit 0; 47 qualified source pointers exist within file line ranges, all eight required sections present, no trailing whitespace, final newline present. Pointer ranges alone do not certify semantic correctness. |
| `git diff --check`; `git status --short` | Exit 0; no whitespace errors. At handoff the only modified path was this deliverable; Oracle had committed its earlier draft. No commit was made by this Executor. |

Representative source fingerprints, SHA-256 over ordered `(UTF-8 relative path, NUL, raw bytes, NUL)` records:

- ICQ: `src/harness.rs`, `src/session.rs`, `src/native.rs`, `extensions/host.mjs`, `extensions/bridge.mjs` → `fcfe4e13e84c39259480d7b33583fbdc8ce6197fc3f542c24de3129337911a2d`.
- Pi: `package.json`, `dist/core/extensions/types.d.ts`, `dist/core/agent-session.js` → `e9065c7566f4033bf5908934aa26a5c0840bad11107fd91677b8ce0702adedc6`.
- OMP: `package.json`, `dist/types/extensibility/shared-events.d.ts`, `dist/types/session/agent-session-types.d.ts` → `bdee0d56159c47045d4f831f572d1efda61cc576fd717bd47f893a8c9246a552`.

Native artifact SHA-256 (whole files, independent of the source-record hashing convention above):

- `ClaudeBin`, 271161296 bytes → `b9ac5374839f14abf3f1f45610637c332b5daee5ce0d1634726cef87aab78e6a`.
- `CodexBin`, 249952688 bytes → `8813367285524ac798dd96d1b071aea30d58708c7226fd236cd275cf1cfdee31`.

**Gap:** all runtime transition/injection claims remain unexercised. This pass supplies installed interface/source evidence and primary documentation, not a compatibility certification. No memory-only assertion is offered as proof. No application code, harness configuration, credentials, task board or parallel research file was edited; only this deliverable was written. Oracle owns convergence into the adapter spec.

— Nyx Copperwing <EXECUTOR>
