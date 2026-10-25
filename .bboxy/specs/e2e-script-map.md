---
id: 01M5R9D3Y7S6NATG9B47HD5YZA
type: spec
number: 317
title: "e2e script map: one row per promise, the night script, its coverage, the missing assertions and the task tracking them; agrees with the scripts' coverage headers"
status: draft
links:
  - { to: "[[e2e-acceptance]]", rel: builds-on }
  - { to: "[[d316-pass-means-the-whole-promise]]", rel: decision }
created: "2026-10-25T01:04:37Z"
updated: "2026-10-25T04:25:15Z"
---
# e2e script map

**Lens.** Each promise of [[e2e-acceptance]] §1 has one night script, `scripts/e2e/<ID>.sh`. Decision [[d316-pass-means-the-whole-promise]]: its prefix-matched `# coverage:` header is the source of current coverage; only `full` is green. Whoever changes a header updates its row in the same commit.

The coverage index below reads script presence and headers at `114ce13d897384d6f83689febd783d73426def5f`. There are 55 current promises, including T9. A present script without a coverage header is `partial` with `unstated` missing assertions; an absent script has its whole promise missing. Blank `tracked-by` means no open task number was identified, so the partial is untracked/red, never an implied waiver. This record creates no tasks and claims no script execution.

The detailed plans preserve all 54 rows from Aster’s six-part ICQ map. Their code baseline is `afff4758`, their spec baseline is `e48d6f8f`; their steps, assertions, missing hooks and source lines are a dated plan, not a claim about current implementation. Some hooks have since landed. Current coverage comes from the index, not from that historical gap list. T9 was added later and is indexed from its script; it is outside the archived 54-row plan.

| ID | script | coverage | missing | tracked-by |
|---|---|---|---|---|
| T1 | `scripts/e2e/T1.sh` | partial | unstated (no coverage header) | |
| T2 | `scripts/e2e/T2.sh` | partial | unstated (no coverage header) | |
| T3 | `scripts/e2e/T3.sh` | partial | unstated (no coverage header) | |
| T4 | `scripts/e2e/T4.sh` | partial | unstated (no coverage header) | |
| T5 | `scripts/e2e/T5.sh` | partial | unstated (no coverage header) | |
| T6 | `scripts/e2e/T6.sh` | partial | unstated (no coverage header) | |
| T7 | `scripts/e2e/T7.sh` | partial | solved pane rectangle and focus after undo; visible grace strip | |
| T8 | `scripts/e2e/T8.sh` | partial | unstated (no coverage header) | |
| T9 | `scripts/e2e/T9.sh` | partial | unstated (no coverage header) | |
| L1 | `scripts/e2e/L1.sh` | partial | script not implemented: ⌘T opens the centred launcher over the blurred window with harness rows, search, and role presets; Esc closes it with no spawn | |
| L2 | `scripts/e2e/L2.sh` | partial | script not implemented: a new seat lands exactly where the operator picked (split right, split down, new tab, new space), never a guessed slot | |
| L3 | `scripts/e2e/L3.sh` | partial | script not implemented: every launched seat has our MCP server, the hook shim and the bboxy450 skill at launch; the user's own ~/.claude and ~/.codex are byte-identical before and after | |
| L4 | `scripts/e2e/L4.sh` | partial | script not implemented: a fresh Claude seat and a fresh Codex seat each call orient and task_report on their first turn in a fixture task | |
| S1 | `scripts/e2e/S1.sh` | partial | script not implemented: scrolling up shows earlier output byte-identical to the ring, paged lazily; the top row of a 20k-line fixture is line 1; the join with live rows is seamless | |
| S2 | `scripts/e2e/S2.sh` | partial | script not implemented: a gap in the ring shows the gap marker at the right row, never a silent splice | |
| S3 | `scripts/e2e/S3.sh` | partial | script not implemented: 50 watched seats scrolled to the top stay under the retained-heap budget; paging shows no hitch on the meter | |
| S4 | `scripts/e2e/S4.sh` | partial | script not implemented: a pane whose dimensions cannot fit three viewports in the budget refuses history by name, and the live pane is unaffected | |
| K1 | `scripts/e2e/K1.sh` | partial | script not implemented: every key combination reaches the program under the kitty protocol when the program asks for it; ⌘⌥ navigation stays the app's; plain keys are the terminal's | |
| K2 | `scripts/e2e/K2.sh` | partial | script not implemented: ⌘M mutes the focused seat: its row stays, the amber muted mark shows, it works one to one with the operator | |
| K3 | `scripts/e2e/K3.sh` | partial | script not implemented: ⌘G opens the Graph tab; ⌘-click on a .md path or a wikilink opens the document modal | |
| R1 | `scripts/e2e/R1.sh` | partial | script not implemented: every pane and strip carries the alive rim centred on the border, two counter-moving waves, seat colour, pulse by activity, focused ×1.3; text is never covered | |
| R2 | `scripts/e2e/R2.sh` | partial | script not implemented: Settings › Rim size 2 to 8 moves only the gutter and the stage inset; at 2, 4 and 8 the gaps are 2, 4, 8 and the insets 1, 2, 4 | |
| R3 | `scripts/e2e/R3.sh` | partial | script not implemented: focusing a block swells its silk 40 px past the rim and settles in 0.9 s; nothing moves; the old locate ripple is gone | |
| R4 | `scripts/e2e/R4.sh` | partial | script not implemented: seat rows, tabs and cards carry the rim and the swell out of the box; idle rows show no rim at rest | |
| R5 | `scripts/e2e/R5.sh` | partial | script not implemented: a seat waiting on the operator loops the swell in amber on its row and pane until handled; focusing it stops the loop with one swell | |
| R6 | `scripts/e2e/R6.sh` | partial | script not implemented: a waiting seat posts one desktop notification with the question's first line; a repeat replaces it; clicking it raises the window, switches space and tab, focuses the pane; muted seats post nothing | |
| M1 | `scripts/e2e/M1.sh` | partial | script not implemented: a DM, a channel post and a swarm-stream post from the composer each arrive at the right seats with a receipt; the operator principal signs as the device | |
| M2 | `scripts/e2e/M2.sh` | partial | script not implemented: the role chip fans out to the exact seats holding the role, one receipt each, shown as one send; above 32 seats it refuses with a named reason | |
| M3 | `scripts/e2e/M3.sh` | partial | script not implemented: the operator delegates a task by #N or path to a seat; the seat's frontmatter says claimed and its row shows the task; the seat gets one oracle-class message | |
| M4 | `scripts/e2e/M4.sh` | partial | script not implemented: a report with verify_result whose command differs from verify, or whose exit code is non-zero, is refused on accept with the reason named | |
| M5 | `scripts/e2e/M5.sh` | partial | script not implemented: the operator alone lists open unassigned tasks account-wide; a seat asking gets the existing refusal | |
| M6 | `scripts/e2e/M6.sh` | partial | script not implemented: groups stay their own destination; a message to a muted seat waits; priority delays, never deletes | |
| F1 | `scripts/e2e/F1.sh` | partial | script not implemented: the stream dock shows one swarm at a time with its channels; arrivals appear within one event-lane delivery; nothing polls | |
| F2 | `scripts/e2e/F2.sh` | partial | script not implemented: unread counts come from the operator's read marks and survive a restart; viewing a stream marks it read | |
| F3 | `scripts/e2e/F3.sh` | partial | script not implemented: killing logic and restarting resumes the events lane at its cursors: no duplicate row, no lost event | |
| F4 | `scripts/e2e/F4.sh` | partial | script not implemented: a filter change while attached migrates the consumer: pending rows for still-granted subjects are delivered, revoked ones dropped unprojected, one audit line on account/control | |
| G1 | `scripts/e2e/G1.sh` | partial | script not implemented: Settings › Appearance: Rim size 2 to 8 default 4; depth is no longer a setting | |
| G2 | `scripts/e2e/G2.sh` | partial | script not implemented: Settings › Notifications: on by default, per-harness off, only-when-not-frontmost | |
| G3 | `scripts/e2e/G3.sh` | partial | script not implemented: Settings › Plugins lists every discovered plugin with trust state, tier, shape, health and the call-and-hook timeline; a 10 s hook shows the warning | |
| G4 | `scripts/e2e/G4.sh` | partial | script not implemented: Settings › Knowledge lists roots with their provider; swapping to a stub provider and back moves no file | |
| G5 | `scripts/e2e/G5.sh` | partial | script not implemented: theme is data: a token change in design/tokens.toml changes the look without a code change | |
| D1 | `scripts/e2e/D1.sh` | partial | script not implemented: a .md opens in the modal rendered natively: headings, lists, tables, code, task lists, footnotes; true size, never scaled | |
| D2 | `scripts/e2e/D2.sh` | partial | script not implemented: a mermaid fence renders through our engine: no overlaps, edges on borders, deterministic | |
| D3 | `scripts/e2e/D3.sh` | partial | script not implemented: `[[slug]]`, `[[#N]]` and `[[slug\|alias]]` resolve; a bad link shows as broken, never silently skipped; ![[bboxy.png]] renders the image from .bboxy/images | |
| D4 | `scripts/e2e/D4.sh` | partial | script not implemented: the modal opens, scrolls, copies, closes, and feeds the document into the focused harness | |
| N1 | `scripts/e2e/N1.sh` | partial | script not implemented: every record has a unique number; a new record created through the app takes the next free one; a duplicate or invalid number is refused and shown as broken | |
| N2 | `scripts/e2e/N2.sh` | partial | script not implemented: every decision is its own node; the Graph tab shows a decision with its decides, constrains, evidence, answers, amends edges | |
| N3 | `scripts/e2e/N3.sh` | partial | script not implemented: a comment pinned to a digest shows beside the version it was made on; the RFC loop runs in the app: proposal, comments, decision, spec | |
| N4 | `scripts/e2e/N4.sh` | partial | script not implemented: the Graph tab shows agents (seat, role, now, blocked on), the work tree, the provenance card and what each seat listens to | |
| I1 | `scripts/e2e/I1.sh` | partial | script not implemented: first launch enrols one device key under the GPG root with exactly one custody prompt request, later launches request none; the night proves it against an isolated non-operator native-custody test backend that counts enrollments and prompt requests, never the login Keychain, whose real first launch is the operator's own and is judged, not scripted | |
| I2 | `scripts/e2e/I2.sh` | partial | script not implemented: every seat has its own generation key certified by the device key; every message envelope verifies; a seat with a nil seat id and a non-operator role is refused | |
| I3 | `scripts/e2e/I3.sh` | partial | script not implemented: the local-dev issuer path works offline and is marked as such everywhere it shows; pinned receivers refuse local-dev envelopes | |
| I4 | `scripts/e2e/I4.sh` | partial | script not implemented: every record write from the app is signed by the device key with signer device:<id>; git log shows only good signatures on main | |
| P1 | `scripts/e2e/P1.sh` | partial | script not implemented: the hitch meter shows zero hitches idle at 120 Hz with 1000 sidebar rows and 50 watched panes once R5g lands; the HUD is in our look | |
| P2 | `scripts/e2e/P2.sh` | partial | script not implemented: an Oracle broadcast reaches 1000 seats under the bus budget; 400 idle seats stay under the memory budget | |

