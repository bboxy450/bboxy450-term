---
type: spec
title: 'Gaps and improvements: ranked by gain per cost (2026-11-12)'
status: draft
id: 01M79JBMFXKVMEHDV6X4WXKHRM
created: 2026-11-13T04:23:54.208Z
number: 1794292861627500
intent: Rank the measured gaps in bboxy by gain per cost so the next work is chosen from evidence, not from the last thing someone noticed.
updated: 2026-11-16T19:53:41.523Z
created_by: seat:01M79HGAW321GYNK8468FYN4K9
updated_by: seat:01M79HGAW321GYNK8468FYN4K9
authorship:
  signer: seat:01M79HGAW321GYNK8468FYN4K9
  kind: seat
  public_key: O/GLv3wEDMQc2X2CJwZN8QH5avRWsmx3x4SdZZC6lKU=
  process_generation: '1'
provenance:
  digest: sha256:0b9f7b90da7e7c220f7523c51ff896309691e891e2ec4b921065397b6ab7b970
  frontmatter_digest: sha256:b005ee998f0eaad986c4e804b278237824a022e7e012304ea04f03da3b835d22
  signer: seat:01M79HGAW321GYNK8468FYN4K9
  sig: vrP3dR/j31pgiDjKQ3Kcxfk6kqBKd+s5MS+BPlakPMRjITRVYiVgWWqYOrI1VMlrjAErfD1MYdI5GdHpgFYmBQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151812
  event:
    id: 01M7JYR9ZZ1XB06AGTPTH1FWAQ
    sequence: 151812
    hash: e9b621a199fa186c565c5e47ad21b72a7f390a4e936c478740e3dcabce6e603d
---
# Gaps and improvements, ranked by gain per cost

Written 2026-11-12 by Lumen Foxglove (solo) from fifteen read-only sweeps: three of the code and twelve comparing each piece of bboxy with the current state of the art. Nothing was built or run. Evidence is `file:line` or a record path; numbers marked *estimated* were not measured.

The stand is [[philosophy]]. Every section says what to delete as well as what to add.

## 1. What bboxy is today

| Piece | Where | Size | State |
|---|---|---|---|
| App (window, scene, ctl, cockpit) | `crates/bboxy450-term` | 82k src / 45k tests | `main.rs` is 14,225 lines |
| Mux logic (registry, messaging, MCP, knowledge, provenance, dream, plugins, identity) | `crates/bbt-mux` | 114k src / 43k tests | `logic.rs` is 15,480 lines; one `bbt-logic` thread, ~115 `Command` variants |
| Supervisor (PTYs, ring, journal) | `crates/bbt-supervisor` | 15k | one mio thread for every seat |
| Harness (own engine, ACP client and server, GUI face, dream) | `crates/bboxy450-harness` (submodule) | 91k src / 79k tests | dream is 34k of it |
| Wake-up kit (OSC 7501, hooks, native sockets, guard) | `crates/bboxy450-harness-wake-up` | 16k | 4.6k of it is dream |
| UI components and model | `crates/bbt-ui`, `bbt-ui-model` | 44k | custom retained layer on winit + wgpu |
| Terminal core and renderer | `crates/bbt-core`, `bbt-render` | 27k | vendored vte/alacritty term |
| Formal stack | `bboxy450-tlc`, `bbt-yarn*`, `bboxy450-den`, `verification/` | 21k Rust + 1.5k Python | 958 models; 713 are under 1,000 states |
| Plugins | `bbt-plugin*`, `bbt-mux/src/plugins`, `plugins/gitkb` | ~34k | 8 contracts, 2 with a caller, 1 real provider |
| Knowledge base | `.bboxy/` | 994 MB on disk | 236 tasks, 61 specs, 136 decisions, 27,881 artifact records |

Processes: the app binary runs as GUI, `supervisor` and `logic` (`cockpit.rs:701-703`), plus `mcp` (stdio bridge), `ctl`, `hook`. The supervisor owns PTYs and survives a logic crash; logic owns one SQLite per object. Seats talk to logic through the `bboxy` MCP bridge; harness seats also expose a rich Unix socket that the app embeds as a pane.

