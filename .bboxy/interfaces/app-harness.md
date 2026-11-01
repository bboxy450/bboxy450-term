---
id: 01M5WG2WRXB79K84R58QP93KT8
type: interface
number: 348
title: "ICD: app ↔ harness"
status: baselined
links:
  - { to: "[[interfaces]]", rel: index }
created: "2026-10-26T16:18:20Z"
updated: "2026-11-01T13:00:00Z"
---
# ICD: app ↔ harness

| | |
|---|---|
| Side A | the launcher and the harness kits (`bboxy450-term`, `bboxy450-harness-wake-up`, `bboxy450-harness-host`) |
| Side B | the native clients (Claude Code, Codex) and zsh, external |
| The agreement | [[w2-5-mcp-and-seat-wiring]]; [[harness-kits]] |
| Amended by | #262 roles; #265 trust; #270 the hook review recipe; #279 a runtime per seat; #4503 capability-file delivery (Side A PR for Oracle signature) |
| What crosses | our MCP server, hook shim and skill by launch arguments, environment and the per-seat runtime; shell integration for zsh. Admission crosses only as `BBOXY_CAPABILITY_FILE`, a regular 0600 euid-owned file in the 0700 per-launch ULID overlay, containing 64 hexadecimal bytes plus a newline. The value never crosses in any process environment; `BBOXY_CAPABILITY` is never set for any seat kind. |
| Timing | a launched seat is a starting pane in the next frame; the harness comes up asynchronously |
| States and lifecycle | Codex uses its installed CLI's shared daemon (operator, 2026-10-27: "respect the daemon" and wake-up "will work with shared server"); the CLI starts it idempotently if absent, and bboxy never restarts, stops or cleans it up. Bboxy owns only a per-seat connection proxy; it adds configuration to that TUI's first start/resume, registers the server-returned thread through the authenticated seat hook, and leaves unrelated threads alone. Native control pins the real daemon's kernel peer PID/start stamp and exact authenticated thread; shared status polling is read-only, and unknown/notLoaded refuses a wake without loading or choosing another thread. Older supervisor-owned private runtimes remain attachable. A seat restart is generation plus one, resumed by session id. The capability file has the overlay lifetime: close/process exit and generation replacement remove the old overlay; each generation has a fresh overlay, file and capability |
| Startup context (operator, 2026-10-27) | Claude append-system-prompt and Codex developer_instructions contain NAME, ROLE, host/cwd, and an instruction to call bboxy's orient MCP first. Codex's per-seat private wrapper supplies admission to MCP/hooks independently of the shared daemon's environment; the capability never enters CLI flags, nor any process environment. Bridge and hook consumers reopen the file on every start, reject symlinks/non-regular files, another uid, modes other than 0600 and malformed content with typed reasons, and never unlink it on read |
| Launch permissions (operator, 2026-10-28) | Every fresh/resumed Claude launch uses the installed CLI's `--dangerously-skip-permissions` flag (the operator requested bypass-permissions intent); local Codex launches use `--yolo`; managed remote launches carry the same policy through the proxy, compatible with native remote resume. The remote TUI receives configuration after permission selectors have been captured for the first RPC. Caller-supplied conflicting permission-mode/approval/sandbox selectors or policy overrides, and a `--` separator that would disable managed options, refuse before creating an overlay. Codex carries `approval_policy="never"` and `sandbox_mode="danger-full-access"` in that seat's first TUI start/resume config and typed request fields so remote/shared-daemon clients obey the same policy. This never changes global client/daemon configuration, unrelated threads, passive observation or wake requests; hook-trust bypass remains separately unrequested |
| Composed launchers (operator, 2026-10-28) | A shim preserves the caller-selected upstream endpoint, launch-policy intent and native conversation identity. An observing shim may chain a per-client connection proxy; the upstream daemon remains borrowed. Boxy's first start/resume overlay retains distinct incoming `hooks.<Event>` groups and appends its own, deduplicating exact groups; distinct MCP entries and shell environment settings survive the object merge. Other arrays retain seat-overlay precedence, and Boxy retains its cwd and permission policy. Native clients connect beside a live TUI; the authenticated server-returned thread binds the proxy across connections. A mismatched new client is refused without replacing that binding or closing the TUI. Later requests on a configured connection remain unchanged. Portable proxy regressions use mock launchers; `make test-harness-native` also exercises a forwarding shim in a private HOME. Optional installed shim integrations run through `make test-harness-fixture HARNESS_FIXTURE=/absolute/local-driver`; the executable receives the private build path. Third-party tools are not a test dependency; fixtures must preserve operator shims, services and accounts |
| Failure | the wake-up kit says whether a harness can be woken and names the fallback; native outcomes are never retried blindly |
| Wake causes (#60, live ICD201/301) | `WakeOutcome.cause` is an optional bounded machine-readable explanation, assigned by the producing branch; `note` keeps the human diagnostic and `evidence` remains the transport fact. Absent causes omit the field so legacy serialized outcomes retain their bytes. Missing/future/unrecognized causes never authorize input, retry, PTY fallback or model receipt. Native unknown remains non-retriable regardless of cause; journal replay preserves legacy/current typed rows and refuses changed retained facts. MCP consumers use typed causes rather than parsing notes. MCP capability version9 activates these logic-owned producers/consumers through the existing retained-logic refresh; it does not change UI control or signed envelope bytes. Private native/readiness/refusal, legacy-byte, replay and same-account advertised-version upgrade tests verify this addition; the upgrade fixture uses the same executable, not an actual older-binary claim. |
| Hook diagnostics (#56, Oracle cf405d78) | isolate inherited parent session IDs at Codex TUI/service exec; observe only its verified owner-PID, uniquely loaded thread via `thread/resume` with `excludeTurns:true` and no overrides, cancelled on binding/generation change; forward bounded `hook/completed` failures to the best-effort run journal, never hook input/context; unavailable stderr/exit/duration and async-hook coverage remain unknown, never inferred |
| Versioning | the app never writes the user's `~/.claude` or `~/.codex`; writes the client makes there are listed with cause; the hook-trust record is the one accepted app-caused write |
| Verified by | promises L3, L4, T6, T9; the native integration proof on installed clients. `make test-harness-native` includes the #4503 all-kind child scan of `ps -Eww` for the seat and every ancestor through its owned supervisor plus child `env`, asserting zero admission-value/key copies; authenticated orient/hook, file shape, typed refusals and generation/close lifetime. The environment reintroduction control fails that scan. |
| Open | a real model's choice to orient is advisory only, never a verdict |

## Phase 1a process seat amendment

Per [harness decision d4071](https://github.com/bboxy450/bboxy450-harness/blob/main/.bboxy/decisions/d4071-icd-348-amendment-signoff.md), the following additive rows are agreed in the gated Side A PR and signed off by the Oracle at landing. They are copied verbatim from [embed-seat §7 at 42e7a20](https://github.com/bboxy450/bboxy450-harness/blob/42e7a20/.bboxy/interfaces/embed-seat.md). The harness ICD stays proposed until that landing.

| #348 row | Change |
|---|---|
| Side B | add: `bboxy450-harness seat`, an in-suite line-mode harness (this ICD) |
| Amended by | add: `bboxy450-harness:[[embed-seat]]`, phase 1a process seat |
| What crosses | add: for `bboxy450-harness`, the MCP server by `--mcp-config` (the same `mcp.json`) and the startup context by `--append-system-prompt`. No hook shim, no skill/plugin dir |
| States and lifecycle | add: a `bboxy450-harness` seat is a PTY process with no daemon. State is `Cadence`, grade Inferred. Phase 1a: no automatic wake (a guarded input is refused as `composer-readiness-missing`, with zero PTY bytes), and a restart is a fresh session. Wake readiness, persisting the session id and resume are phase 1b, in a separate amendment |
| Startup context | add: `bboxy450-harness` receives the same `startup_prompt` text through `--append-system-prompt`. The capability never enters argv |
| Launch permissions | add: no permission flag is passed to `bboxy450-harness`. Its own `Policy` governs, and approvals are answered in line mode (§2) or fall to `on_unattended` (default deny) |
| Versioning | add: the app never writes `~/.bboxy450-harness`. The harness writes only its own sessions there |
| Verified by | add, phase 1a (E4): `make test-harness-native` `bboxy450-harness` case covering spawn, a PTY line that becomes a turn, a guarded `seat_input_text` refused as `composer-readiness-missing` with zero PTY bytes and no retry, and seat close under the supervisor's 250 ms SIGHUP→SIGKILL. On close, every group reaches ESRCH within 2 s, the exit is recorded, and a manual `--resume` recovers. Phase 1b (readiness, session-id persistence, resume) is **unproven** and is not claimed by this row |

A change to this interface names both sides, the reason and the verification it needs, and lands only with both sides' owners agreeing. A question an Executor asks about this boundary is a missing line here: the answer is written into this table, not only sent.

Owner: Corvin Duskmantle <ARCHITECT>.

Capability-file amendment (#4503, Side A): implements harness seat-capability-delivery §§1–5 under the Oracle's task #4505, following d4071; this PR presents the #348 rows for the Oracle's signature. This prevents passive exposure in process environments, not active same-uid reads of the path. Native daemon ownership and protocol authority remain unchanged.