## Deterministic plans: original 54 promises

Every plan uses its own private mode-0700 account/run/repository, pinned binary/source, and headless app with file custody. Retain JSONL requests/replies, exact exits, IDs/generations/op IDs, record and bounded journal reads, screen/capture metadata and hashes. Use predicate waits and admitted orderly stop; never OS input or an operator credential. A seat PTY key is not an app UI key. A capture frame number alone does not bind an input operation to a frame, and offscreen GPU completion is not display presentation. Proposed hooks below remain labelled as proposals at their recorded baseline.

A bounded journal probe, where proposed, opens only the test account’s SQLite store with `mode=ro` and `query_only=ON`, reads an identified `control/op/<op>` row, and checks actor/tool/argument digest/result without dumping identity or arbitrary keys. A transient control/audit outbox row disappearing is not proof that no audit exists; the durable retained event is the receipt. Never open a live Ring directly. Real-agent checks need their own provisioned harness account; an assistant self-report is not a verdict.

Baseline source shorthand: app filenames are under `crates/bboxy450-term/src/`; `gui.rs` and `seat_state.rs` under `crates/bbt-proto/src/`; `logic.rs`, `mcp/*`, `store/*` under `crates/bbt-mux/src/`. The original row text and all baseline pointers below are preserved from the archived map.

### T1

**Promise:** ⌘T on a Claude, Codex or zsh row opens a real PTY seat in a pane within one frame, named from the furry pool, in the chosen placement.

**Deterministic steps (plan):** Record seat ids; for H=claude,codex,zsh run `ctl seat spawn H --place tab --cwd "$E2E_WORK"` without --name, retain data.seat/op; use screen/wait and a TTY-reporting fixture in zsh; capture the pane. Separately invoke the launcher's Cmd-T/row/Enter path once the UI hook exists.

**Required assertions:** One new running seat per op, real PTY, unique name selected by seat_names::POOL plus its numbered tiers; requested tab contains that seat. First frame containing it is <=1 GUI frame after the launch action. Explicit --name remains exact.

**Gaps at baseline afff4758:** ui key/open/row selection/layout; input-or-spawn-op -> composed frame/model correlation. Existing script T1 only tests zsh tab/name/process, not the whole promise or one-frame bound.

**Baseline pointers:** ctl.rs:251,505; logic.rs:47; scripts/e2e/T1.sh:5.

### T2

**Promise:** typing in a focused pane reaches the program; the echo appears within one frame; a 100 kB paste arrives intact.

**Deterministic steps (plan):** `ctl seat focus S`; type a nonce plus Enter, wait for a program-generated echo; capture. Paste exactly100000 known bytes into a raw-mode receiver that first prints READY, consumes an exact count and emits length/SHA256; compare the entire received file, not a prefix. Repeat with bracketed-paste enabled and verify one start/end pair.

**Required assertions:** Exact echoed nonce and SHA256/length100000, no extra byte, reordering or duplicate; the frame showing the echo is at most one GUI frame after the admitted input. Focused UI keyboard event takes the same path.

**Gaps at baseline afff4758:** UI key routing and op->frame correlation; deterministic receiver helper/READY barrier. ctl acknowledgements and screen waits alone cannot prove frame latency. Existing T2.sh sleep0.5 and truncating one file before compare are weaker than this assertion.

**Baseline pointers:** ctl.rs:1920,1965; scripts/e2e/T2.sh:11; gui.rs:253.