Four structural facts drive most of the list below:

1. **Two files hold the system.** `logic.rs` (76 commits since 10-29) and `main.rs` (50). Everything serialises through one `Worker`.
2. **Dream is 102k lines in five places and returns nothing.** A live `dream_search` answered `not_prepared` / `no_published_input`.
3. **The gate is slower than the work.** Last green full run: 3 h 42 min. 9 of the last 12 full runs went red, 20–146 min in.
4. **Records outnumber consumers.** 27,774 of 27,881 artifact records are unclaimed; 79 accepted tasks never closed; 6 of 8 plugin contracts have no caller; 6 message kinds have no producer.

## 2. The ranked list

Cost: S = hours to a day, M = days, L = a week or more. One list, best gain per cost first.

### Low cost, high gain

| # | Change | Cost | Evidence |
|---|---|---|---|
| 1 | **Full gate from ~222 to ~100 min** with four script edits: formal reuse on in the full gate; Java TLC in a pool of 6–8; formal and clippy before the long test stages; delete invariant checks that re-run nextest tests and the second workspace clippy. Details in §6. | S | `scripts/ci-local.sh:93`, `scripts/formal.py:913-915`, ledger `2026-11-12` |
| 2 | **Bound `orient` under 8 KB.** Drop the tool list, page `destinations` (it lists two rows for every seat the swarm ever had), stop serialising every payload twice, write hints once and only when non-obvious. Enforce the specified 16 KiB result cap. | S | `logic.rs:6104-6161`, `mcp/mod.rs:43,185-188`, `mcp/hints.rs:431-456` |
| 3 | **One python server.** The same four tools are served by `bboxy` and by `bboxy_python`. Delete one. | S | `mcp/bridge.rs:298-313`, `harness/src/mcp.rs:304-316` |
| 4 | **Task rows show their documents** from data that already exists (§4). | S | `bbt-proto/src/messaging.rs:1124-1133`, `seat_row.rs:970-978` |
| 5 | **Stream dock shows signal by default** (§5, step 1): decisions, task delegate/report/reject, red gates, questions. Hide receipts, signatures, presence. Pin a "needs you" section. | S | `stream_dock.rs:770-965` |
| 6 | **Raw PTY bytes in supervisor frames.** `bytes: Vec<u8>` travels as a JSON number array, 3–4× the size, on the leg measured at 1,349 ms of a 1,398 ms input-to-GPU path. | S | `bbt-supervisor/src/protocol.rs:335`, `specs/budgets.md` |
| 7 | **Knowledge base hygiene.** Reconcile 29 unowned open tasks, close 79 accepted, one status serialisation (`done` vs `"done"`), fill an artifact's `for` when its seat holds exactly one live task. | S | `provenance/artifacts.rs:119` |
| 8 | **See what a seat costs.** Per-seat CPU-seconds, RSS and bytes on disk in `usage_read`. The run dir is 21 GB and the disk is at 89%. | S | `bboxy450-usage/src/lib.rs:128-136` |
| 9 | **ACP rows the harness already has types for.** Map subagent parent ids to `Item::Subagent`; advertise `session.compaction` and map `compaction_update`; report a question as `kind=question` on OSC 7501, keep `done` distinct, parse `progress`. | S | `acp_agent.rs:707,2318-2337`, `seat.rs:268-272`, `observe.rs:511` |
| 10 | **Renderer allocations.** Retain the instance, background and uniform buffers in `prepare`; bound the glyph cache. Encode measures 10.8–20.8 ms against a 1.0 ms allocation. | S | `bbt-render/src/cells/terminal/gpu.rs:143,357-456` |
| 11 | **Perceived quality, five small ones.** Hover transitions on every interactive row, grab cursor and shadow while dragging, truncate the subtitle before the name in narrow strips, honour reduce-motion in the chrome, one `icon()` helper instead of five. | S | `accordion.rs:156`, `pointer_cursor.rs:19-23`, `widgets.rs:57` |
| 12 | **Sidebar subtraction.** Remove the SSH Hosts and Cloud accordions (Settings already holds them), collapse idle seats to "N idle", hide accepted and done tasks. | S | `sidebar.rs:20-30`, `settings.rs:227-228` |
| 13 | **Specs that describe what exists.** Fix or cut: `architecture.md:174,206`, Quinn in `networking.md:19`, parser shards in `runtime.md:33`, Pi/Omp and resolved gaps in `harness-adapters.md`, phantom corpora in `renderer.md:97-101`, `plugins.md` status, the stale ACP ICD lines on elicitation. Reconcile the dream task ("never refused") with its decision (50 ms) and the code. | S | as cited |
| 14 | **Formal evidence store.** Keep DOT graphs only for the newest receipt; hashes cover the rest of 5.7 GB. | S | `.git/bboxy-formal-evidence` |

