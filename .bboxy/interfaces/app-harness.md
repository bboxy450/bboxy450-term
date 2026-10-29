---
id: 01M5WG2WRXB79K84R58QP93KT8
type: interface
number: 348
title: "ICD: app ↔ harness"
status: baselined
links:
  - { to: "[[interfaces]]", rel: index }
created: "2026-10-26T16:18:20Z"
updated: "2026-10-29T01:18:52Z"
---
# ICD: app ↔ harness

| | |
|---|---|
| Side A | the launcher and the harness kits (`bboxy450-term`, `bboxy450-harness-wake-up`, `bboxy450-harness-host`) |
| Side B | the native clients (Claude Code, Codex) and zsh, external |
| The agreement | [[w2-5-mcp-and-seat-wiring]]; [[harness-kits]] |
| Amended by | #262 roles; #265 trust; #270 the hook review recipe; #279 a runtime per seat |
| What crosses | our MCP server, hook shim and skill by launch arguments, environment and the per-seat runtime; shell integration for zsh |
| Timing | a launched seat is a starting pane in the next frame; the harness comes up asynchronously |
| States and lifecycle | Codex uses its installed CLI's shared daemon (operator, 2026-10-27: "respect the daemon" and wake-up "will work with shared server"); the CLI starts it idempotently if absent, and bboxy never restarts, stops or cleans it up. Bboxy owns only a per-seat connection proxy; it adds configuration to that TUI's first start/resume, registers the server-returned thread through the authenticated seat hook, and leaves unrelated threads alone. Native control pins the real daemon's kernel peer PID/start stamp and exact authenticated thread; shared status polling is read-only, and unknown/notLoaded refuses a wake without loading or choosing another thread. Older supervisor-owned private runtimes remain attachable. A seat restart is generation plus one, resumed by session id |
| Startup context (operator, 2026-10-27) | Claude append-system-prompt and Codex developer_instructions contain NAME, ROLE, host/cwd, and an instruction to call bboxy's orient MCP first. Codex's per-seat private wrapper supplies admission to MCP/hooks independently of the shared daemon's environment; the capability never enters CLI flags |
| Launch permissions (operator, 2026-10-28) | Every fresh/resumed Claude launch uses the installed CLI's `--dangerously-skip-permissions` flag (the operator requested bypass-permissions intent); local Codex launches use `--yolo`; managed remote launches carry the same policy through the proxy, compatible with native remote resume. The remote TUI receives configuration after permission selectors have been captured for the first RPC. Caller-supplied conflicting permission-mode/approval/sandbox selectors or policy overrides, and a `--` separator that would disable managed options, refuse before creating an overlay. Codex carries `approval_policy="never"` and `sandbox_mode="danger-full-access"` in that seat's first TUI start/resume config and typed request fields so remote/shared-daemon clients obey the same policy. This never changes global client/daemon configuration, unrelated threads, passive observation or wake requests; hook-trust bypass remains separately unrequested |
| Composed launchers (operator, 2026-10-28) | A shim preserves the caller-selected upstream endpoint, launch-policy intent and native conversation identity. An observing shim may chain a per-client connection proxy; the upstream daemon remains borrowed. Boxy's first start/resume overlay retains distinct incoming `hooks.<Event>` groups and appends its own, deduplicating exact groups; distinct MCP entries and shell environment settings survive the object merge. Other arrays retain seat-overlay precedence, and Boxy retains its cwd and permission policy. Native clients connect beside a live TUI; the authenticated server-returned thread binds the proxy across connections. A mismatched new client is refused without replacing that binding or closing the TUI. Later requests on a configured connection remain unchanged. Portable proxy regressions use mock launchers; `make test-harness-native` also exercises a forwarding shim in a private HOME. Optional installed shim integrations run through `make test-harness-fixture HARNESS_FIXTURE=/absolute/local-driver`; the executable receives the private build path. Third-party tools are not a test dependency; fixtures must preserve operator shims, services and accounts |
| Failure | the wake-up kit says whether a harness can be woken and names the fallback; native outcomes are never retried blindly |
| Hook diagnostics (#56, Oracle cf405d78) | isolate inherited parent session IDs at Codex TUI/service exec; observe only its verified owner-PID, uniquely loaded thread via `thread/resume` with `excludeTurns:true` and no overrides, cancelled on binding/generation change; forward bounded `hook/completed` failures to the best-effort run journal, never hook input/context; unavailable stderr/exit/duration and async-hook coverage remain unknown, never inferred |
| Versioning | the app never writes the user's `~/.claude` or `~/.codex`; writes the client makes there are listed with cause; the hook-trust record is the one accepted app-caused write |
| Verified by | promises L3, L4, T6, T9; the native integration proof on installed clients |
| Open | a real model's choice to orient is advisory only, never a verdict |

A change to this interface names both sides, the reason and the verification it needs, and lands only with both sides' owners agreeing. A question an Executor asks about this boundary is a missing line here: the answer is written into this table, not only sent.

Owner: Corvin Duskmantle <ARCHITECT>.