### T3

**Promise:** resizing a pane or the window resizes the PTY and the program redraws at the new size with no stale cells; under a window owner the layout owns the PTY size, so the night drives it with `ctl app resize <cols>x<rows>` and asserts `stty` in the seat equals the pane size `seat ls` reports.

**Deterministic steps (plan):** In a one-pane owned GUI run start a SIGWINCH fixture that fills every row with a size/epoch pattern. Per new spec e48d6f8f, test seat resize refusal under a GUI owner, then use the specified but UNIMPLEMENTED `ctl app resize 100x30` and72x20. Wait for layout convergence; compare stty with the pane size reported by seat ls, Screen and capture. Test bare Session resize separately.

**Required assertions:** Under window/headless GUI ownership seat resize returns layout_owned with app resize hint. App resize changes the actual layout/PTY; stty, solved pane grid and Screen dimensions agree, final epoch fills it with no stale cells. Bare Session resizing is a different contract.

**Gaps at baseline afff4758:** app resize, layout_owned refusal and pane cols/rows in seat ls are newly specified at e48d6f8f but not in the28-verb parser/GuiSeat. Existing direct seat resize can be overwritten by GUI desired size; it is not the T3 proof.

**Baseline pointers:** ctl.rs:251,1200; gui SeatScreen:270; ctl-cli.md:41,46 at e48d6f8f; scripts/e2e/T3.sh:4.

### T4

**Promise:** a pane shows real cells: colours, bold, italics, underline, wide glyphs and emoji align to the grid; the cursor shape follows the program.

**Deterministic steps (plan):** Run the pinned terminal/firehose byte fixture through a real zsh PTY; `seat screen S --cells`, capture S at light/dark and1x/2x offscreen sizes; compare against pinned parser cell hashes and approved per-layer image fixtures. Emit DECSCUSR block/beam/underline, read cursor_style and capture each.

**Required assertions:** Exact parser cells/colours/flags/cursor and correct wide/ZWJ continuation positions; approved native image metric at true scale. Terminal grid equality and image fidelity are separate assertions.

**Gaps at baseline afff4758:** Full cell column/width/continuation or Screen hash read: current StyledRun has text/style but no widths. Deterministic theme/scale/capture-state control for the full matrix; current --cells text equality alone cannot prove grid alignment.

**Baseline pointers:** ctl.rs:1172; gui.rs:270,303; scripts/e2e/T4.sh:7; T2 terminal proof fixtures.

### T5

**Promise:** the app is killed and relaunched: every seat is attached to the same PTY, layout, focus and scroll positions are identical, the programs never noticed.

**Deterministic steps (plan):** Start a dedicated app with two independently advancing process counters; save seat ids,generations,pids,PTY names,layout,focus and scroll positions. Crash only the GUI app owner, relaunch/adopt the existing backend, then type a nonce into each and compare state.

**Required assertions:** Same process PID/PTY/generation and monotonic counters with no restart marker; identical layout/focus/scroll. Stop/start is deliberately not used: app stop destroys the backend by contract.

**Gaps at baseline afff4758:** Controlled app-crash/relaunch/adoption plus UI-state read/persistence. U1 starts a new Session; adoption is explicitly separate. Existing T5.sh kills logic, not the app, so it must not pass T5.

**Baseline pointers:** scripts/e2e/T5.sh:8; ctl/app.rs:97; ctl-cli.md:67.

### T6

**Promise:** the supervisor dies: seats survive; the app says so, restarts it, re-attaches, resumes by session id.

**Deterministic steps (plan):** Dedicated run: spawn zsh plus real resume-capable Claude/Codex fixture seats; record seat ids,generations,session ids and screens. Fault only its owned supervisor, observe restart/reconnect, wait for a new program marker, read records and capture restart/gap UI.

**Required assertions:** Seat ids retained, generation increments, zsh has a new PID as specified; harness resumes the same session id and prior conversation. Restart/gap notice visible; pre-restart queued keystrokes never enter the new generation.

**Gaps at baseline afff4758:** Deterministic owned-process fault/status hook; harness resume-session and UI notice read. Existing T6.sh proves only zsh respawn, not same PTY survival or real harness resume. 'Seats survive' must not be reported as 'old shell PID survives'.

**Baseline pointers:** scripts/e2e/T6.sh:2,14; c1-live-seats.md:212; ctl.rs:1651.

### T7

**Promise:** closing a pane shows the grace strip for 60 s, ⌘Z brings it back with the PTY and its position; after grace it is gone and says so: the pane and its sidebar row leave, one stream line reads "<name> closed; its history is kept" (nothing is deleted, the ring and the records stay reachable from the seat's record and the Graph tab, [[mantras]] 22), and ⌘Z is no longer offered for it (ruling 2026-10-24; not implemented in the window as of this date, Velvet builds it with Wren's reference).

**Deterministic steps (plan):** Spawn two seats, record exact placement and focused position. Close one through the GUI relay; at grace+59s undo using the same app path and compare PID/layout. Close again, advance past60s, read seat list, stream, history and record; attempt undo.

**Required assertions:** During grace strip/row present; undo restores the same PTY and Position. After expiry pane+row absent, exactly one '<name> closed; its history is kept' event, retained ring/record reachable, no undo offer. No deletion.

**Gaps at baseline afff4758:** seat close/undo-close and UI Cmd-Z relay, controllable grace clock, layout/scroll/stream reads. T7 is Velvet's in-flight work; do not substitute a process exit or app stop.

**Baseline pointers:** ctl.rs:251 (verbs absent); ctl-cli.md:60; e2e-acceptance.md:T7.

### T8

**Promise:** OSC 7501 state from a harness shows as reported grade; a harness that stays silent shows idle, never an invented state.

**Deterministic steps (plan):** Spawn silent and reporting fixture seats. Read seat ls/watch; silent fixture only prints ordinary output, reporting fixture emits approved OSC7501 state transitions from its own PTY. Wait by observed state, then read state.activity,grade and capture row.

**Required assertions:** Reporting transitions have grade reported and exactly the requested activity; silence stays idle with no invented active state. Reject malformed/unapproved report cases and retain their named reason.

**Gaps at baseline afff4758:** No new ctl verb needed for state/grade checks; use a valid admitted OSC fixture. UI row grade/cell projection needs UI-state/capture assertion. Existing T8.sh does not assert reported grade.

**Baseline pointers:** ctl.rs:1665; seat_state.rs:16; scripts/e2e/T8.sh:9.

### L1

**Promise:** ⌘T opens the centred launcher over the blurred window with harness rows, search, and role presets; Esc closes it with no spawn.

**Deterministic steps (plan):** Record seat set; use app-owned UI key Cmd-T, inspect launcher model/capture; type a fixed search, select Claude role presets, then Esc. Reopen from the app's own shortcut, repeat with each role and empty query.

**Required assertions:** Centered launcher bounds and approved blur image, expected harness/preset rows, deterministic search results; Esc removes overlay and leaves seat set/control journal with zero Spawn effects.

**Gaps at baseline afff4758:** ui open/key/query/select/layout and read-only launcher projection. seat key sends PTY bytes and cannot stand in for Cmd-T/Esc on app chrome.

**Baseline pointers:** ctl.rs:251; ctl-cli.md:47; launcher.rs.

### L2