### The operator's features (mid cost, high gain)

| # | Change | Cost | Section |
|---|---|---|---|
| 15 | **Automation: a decision model answers harness questions**, per-seat toggle next to mute, default on. | M | §3 |
| 16 | **Move a seat between swarms**, and out into its own. | M | §7 |
| 17 | **The stream becomes a task-threaded ledger.** | M | §5 step 2 |

### Mid cost, mid to high gain

| # | Change | Cost | Evidence |
|---|---|---|---|
| 18 | Gate under 50 min: replace the five mutate-and-rebuild controls with a runtime switch, one prebuilt nextest invocation for all stages, 6–8 jobs with a serial group for PTY and timing tests. | M | §6 |
| 19 | Resume ACP sessions when a seat restarts (`session/resume`, fall back to `session/load`). Today only `session/new` is called, so a restart loses agent context. | M | `acp_agent.rs:1795` |
| 20 | Merge the tool catalogue from 79 to about 35 (`ch_*`, `provenance_*`, `skill_*`, task transitions, mute/unmute, the three `*_orient`), set always-load on five, add a concise/detailed switch. | M | `mcp/tools.rs:22-108` |
| 21 | Full-text search for the KB (SQLite FTS5, match-centred snippets). Today it is a linear substring scan. | M | `knowledge/bboxy.rs:923-990` |
| 22 | Live tokens and estimated cost per seat, by moving the nightly transcript reader into the poller. Fill the wake ledger's `input_tokens` (null on all 2,712 rows). | M | `scripts/inference-ledger.py`, `messaging/wake_cost.rs` |
| 23 | Memory that returns something: write own-seat lessons as plain KB records at session close and search them with #21. Keep the proof machinery off the read path. | M | `specs/dreaming-mcp.md:61` |
| 24 | Daemon stops linking the UI: move canvas rendering out of `bbt-mux` so it drops `bbt-render`, `bbt-ui`, `png`. Then move dream, knowledge, provenance and identity to their own crates. | M | `bbt-mux/Cargo.toml`, `mcp/canvas/render.rs` |
| 25 | Model-to-code link: run `den replay` over traces from existing integration tests as a gate stage; reject vacuous invariants. | M | `bboxy450-den/src/lib.rs:79`, `Makefile:1836-1850` |
| 26 | Delete what has no consumer: six plugin contracts without a caller, six message kinds without a producer, `ch_link`/`ch_pause`/`ch_grant`, `dream_feedback`, artifact records with no producer and no task (keep the events), document types with zero or one record (concern, epic, runbook, proposal). | M | `bbt-plugin/src/contracts.rs:33`, `messaging/wire.rs` |
| 27 | OSC 133 prompt marks in core: waiting-on-input for plain shells, jump to prompt. | M | `bbt-core/src/vendor/vte/ansi.rs:1593` |
| 28 | Unattended seats run inside Seatbelt instead of with every agent sandbox disabled. | M | `wake-up/src/launch.rs:160-238` |
| 29 | Operator-run sweep for cold blobs. Nothing can delete today. | M | `bbt-supervisor/src/store/mod.rs:49-59` |
| 30 | Spring motion that keeps velocity; a native first run. | M | `scene/sidebar_motion.rs:15,223` |