**Promise:** a new seat lands exactly where the operator picked (split right, split down, new tab, new space), never a guessed slot.

**Deterministic steps (plan):** For each right/down/tab/space: focus anchor A; choose placement through launcher, capture target and revision, then change focus to B before the correlated spawn completes. Read layout and capture A/B/new seat.

**Required assertions:** New seat uses the captured anchor/generation and exact selected side/tab/space; sibling ratios and prior panes preserved, no placement resolved from later focus.

**Gaps at baseline afff4758:** Parser accepts only --place tab (other values explicitly refused); right/down/space controls, captured-placement injection/relay and solved layout dump are missing. Space support must be observed, not guessed from protocol.

**Baseline pointers:** ctl.rs:513; gui Placement in seat_state.rs:64; ctl-cli.md:40.

### L3

**Promise:** every launched seat has our MCP server, the hook shim and the bboxy450 skill at launch; the user's own ~/.claude and ~/.codex are byte-identical before and after.

**Deterministic steps (plan):** On a dedicated account/home fixture, hash its harness configuration before/after; launch each supported harness through seat spawn, retaining launch op. Have a deterministic probe read argv/owned overlay, MCP handshake, hook paths and installed skill digest; inspect the per-seat launch artifact and real MCP bridge audit.

**Required assertions:** bboxy MCP server, expected hook set and pinned skill are available on first launch; installed paths/argv belong to this seat; before/after user configuration digest sets identical. No operator credential/content is read or changed.

**Gaps at baseline afff4758:** Read-only launch manifest/MCP-call audit export and provisioned throwaway harness account. Fixture process/argv proves wiring only; a real client handshake is a separate assertion. recipe control is not yet in the ctl parser.

**Baseline pointers:** ctl.rs:505; adapters launch path; ctl-cli.md:80; tests/mcp.rs.

### L4

**Promise:** a fresh Claude seat and a fresh Codex seat each call orient and task_report on their first turn in a fixture task.

**Deterministic steps (plan):** `task create 'L4-H' --verify 'sh scripts/e2e/L4.sh'`, spawn a fresh H=claude,codex, delegate by #N, send a deterministic first-turn instruction, run prompt recipes if matched, wait for its report. Read the real bridge calls and task document rather than treating the assistant's text as evidence.

**Required assertions:** For each harness, first model turn includes successful orient then valid task_report for its assigned task; recorded scope seat/generation matches launch, verify_result.command exactly matches task.verify, exit_code0. Script independently checks effects.

**Gaps at baseline afff4758:** recipe run/detect, read-only bridge/turn journal, task report/status collection exposed to ctl; own harness login provisioning. No self-report can supply the missing first-turn/call-order evidence.

**Baseline pointers:** ctl.rs:1817; proto/messaging.rs:480; mcp/journal.rs:120; e2e-acceptance.md:4a.

### S1

**Promise:** scrolling up shows earlier output byte-identical to the ring, paged lazily; the top row of a 20k-line fixture is line 1; the join with live rows is seamless.

**Deterministic steps (plan):** Run numbered lines00001..20000 ending in an explicit DONE marker; save ring/capture sequence. Scroll the GUI pane to top, page until settled, read displayed rows/capture, then scroll through the history/live boundary.

**Required assertions:** Top is00001; each row appears exactly once in order with exact bytes from the corresponding ring records; seamless join and bounded on-demand pages, no full eager materialization.

**Gaps at baseline afff4758:** GUI scroll/seek and historical displayed-screen state/page counters. seat screen --scrollback FROM..TO replays a range in the ctl process, not the GUI Playback; it cannot prove lazy pane paging.

**Baseline pointers:** ctl.rs:1180; scrollback.rs:192; c1-live-seats.md:189.

### S2

**Promise:** a gap in the ring shows the gap marker at the right row, never a silent splice.

**Deterministic steps (plan):** Seed an owned ring fixture with a known bounded missing sequence interval surrounded by numbered rows; scroll the real pane across it. Read ordered ring History pages and the displayed model/capture.

**Required assertions:** Exactly one gap marker for the supplied range at the boundary row, with both surviving sides intact and no silent splice or duplicate. The ring query itself must report gap/next and its time.

**Gaps at baseline afff4758:** Deterministic ring-gap fixture/fault hook plus GUI scroll/displayed history read. Never open a live Ring directly: open() can repair/mutate it.

**Baseline pointers:** scrollback.rs; ctl.rs:1180; c1-live-seats.md:189.

### S3

**Promise:** 50 watched seats scrolled to the top stay under the retained-heap budget; paging shows no hitch on the meter.

**Deterministic steps (plan):** Spawn50 owned real PTYs, produce fixed20k-line histories, scroll every watched pane to top; retain page/heap metrics during bounded seeks and captures, then return all panes to live.

**Required assertions:** Each Playback retained_heap_bytes <=4000000 and total <=200000000; no missing row, paging work off UI, meter within the written budget. Returning live releases cached history pages.

**Gaps at baseline afff4758:** Watched-set/layout control for50 panes, scroll seek and per-Playback retained heap+frame meter export. A ctl range replay or RSS alone is not the history-heap measurement.

**Baseline pointers:** scrollback.rs:14,244; c1-live-seats.md:189; ctl.rs:251.

### S4

**Promise:** a pane whose dimensions cannot fit three viewports in the budget refuses history by name, and the live pane is unaffected.

**Deterministic steps (plan):** Choose cols/rows whose three viewports exceed the actual heap admission arithmetic; resize and verify live nonce first; request GUI history, inspect named refusal; type a second live nonce and capture.

**Required assertions:** history_viewport_exceeds_budget with valid dimensions; no panic or false 'dimensions unsupported'; generation/live grid still updates and new nonce visible. History allocations remain within cap.

**Gaps at baseline afff4758:** GUI history admission/scroll/refusal read and dimension-aware test setup; ctl --scrollback does not exercise this refusal. Use core-valid dimensions and checked arithmetic.

**Baseline pointers:** scrollback.rs:194,207; ctl.rs:1180; c1-live-seats.md:189.

### K1

**Promise:** every key combination reaches the program under the kitty protocol when the program asks for it; ⌘⌥ navigation stays the app's; plain keys are the terminal's.

**Deterministic steps (plan):** Run a raw Kitty negotiation/decoder fixture. Table-drive every encoder-supported key x modifiers x press/repeat/release via app keyboard-event seam; compare received bytes to pinned expected encodings. Exercise Cmd-Opt navigation and plain terminal keys while capturing active focus.

**Required assertions:** Exact protocol bytes for requested Kitty modes, no app navigation bytes in PTY, plain keys arrive once; press/release suppression and IME/composition remain distinct.

**Gaps at baseline afff4758:** Structured UI key event (physical/logical/modifiers/repeat/release), focus read. Current named seat key covers a subset, always synthetic press; it cannot prove all combos or app shortcut routing.

**Baseline pointers:** keyboard.rs:15,190,262; ctl.rs:2058.

### K2

**Promise:** ⌘M mutes the focused seat: its row stays, the amber muted mark shows, it works one to one with the operator.

**Deterministic steps (plan):** Focus A; send app Cmd-M; inspect row/capture and muted state, send ordinary non-operator message to A and operator DM via message send. Unmute and inspect the original pending message's receipts.

**Required assertions:** Row retained with amber muted mark; operator1:1 still arrives, non-operator pending is delayed and later delivered exactly once. Mute action is journaled under the correct device.