### High cost

| # | Change | Cost | Gain |
|---|---|---|---|
| 31 | One actor per seat (own queue, own SQLite) replacing `Command` and `Worker`; the storage spec already names this model. Removes the head-of-line `block_on`. | L | high |
| 32 | Seeded deterministic simulation of mux and supervisor with model invariants as runtime assertions. | L | high |
| 33 | One observation path per harness: ACP seats already carry state, permissions and usage in-band, so the hook shim, native-socket wake and PTY guard retire for Claude and Codex once #19 lands. | L | high |
| 34 | Split `main.rs` and `logic.rs` by owner. Falls out of #31 for logic. | L | mid |
| 35 | AccessKit tree. | L | mid |
| 36 | MCP 2026-07-28 (stateless, `server/discover`). | L | mid |

### Not worth doing

SIMD parser (parsing is not the measured bottleneck), a native Metal port, ligatures, a WASM plugin host, a plugin marketplace, a protobuf migration, a kanban board, attenuable capability tokens, embeddings or a graph memory, retiring Java TLC by chasing complete TLC conformance, Verus or Aeneas on product crates.

## 3. Automation: a decision model answers questions

### What is true today

- Term launches every harness seat in full access (`wake-up/src/launch.rs:160-238`), so permission requests are already approved by rule (`acp_agent.rs:2043-2053`).
- A question (ACP form elicitation, which is how `AskUserQuestion` arrives) always waits for a human: `question()` at `acp_agent.rs:2096-2147` has no other branch. This is the only place a harness seat blocks on the operator.
- A `decision_model` plugin contract exists in term (`bbt-plugin/src/decision_model.rs:16-37`) with no caller and no Jev provider. No Jev client or key exists in the workspace.

### Behaviour

When a question arrives and automation is on for the seat, the harness asks the configured decision model and answers with its choice. Otherwise the card waits as it does today.

**One home.** The question, its schema and the answer path all live in the harness, and the harness must work without term. So the harness calls the model directly over HTTPS (it already has `reqwest`). Term owns only the per-seat toggle and its icon. The callerless term contract is deleted under #26 rather than grown into a second path.

**Eligible questions.** Every field is single-select with 2–6 options. Free text, multi-select and URL elicitations always wait for the human.

**Always the human.** Before the model is called, a fixed word list on the question and option text (credential, token, password, login, publish, release, push, merge, deploy, pay, spend, delete, force, reset) sends the card to the human. The list is crude on purpose: it can only make escalation more likely.

**What the model sees.** The question text, each option's label and description, the seat's role, and the latest operator message of the session, capped at 4 KiB. No tool output. One extra option, `ask_human`, is always offered. Instructions are fixed text owned by the harness.

**Accepting an answer.** The model is asked twice, the second time with the options reversed (Jev favours the first option). Both must agree, name a real option other than `ask_human`, and reach confidence 0.6.

**Never hangs, never guesses.** Deadline 5 s for both calls together. A timeout, an error, a missing key, disagreement, low confidence or `ask_human` all leave the card waiting for the human, labelled with the reason.

**Circuit breaker.** Three consecutive non-answers on a seat pause automation there and raise attention.

**Record.** `Resolver` gains `Model { model, confidence }`. The completed card reads "answered by automation: <option> (0.83, jev-1.13.0)". The human can still interrupt and correct.

### Interfaces