**Gaps at baseline afff4758:** ui key Cmd-M/mute relay and row/receipt projection; current ctl has no mute verb although messaging Request::Mute exists.

**Baseline pointers:** proto/messaging.rs:444; ctl.rs:251; messaging.rs.

### K3

**Promise:** ⌘G opens the Graph tab; ⌘-click on a .md path or a wikilink opens the document modal.

**Deterministic steps (plan):** From a pane showing a fixture.md path and `[[#N]]`, issue app Cmd-G and app-owned Cmd-click on each semantic text target, then capture/read active tab/modal and resolved record.

**Required assertions:** Graph tab selected; each click opens the intended document and digest, not a shell command; focus/terminal input remain unchanged when dismissing modal.

**Gaps at baseline afff4758:** ui key/open plus semantic pointer/link activation and active-tab/document state; PTY seat key cannot generate chrome Cmd-G or Cmd-click.

**Baseline pointers:** ctl-cli.md:47; ctl.rs:251; main.rs app routing.

### R1

**Promise:** every pane and strip carries the alive rim centred on the border, two counter-moving waves, seat colour, pulse by activity, focused ×1.3; text is never covered.

**Deterministic steps (plan):** Set deterministic seat colours/activity/focus, freeze animation at t0,t1,t2; capture full pane and strip at1x/2x and compare geometry/pixels against approved rim fixtures.

**Required assertions:** Rim straddles border; two waves move in opposite directions; colour/activity pulse and focused gain1.3 match reference; terminal text mask never intersects rim ink.

**Gaps at baseline afff4758:** Read/set deterministic UI fixture time, activity, focus and paint geometry; image comparator with approved reference. Captures alone with wall-clock sleeps cannot prove wave direction.

**Baseline pointers:** rim-glow.md; ctl.rs:251; gui.rs:253.

### R2

**Promise:** Settings › Rim size 2 to 8 moves only the gutter and the stage inset; at 2, 4 and 8 the gaps are 2, 4, 8 and the insets 1, 2, 4.

**Deterministic steps (plan):** Open Appearance via own UI; choose rim size2,4,8 in turn. Dump solved outer/pane/body bounds and capture each with the same tree/window dimensions.

**Required assertions:** Gap2/4/8 logical px, stage inset1/2/4; pane order and everything outside the ruled inset/gutter effect unchanged. Persisted setting reopens with same value.

**Gaps at baseline afff4758:** UI settings action, settings read and solved layout dump; current seat resize changes PTY, not rim size.

**Baseline pointers:** ctl-cli.md:47; settings.rs; e2e-acceptance.md:R2.

### R3

**Promise:** focusing a block swells its silk 40 px past the rim and settles in 0.9 s; nothing moves; the old locate ripple is gone.

**Deterministic steps (plan):** Capture identical geometry before focus; focus a new block, advance injected animation time through0,peak,900ms and after; capture each and inspect solved bounds.

**Required assertions:** Silk reaches40logicalpx past the rim, returns to settled by0.9s, layout bounds byte-identical throughout; no locate-ripple paint node/pixels.

**Gaps at baseline afff4758:** Deterministic animation clock, UI paint/layout read. seat focus exists but callback/capture has no guaranteed applied-focus frame fence before T7 correlation lands.

**Baseline pointers:** ctl.rs:1665; gui.rs:253; rim-glow.md.

### R4

**Promise:** seat rows, tabs and cards carry the rim and the swell out of the box; idle rows show no rim at rest.

**Deterministic steps (plan):** Materialize seat row/tab/card for active and idle seats; set known focus and animation times, capture each native composition and inspect paint state.

**Required assertions:** Shared rim/swell behaviour on all three surfaces; idle unselected row has zero rim at rest; no extra layout shift or per-surface ad-hoc renderer.

**Gaps at baseline afff4758:** UI surface selection/projection/time hooks; static bboxybook/parity fixtures support pixels but do not alone prove the live row/tab/card state producer.

**Baseline pointers:** ctl.rs:251; rim-glow.md; bbt-ui seat_row/strip/card.

### R5

**Promise:** a seat waiting on the operator loops the swell in amber on its row and pane until handled; focusing it stops the loop with one swell.

**Deterministic steps (plan):** Reporting fixture enters waiting with a known question; wait --state waiting, capture row+pane at two loop phases; focus it and advance time through one swell, capture again.

**Required assertions:** Both surfaces loop amber while waiting; focus ends the loop and emits exactly one final swell, no duplicate notification/focus transition.

**Gaps at baseline afff4758:** Deterministic UI animation and attention-state counters; capture-fenced focus. seat wait proves activity only, not looping visuals.

**Baseline pointers:** ctl.rs:1665; attention.rs; e2e-acceptance.md:R5.

### R6

**Promise:** a waiting seat posts one desktop notification with the question's first line; a repeat replaces it; clicking it raises the window, switches space and tab, focuses the pane; muted seats post nothing.

**Deterministic steps (plan):** In a private automation session attach a recording notification backend. Emit waiting question, repeat with changed first line, invoke its recorded activation callback; repeat with muted seat and with frontmost/background policy.

**Required assertions:** One outstanding notification per seat, repeated request replaces id/content; activation raises the intended app/space/tab and focuses exact pane; muted sends0. Record native adapter request/completion separately.

**Gaps at baseline afff4758:** Notification record/activation hook, frontmost/space/tab inspection and safe isolated native-backend automation. A headless mock verifies policy but cannot claim physical OS presentation or space activation; no operator click fallback.

**Baseline pointers:** attention.rs; ctl-cli.md:47; ctl.rs:251.

### M1

**Promise:** a DM, a channel post and a swarm-stream post from the composer each arrive at the right seats with a receipt; the operator principal signs as the device.

**Deterministic steps (plan):** Seed channel/swarm recipients in a private fixture; through composer send unique DM/channel/swarm nonces, retain ids; use message receipts and recipient-side bridge reads, then journal+envelope verification.

**Required assertions:** Exact recipient sets with no leakage, one persisted message/recipient and coherent stored/delivered/ack states; envelope actor is operator device, valid signature. Same --op retry is not a second send.

**Gaps at baseline afff4758:** Composer UI actions/projection, channel subscription setup and raw envelope/journal read. ctl message send/post/receipts cover transport only; operator inbox is not a recipient seat's inbox.

**Baseline pointers:** ctl.rs:1736; proto/messaging.rs:399; mcp/journal.rs:120.

### M2

**Promise:** the role chip fans out to the exact seats holding the role, one receipt each, shown as one send; above 32 seats it refuses with a named reason.

**Deterministic steps (plan):** Create2 role-matching recipients plus a nonmatching seat; select the composer role chip and send once; compare receipt set and one displayed send group. Repeat at32 and33 matches.

**Required assertions:** Exact role membership, one receipt each and one grouped send;33 refuses role_recipient_limit_32 before any effect. No classname inferred from display label.

**Gaps at baseline afff4758:** Composer role-chip send and send-group read. Looping ctl message send per seat would bypass the fanout cap and does not test this promise.

**Baseline pointers:** messaging.rs:542; ctl.rs:251; role mapping #262.

### M3

**Promise:** the operator delegates a task by #N or path to a seat; the seat's frontmatter says claimed and its row shows the task; the seat gets one oracle-class message.

**Deterministic steps (plan):** task create fixed verification task; task get returns its number/path. Delegate once by '#N', retry same op, repeat another task by path. Read task get, seat state, receipt/journal and capture row.

**Required assertions:** Frontmatter assignee/claimed state and current row task match; exactly one oracle-class delegation message per operation, no duplicate on retry, command/authority unchanged.

**Gaps at baseline afff4758:** Row-task projection and raw delegation envelope/audit read. Current task create/get/delegate supply deterministic backend steps.

**Baseline pointers:** ctl.rs:1227,1817; proto/messaging.rs:480; messaging.rs:244.

### M4

**Promise:** a report with verify_result whose command differs from verify, or whose exit code is non-zero, is refused on accept with the reason named.

**Deterministic steps (plan):** Create/delegate task with verify='sh fixture-verify.sh'; via real seat MCP task_report submit a mismatched command, then a matching command exit1; attempt task accept after each. Finally submit matching command exit0 and accept.

**Required assertions:** First two accepts return their existing named verification refusal, task not accepted; final valid report accepted once. Script reads task metadata and recorded verify_result, not assistant wording.

**Gaps at baseline afff4758:** ctl task report/accept/reject or deterministic admitted seat-MCP driver plus operator accept verb. TaskRequest has these but the ctl parser does not expose them.

**Baseline pointers:** proto/messaging.rs:504,512; ctl.rs:251; mcp/work_items.rs.

### M5

**Promise:** the operator alone lists open unassigned tasks account-wide; a seat asking gets the existing refusal.

**Deterministic steps (plan):** Seed open unassigned tasks in2 swarms plus assigned/closed controls; ctl task list --unassigned and compare exact set across account. Ask an admitted Worker and Oracle-class seat through real MCP for unassigned=true.

**Required assertions:** Only operator gets every open unassigned task account-wide; both seat attempts produce the existing operator-only refusal and no data leak. --seat/status filters still work.

**Gaps at baseline afff4758:** Real seat-MCP request+error/audit read for negative authority case. No need to add broad seat control authority; running ctl inside a seat only tests ancestry_refused, not task-list scope.

**Baseline pointers:** ctl.rs:1773; proto/messaging.rs:480; mcp/work_items.rs.

### M6

**Promise:** groups stay their own destination; a message to a muted seat waits; priority delays, never deletes.

**Deterministic steps (plan):** Seed an explicit group sink with membership independent of channels; send through message post to that exact path; mute a recipient, enqueue high/low priority nonces, restart delivery owner if needed, unmute and collect receipts.

**Required assertions:** Group address remains a group; low priority/muted rows remain persisted and later deliver once, never silently delete or reroute. Operator DM exception verified separately.

**Gaps at baseline afff4758:** Group/link/filter/mute/priority verbs or fixture setup, recipient inbox/journal reads and controlled dispatch. Existing send/post cannot create the tree or select arbitrary priority.

**Baseline pointers:** proto/messaging.rs:423,431,444; messaging/tree.rs:139; ctl.rs:251.

### F1

**Promise:** the stream dock shows one swarm at a time with its channels; arrivals appear within one event-lane delivery; nothing polls.

**Deterministic steps (plan):** Seed2 swarms/channels; choose stream dock swarm A, retain GUI event cursor; post unique messages to A and B. Wait for one event-lane delivery, dump dock rows/cursor and capture; switch to B.

**Required assertions:** A dock shows only A and its channels, B appears only after switch; displayed event correlated to one delivery/update, zero polling requests by GUI feed.

**Gaps at baseline afff4758:** Stream selection, event-lane/dock projection and request counters. ctl seat watch itself polls Screen every200ms and cannot be evidence that the GUI feed never polls.

**Baseline pointers:** ctl.rs:1680,1724; ctl-cli.md:62; live events path.

### F2

**Promise:** unread counts come from the operator's read marks and survive a restart; viewing a stream marks it read.

**Deterministic steps (plan):** With dock not viewing A post known N messages; read operator unread marks/count; open A and wait until its latest sequence is viewed, restart app through crash/adoption path or preserve the account on orderly close/start, reread marks.

**Required assertions:** Unread count equals persisted operator mark delta, becomes0 after viewing, remains0 on restart; unviewed B remains unread. No receipt-ack substitution for read marks.

**Gaps at baseline afff4758:** UI stream view/read-mark action and unread/cursor export; current message ack is a message acknowledgement, not a stream read mark.

**Baseline pointers:** proto/messaging.rs:415; ctl.rs:251; l9-messaging-tasks.md.

### F3

**Promise:** killing logic and restarting resumes the events lane at its cursors: no duplicate row, no lost event.

**Deterministic steps (plan):** Record GUI lane cursor/dock event ids; post numbered events before,during,after a controlled owned logic crash; wait for actual backend/event ready; page retained stream and compare with dock after reconnect.

**Required assertions:** Exact event-id/sequence set once in order, cursor continues from persisted value, no skipped row or duplicate; all pending rows eventually projected.

**Gaps at baseline afff4758:** Owned logic fault/status and GUI lane/dock cursor read; existing recovery can be exercised but ctl inbox is not the one GUI event consumer.

**Baseline pointers:** ctl.rs:1665; live.rs Event Reply path; l9-messaging-tasks.md:83.

### F4

**Promise:** a filter change while attached migrates the consumer: pending rows for still-granted subjects are delivered, revoked ones dropped unprojected, one audit line on account/control.

**Deterministic steps (plan):** Attach known consumer/filter with held pending rows for subjects A,B,C; update grants/filter so A stays,B revoked,C added; release delivery, read cursors/projection and account/control audit.

**Required assertions:** Still-granted A pending delivered, revoked B never projected, new C follows its admitted cursor; exactly one migration audit, no duplicate consumer or hidden wildcard.

**Gaps at baseline afff4758:** Events/filter/grant migration controls, deterministic held-delivery hook and bounded retained-event/audit read. events tail is specified but not parsed today.

**Baseline pointers:** ctl-cli.md:48; proto/messaging.rs:423,439; ctl.rs:251.

### G1

**Promise:** Settings › Appearance: Rim size 2 to 8 default 4; depth is no longer a setting.

**Deterministic steps (plan):** Open Settings Appearance in own GUI client; inspect default value, enumerate controls; change rim to2 and8 and try out-of-range values; reopen from persisted TOML and capture.

**Required assertions:** Default4, range2..8 with named invalid-input refusal, no depth control; persisted state and solved gap/inset agree. UI edit uses actual Settings writer.

**Gaps at baseline afff4758:** UI Settings open/form actions and read projection; editing TOML directly would bypass the promised UI. No settings noun is implemented.

**Baseline pointers:** settings.rs; ctl.rs:251; ctl-cli.md:47.

### G2

**Promise:** Settings › Notifications: on by default, per-harness off, only-when-not-frontmost.

**Deterministic steps (plan):** Read fresh Notifications defaults; toggle globally/per-harness/only-background through UI; produce waiting events for2 harness types under foreground/background, inspect recorded backend calls; restart and repeat.

**Required assertions:** Default on; disabled harness emits0; enabled one emits only when policy permits; persisted TOML survives restart, no unrelated setting changed.

**Gaps at baseline afff4758:** Settings mutation/read + app frontmost injection and notification recorder; headless mode must label background policy instead of inventing a native frontmost window.