| Thing | Shape |
|---|---|
| Provider config (harness) | `~/.bboxy450-harness/settings.json`: `decision_model: { url, model, api_key }`, `api_key` resolved through the existing `"!command"` form (`profile.rs:46-55`). Wire format is Jev's: `POST /v1/systemone`, bearer, `choice` + `instructions` + `criteria` → `choice`, `confidence`, `probabilities`. |
| Harness default | profile field `automation`, default true (`#[serde(default = "enabled")]`, as `auto_compact`) |
| Harness op | `Op::SetAutomation { on }` over the rich socket |
| Seat record (term) | `SeatRecord.automation: bool`, default true, stored like `muted` (`bbt-proto/src/registry.rs:96-116`) |
| Operator controls | header icon left of mute; `ctl seat automation <seat> on|off`. No seat tool: a seat does not switch its own supervision. |
| Icon | 16 px box, 12 px bolt, same geometry as mute (`seat_row.rs:696-719`), mirrored in the pane strip (`strip.rs:348-359`). On: `color.ui.ok`. Off: muted. Paused or no provider configured: struck through, with the reason in the tooltip. Both slots always reserved. |

Default on with no provider configured shows "unavailable" and behaves exactly as today.

### Scope limits

- The toggle governs questions. Permission approval stays as launched (full access); changing that is #28.
- Seats that are a plain Claude or Codex CLI in a PTY are not covered: there is no ACP there.

### Verification

- Model (state: question ∈ {waiting, asking, answered-by-model, answered-by-human, cancelled}; automation ∈ {on, off, paused}). Safety: a question is answered at most once; no model answer lands after a human answer, a cancel, or a switch to off. Races to cover: human answers during the call, interrupt during the call, toggle off during the call, reply after the deadline, A → off → on. Broken control: remove the still-pending check before settling.
- Process witness: extend `tests/fixtures/acp-agent/fake_agent.py` and a local fake provider; assert the agent receives the chosen option, and receives nothing on timeout until a human answers.
- Term: `ctl seat automation` round-trip and a capture of the icon in its four states.

## 4. Tasks show the documents they touch

No new record type. `touches(task)` is a projection over what is already written:

| Source | Relation | Where it is today |
|---|---|---|
| The task's `links`, `parent`, `derived_from`, `depends_on`, body wikilinks | reads | task frontmatter |
| The task's `artifacts` and `report` | wrote | `mcp/work_items.rs:1395-1397,1751` |
| Artifact events from the assignee seat between claim and report | touched | provenance log (step 2, needs #7) |
| `.bboxy/` paths in commits with the `Bboxy-Task` trailer | committed | `provenance/git.rs:202` (step 2) |

`TaskCard` gains `touches` (capped, grouped by type). A collapsed row shows one chip, "2 specs · 1 report"; expanded, the groups Reads / Wrote / Report. A click opens the existing document viewer. Paths outside `allowed_paths` draw in the error colour. The row's unused `constrains` and `violates` slots carry this instead of new ones.

Acceptance: a task with a linked spec and a report shows both on its row in a `--capture`, and clicking opens the document.

## 5. The stream becomes something worth keeping open

Measured: 2,712 wakes in 18 days, 2,703 of them seat-to-seat DMs, 9 on channels. The dock renders all of it as chat cards with signature lines and receipts.

**Step 1 (S).** Default filter to decisions, task delegate/report/reject, red gates and questions. Hide receipts (unless held or failed), signatures and presence. Pin "needs you": open questions to the operator, waiting seats, reported tasks awaiting acceptance.

**Step 2 (M).** One line per event, grouped by task, needs-you first: `time · task · seat · verb · object`. Verbs are a closed set derived from the message kind and task transition (`asked`, `blocked`, `decided`, `delegated`, `reported`, `rejected`, `landed`, `red`), never parsed from prose. A landed and accepted task folds to one line. Raw chat stays as a per-seat drill-down.

## 6. Tests: from 3 h 42 min to under 50

Where the last green full run went (ledger, `fae377aea`, 2026-11-12):

| Stage | Minutes | Cause |
|---|---|---|
| `make test-formal FORMAL_REUSE=0` | 72.9 | ~65 is Java TLC: two JVM launches per model, one worker, in series |
| `make test-harness-native` | 45.5 | separate cargo invocation |
| Invariants | ~34 | two near-identical passes; ~20 checks re-run tests nextest already ran, plus a second workspace clippy |
| Workspace nextest | 23.9 | two test threads on 16 cores |
| Timed mux tests | 18.9 | 0.4 in another run: a feature-set rebuild (*inferred*) |
| Capability control | 12.6 | edits app source and recompiles the 127k-line crate, twice |
| Builds, lint, smoke | ~15 | |

| Step | Change | Saves | Cost |
|---|---|---|---|
| a | Reuse on in the full gate; uncached Java only when the checker, exporter or a model hash changes, and nightly | 60–65 min | S |
| b | Java TLC pool of 6–8 | ~55 min cold | S |
| c | Delete duplicate invariant checks, the second clippy and the redundant pass | 20–30 min | S |
| d | Formal and clippy first, so a red run fails in minutes | up to 50 min per red run | S |
| e | Mutation controls as a runtime switch in one binary | 25–40 min | M |
| f | One `--no-run` build, then filters, for harness-native and timed stages | 15–30 min | M |
| g | 6–8 jobs, 4–6 test threads, a `max-threads = 1` group for PTY and timing tests | 10–15 min | S |
| h | One test binary each for bbt-ui (38 files), bboxybook (28), bbt-proto (21) | 2–5 min | M |
| i | Injected clocks for the 675 sleeps | unmeasured | L |

`make land` (lean) is already a median of 11 min and stays the per-commit gate. Steps a and b do not weaken the formal check: the reuse key covers model, expectation, tool pins and checker hash.

## 7. Move a seat between swarms

Today `move_seat` refuses across swarms (`bbt-ui-model/src/model/window.rs:92`, `sidebar_reorder.rs:150`), and a seat's grant names its swarm.

- **Model.** `SidebarMove::SeatToSwarm { seat, swarm, before }` and `SeatToNewSwarm { seat }`.
- **Drop targets.** Between rows of another swarm; a swarm header (a collapsed swarm opens after ~500 ms); the "+ new swarm" row, which reads "new swarm from <seat>" while dragging.
- **Without a mouse.** Row menu "Move to ▸", a launcher verb, and `ctl seat move <seat> <swarm|new>`. The existing `ctl ui drag` drives the tests.
- **Mux.** One action, `SeatMove { seat, generation, target }`: re-issue the seat grant for the target swarm under a new epoch, move the seat's inbox node, post a notice in both streams as mute does. The seat's process, session and workspace are untouched.
- **Refusals (v1).** A seat that is assignee or delegator of a non-terminal task is refused with the task named; so is the Oracle of a swarm that still has other seats. Close or reassign first.
- **Feedback.** Lifted ghost row, siblings slide as they do now, Esc cancels.

Model: grant epoch is monotonic; a message addressed before the move is delivered once, to the seat; A → B → A leaves one inbox and one grant. Broken control: keep the old grant valid after the move.

Open, to settle from the code before building: whether seat inboxes live in the per-swarm messaging store (`messaging/service.rs:916,1186-1196`) and so need a copy, or can be re-parented.

## 8. Order of work

No build or test runs until the other swarm's CI repair is finished. Then one item at a time, each landed before the next starts:

1. Gate speed, steps a–d (#1)
2. `orient` and result bounds (#2, #3)
3. Automation (#15): harness first, then the term toggle
4. Task documents (#4)
5. Stream filter (#5), then the ledger (#17)
6. Seat move (#16)
7. The rest of the list in order

## Acceptance criteria

- [ ] Each landed item cites its number here and reports what it removed.
- [ ] Full gate wall time is recorded before and after #1 from the landing ledger.
- [ ] `orient` for an Oracle seat in a 30-seat swarm is under 8 KB.
- [ ] A harness question on a seat with automation on is answered without the operator in the process witness, and is left waiting on timeout.
- [ ] A task row shows its linked spec and report.
- [ ] A seat moves to another swarm and back through `ctl seat move` with one grant and one inbox at the end.