**Baseline pointers:** settings.rs:251,321; attention.rs; ctl.rs:251.

### G3

**Promise:** Settings › Plugins lists every discovered plugin with trust state, tier, shape, health and the call-and-hook timeline; a 10 s hook shows the warning.

**Deterministic steps (plan):** Discover deterministic plugins spanning trust/tier/shape/healthy/failing states; open Plugins; trigger one bounded hook controlled by an injected10s clock; read timeline and capture.

**Required assertions:** Every discovered plugin appears exactly once with correct trust,tier,shape,health; hook call start/finish and10s warning visible, no UI blocking. Untrusted plugin remains unexecuted.

**Gaps at baseline afff4758:** Plugins UI host, discovery/call fixture and clock/timeline read hooks; current ctl exposes none. A file listing or plugin's own claim cannot substitute.

**Baseline pointers:** ctl.rs:251; e2e-acceptance.md:G3; plugins spec.

### G4

**Promise:** Settings › Knowledge lists roots with their provider; swapping to a stub provider and back moves no file.

**Deterministic steps (plan):** Seed a private Knowledge root and hash its complete file manifest; open Knowledge settings, replace its data source with a deterministic stub, read/preview fixed records, then restore original provider and rehash.

**Required assertions:** Roots and provider ids correct; selected provider serves reads; original paths/inodes/content/mtime unchanged and no migrated copy; restoring recovers original records.

**Gaps at baseline afff4758:** Settings Knowledge/provider-switch actions and source/record read projection; ctl record fetch/search not implemented. Provider fixture credentials must be private/dummy.

**Baseline pointers:** ctl-cli.md:44,47; ctl.rs:251; knowledge data_source.

### G5

**Promise:** theme is data: a token change in design/tokens.toml changes the look without a code change.

**Deterministic steps (plan):** Use two private copies of the same source tree; change only design/tokens.toml color.dark.accent from#2E6BFF to a fixed contrasting colour in copy B, regenerate/build through existing token pipeline; start both headless with identical fixtures/theme/scale/time and capture.

**Required assertions:** Source diff is TOML-only (plus generated output), affected accent pixels change to specified value; unrelated geometry/terminal cells stable. No claim of live hot reload: this promise says no code change.

**Gaps at baseline afff4758:** No missing live ctl verb for the data/build comparison; needs pinned-build fixture and deterministic UI state/clock. This read-only plan does not perform the edit/build.

**Baseline pointers:** design/tokens.toml:19; ctl.rs:62; token build pipeline.

### D1

**Promise:** a .md opens in the modal rendered natively: headings, lists, tables, code, task lists, footnotes; true size, never scaled.

**Deterministic steps (plan):** record write a private markdown fixture containing every listed construct; open it via app docs route, read document layout and capture at exact1x/2x viewport sizes; compare against approved native geometry and font-size fixtures.

**Required assertions:** Headings,lists,tables,code,tasks,footnotes present with correct hierarchy and measured logical sizes; no whole-page raster scaling, no missing content.

**Gaps at baseline afff4758:** ctl record fetch and ui open docs, modal/layout read plus deterministic typography capture. Record creation alone does not prove native rendering.

**Baseline pointers:** ctl-cli.md:44,47; ctl.rs:251; bbt-doc/document host.

### D2

**Promise:** a mermaid fence renders through our engine: no overlaps, edges on borders, deterministic.

**Deterministic steps (plan):** Write a fixed Mermaid-containing doc; open native modal; read node/edge layout, capture twice with same inputs/time and compare structural dump/PNG hashes.

**Required assertions:** No node overlaps, each edge endpoint on its node border within existing geometry tolerance, same deterministic layout and pixels on repeats; native Mermaid engine used.

**Gaps at baseline afff4758:** Document-open and diagram geometry dump hook. Mermaid crate fixture tests can support engine correctness but do not prove the app hosts it.

**Baseline pointers:** ctl.rs:251; bbt-mermaid; document host.

### D3

**Promise:** `[[slug]]`, `[[#N]]` and `[[slug|alias]]` resolve; a bad link shows as broken, never silently skipped; ![[bboxy.png]] renders the image from .bboxy/images.

**Deterministic steps (plan):** Write numbered target doc, alias-bearing links, invalid link and private .bboxy/images/bboxy.png fixture; open source modal, activate each slug/#N/alias, then broken link and image.

**Required assertions:** All3 valid targets resolve to same expected id/digest and label; broken link has named broken state and no silent navigation; image decoded at expected geometry/content hash from allowed root.

**Gaps at baseline afff4758:** Record fetch/search and semantic document-link activation/document layout/error read; no OS click. Image source is the existing bboxy identity fixture, never generated.

**Baseline pointers:** ctl-cli.md:44,47; ctl.rs:251; document host.

### D4

**Promise:** the modal opens, scrolls, copies, closes, and feeds the document into the focused harness.

**Deterministic steps (plan):** Open long doc; use app scroll; select known text range and copy into a private injected clipboard; close with own Esc action; reopen and feed document to focused harness A, having changed focus from B before the action.

**Required assertions:** Expected scroll anchor/selection/copy bytes, modal dismissed, exactly one document payload reaches A through admitted input/message path; no unintended PTY paste/OS clipboard access.

**Gaps at baseline afff4758:** UI modal scroll/selection/copy/close/feed actions and clipboard test adapter, focused-seat read; ctl seat paste does not prove modal Feed producer.

**Baseline pointers:** ctl.rs:251; ctl-cli.md:47; document host.

### N1

**Promise:** every record has a unique number; a new record created through the app takes the next free one; a duplicate or invalid number is refused and shown as broken.

**Deterministic steps (plan):** Read bounded Index inventory and next-free number; `task create` and record write valid fixture records concurrently; reread assigned numbers. Submit duplicate/invalid numbers through app write; separately index corrupt private fixture rows to check broken projection.

**Required assertions:** Unique positive allocated numbers, no duplicate effects on same-op retry, invalid writes refused by name; externally malformed records displayed as broken, not silently omitted. Successful app writer may allocate a number when omitted.

**Gaps at baseline afff4758:** ctl record fetch/search/changes and Index diagnostics read. task get exposes task metadata only; raw fixture corruption must stay in isolated data, not production .bboxy.

**Baseline pointers:** ctl.rs:1227,1817; ctl-cli.md:44; Index writer.

### N2

**Promise:** every decision is its own node; the Graph tab shows a decision with its decides, constrains, evidence, answers, amends edges.

**Deterministic steps (plan):** Write a decision plus linked spec/evidence/question/earlier-decision records with all5 edge relations; fetch them, open Graph and inspect graph nodes/edges, then capture.

**Required assertions:** Decision is distinct numbered node; decides,constrains,evidence,answers,amends edges point to exact fixture ids and carry correct direction/labels; no flattened decision text.

**Gaps at baseline afff4758:** General record read and Graph UI projection/selection; ctl e2e list covers checklist only, not full graph.

**Baseline pointers:** ctl.rs:251; ctl-cli.md:44,47; Graph host.

### N3

**Promise:** a comment pinned to a digest shows beside the version it was made on; the RFC loop runs in the app: proposal, comments, decision, spec.

**Deterministic steps (plan):** Create versionA proposal, fetch digestA, write comment pinned to it; amend proposal toB, addB comment, produce decision then spec via app writer; select both document versions/Graph threads.

**Required assertions:** A comment shown only beside digestA, B besideB; proposal/comments/decision/spec ids and relation chain intact, stale etag refused; exact signed writer receipts retained.

**Gaps at baseline afff4758:** Record fetch/version/digest and generic write authority/Graph-document thread projection; if pinned historical version cannot be fetched, name that as a product gap.

**Baseline pointers:** ctl.rs:818; ctl-cli.md:44; work-items/RFC schema.

### N4

**Promise:** the Graph tab shows agents (seat, role, now, blocked on), the work tree, the provenance card and what each seat listens to.

**Deterministic steps (plan):** Spawn known roles, delegate task, hold a dependency and set known subscriptions through admitted paths; open Graph agent/worktree/provenance panels and read structured projection plus journal.

**Required assertions:** Every agent id/role/now/blocked-on/worktree matches actual records; subscription set exact; provenance distinguishes seat vs operator/device/door/issued_by and links to operation/digest.

**Gaps at baseline afff4758:** Graph selection/projection, subscription controls, record/audit read; a seat list without these fields is not an agent-graph proof.

**Baseline pointers:** ctl.rs:251; ctl-cli.md:47,49; mcp/journal.rs:120.

### I1

**Promise:** first launch enrols one device key in the Keychain with one prompt under the GPG root; later launches prompt nothing.

**Deterministic steps (plan):** Run fresh and reopened throwaway file-account headless lifecycle; verify device id stable and no native-store calls. Separately exercise native custody enrollment with an isolated non-operator Keychain/test backend that records enrollment/root-cert/prompt attempts.

**Required assertions:** Automation run proves its own file custody and0operator prompts. Exact legacy I1 additionally requires one device enrollment rooted by GPG and one first-access prompt,0later prompts in the isolated native fixture.

**Gaps at baseline afff4758:** Policy conflict: literal login-Keychain/prompt criterion cannot be proved by file custody or require operator action. Need isolated native-custody/prompt-count adapter or a revised criterion; never silently pass I1 from memory/file tests.

**Baseline pointers:** e2e-acceptance.md:I1,4a; ctl-cli.md:50; tests/headless.rs:351.

### I2

**Promise:** every seat has its own generation key certified by the device key; every message envelope verifies; a seat with a nil seat id and a non-operator role is refused.

**Deterministic steps (plan):** Spawn2 real seats and restart one generation; read their cert chains and actual sent envelopes from bounded journal/export. Verify Ed25519 signatures offline with their own test device root, unique generation keys; submit nil-seat/nonoperator forged identity through negative bridge fixture.

**Required assertions:** Distinct per-seat/generation keys, matching certified seat/generation/device, every asserted envelope cryptographically verifies; nil-seat worker/oracle is refused before journal effect, no forged class uplift.

**Gaps at baseline afff4758:** Certificate/envelope inspection and negative-admission fixture/identity status export. ctl normal operator calls cannot fabricate a seat principal by label.

**Baseline pointers:** proto/messaging.rs:136; identity modules; ctl.rs:251.

### I3

**Promise:** the local-dev issuer path works offline and is marked as such everywhere it shows; pinned receivers refuse local-dev envelopes.

**Deterministic steps (plan):** Own offline fixture blocks network, selects local-dev issuer, starts/spawns/messages; read seat/frame/receipts/Graph grades. Submit the same signed envelope to a fixture receiver pinned to the trusted test root.

**Required assertions:** Local-dev path works without network and label persists at every projection; pinned receiver returns its existing untrusted-issuer refusal and no delivery. Offline and trust are independent checks.

**Gaps at baseline afff4758:** Selectable local-dev/pinned-receiver fixture, network isolation and all identity-grade projections. 'Unattended file-account' alone is not proof of local-dev grade everywhere.

**Baseline pointers:** identity modules; ctl.rs:251; messaging trust policy.

### I4

**Promise:** every record write from the app is signed by the device key with signer device:<id>; git log shows only good signatures on main.

**Deterministic steps (plan):** Create task/e2e/doc through app writer in own account, read exact stored bytes/signature blocks and verify against its device certificate; retry same op and verify no duplicate. Run `git log --format='%H %G?' origin/main` plus signature verification for the checked acceptance range.

**Required assertions:** Each app record signer is device:<own-device-id>, signatures bind exact bytes and correct account; checked main commits all good signed. Keep root/private keys out of logs and Git.

**Gaps at baseline afff4758:** Bounded record/device-certificate read/export (record write returns outcome, not all cryptographic provenance). Existing git commands prove Git only, not app writer signatures.

**Baseline pointers:** ctl.rs:818; mcp/journal.rs:120; AGENTS.md signed commits.

### P1

**Promise:** the hitch meter shows zero hitches idle at 120 Hz with 1000 sidebar rows and 50 watched panes once R5g lands; the HUD is in our look.

**Deterministic steps (plan):** Use own safe optimized+debug-assertions build; at quiet loadavg1<8 record before/after and run `GATE_SEATS=967 GATE_WATCHED=50 GATE_SECONDS=10 GATE_BUDGET=8.33 sh scripts/gates.sh frame` on this host; retain full Report/capture spans, all50 advancement and settled IDLE checks; run3 only after successful mechanism-valid attempts and report per-metric median. Separately drive exact1000 real seats/120Hz on a capable test host.

**Required assertions:** Zero idle frames/callbacks/hitches; all watched panes advance and settle; frame-work p99<=budget, CPU p99<=4ms and other unchanged bounds. Exact1000/120Hz only if actually measured; HUD pixels checked against approved fixtures.

**Gaps at baseline afff4758:** Current host has967PTY/75Hz proof ceiling; frame invariant remains pending and latest run is unsettled. Need app meter/clock/visible-set export for general ctl runner. Offscreen or synthetic1000 cannot be relabelled as physical120Hz/1000PTY.

**Baseline pointers:** scripts/gates.sh:11; invariants/seats-967-frame.md:9; runtime.md:57.

### P2

**Promise:** an Oracle broadcast reaches 1000 seats under the bus budget; 400 idle seats stay under the memory budget.

**Deterministic steps (plan):** Run `GATE_SEATS=1000 GATE_BUDGET=13299 sh scripts/gates.sh bus` and independently `GATE_SEATS=400 GATE_RATE=idle GATE_SECONDS=20 GATE_BUDGET=420 sh scripts/gates.sh memory`; retain GATE lines, exit codes, recipient totals and supervisor+logic RSS scope. Add ctl-driven Oracle send/receipts scenario when explicit bus fixture setup is exposed.

**Required assertions:** Bus fixture: all30 broadcasts reach expected recipients, p99<=13299ms; label its1000 seats synthetic. Memory fixture:400 real idle PTYs, supervisor+logic RSS<=420MiB. No loaded-memory or1000realPTY inference from these separate cases.

**Gaps at baseline afff4758:** No missing verb to run existing gates; ctl needs batch fixture/metric export to combine them into the runner. Safe custody/build settings and own private run directories mandatory; performance jobs require resource claims/quiet conditions, not operator acts.

**Baseline pointers:** invariants/seats-1000-bus.md:9; invariants/seats-400-memory-idle.md:9; scripts/gates.sh:29.

Owner: Corvin Duskmantle <ARCHITECT>; rows and archived plans by Aster Cinderwing <EXECUTOR> under the Oracle’s brief.
