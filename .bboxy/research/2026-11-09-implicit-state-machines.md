---
type: research
title: 'Implicit state machines: inventory and ranked dispositions'
id: 01M70FTF88ZZCCWG3FQT7THKCS
updated: 2026-11-09T16:48:29.126Z
number: 93377961366462
created_by: seat:01M70FBP1RV5AWZ5AE3ZZWKNHB
updated_by: seat:01M70FBP1RV5AWZ5AE3ZZWKNHB
authorship:
  signer: seat:01M70FBP1RV5AWZ5AE3ZZWKNHB
  kind: seat
  public_key: SkH0jBJLMgNQ8eyjFw4BxnIDzq9wMmgZUeBUslwMcCw=
  process_generation: '1'
provenance:
  digest: sha256:3f9dc3332c279741ec3aff5f0d6e96e68be5e3f92621f36ddd1bca89b33d8312
  frontmatter_digest: sha256:97cd5b11a55054fe9fe4cc5d9c36530d73dfb7457998e971c0093c7863aa5d1a
  signer: seat:01M70FBP1RV5AWZ5AE3ZZWKNHB
  sig: oywwjd7kcqIss7spN4pXyHfPybb7wpsVN6rJh1yFXmI96PHj8nnhAdz0zQ/5ma15iEa6owsqp/QSnvrKAeaKAQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151077
  event:
    id: 01M70KC56EF66N4ZECM97GH9NG
    sequence: 151077
    hash: 328e40d4dcd305400023e544082401a363ec4cae893f603658a90acf447e1c33
---
# Implicit state machines: inventory and ranked dispositions

These are lifecycles encoded as bool, `Option`, integer or string fields with transitions scattered across methods, rather than as one typed enum with one transition function. The inventory came from four read-only sweeps (one per crate group) on 2026-11-09 against `main` 96dbc71ba. I re-checked finding T1 against the source myself. The other findings come from reading the code: each is a lead to verify before its fix, not a confirmed bug.

## Decision

**The task lifecycle comes first.** It is the one machine that serves all three audiences:
- **UX:** the UI's Claim, Accept and Reject buttons and status colours.
- **DX:** the same status literals are written out in about 20 files.
- **AX:** every agent depends on the guarantee that accepted and done tasks stay frozen.

It is also the only finding where the code has a confirmed defect against a model that already exists.

**First cut: landed on main** as 161ce9173904aa3bec7de78f0058b41f53fc01a8 (signed G). It was pushed directly without `make land`, on the operator's explicit authorization of 2026-11-09 ("bypass make land ... tests are being heavy under the load"). In `crates/bbt-mux/src/mcp/work_items.rs`, Reject now refuses tasks that are accepted or done, matching Report, Delegate and Reassign. The TaskAmendment model has always guarded Reject with `status < 6` (`crates/bboxy450-term/tests/formal_contracts/task_amend_model.rs:58-66`). The code now matches the model; the model itself is unchanged.

Evidence:
- **Targeted tests:** 18/18 pass.
- **Broken control:** with the guard removed by hand, `mcp_task_accept_requires_matching_zero_verification_and_declared_criteria_only` fails as intended.
- **`make land`:** red twice before the bypass, exit 2 both times.
  - Run 1 failed three dream_c1 tests on `resource_limit`.
  - Run 2 failed three different tests: two dream_c1 tests and `index::tests::sibling_build_writes_do_not_dirty_watch_or_trigger_reconciliation`.
- **Flakiness:** the same HEAD went 0/3 and then 3/3 on back-to-back runs at load average about 9. The diff does not reach dream_c1.
- **Rebase:** a no-op, because main had not moved, so the pushed source is exactly the tested source.
- **Not run:** clippy, and the dependents' `cargo check`.

**Next cut, recommended:**
- Make `TaskStatus` a single typed enum in bbt-proto with an `is_closed`/`is_live` table.
- Replace the literal lists in `workspaces.rs:357` (which accepts the invalid "assigned"/"active"), `knowledge.rs:494`, `e2e_checklist.rs:230`, `suppression.rs:223,240` (a Reconcile to `done` leaves delegate wakes pending), and the UI's `marks.rs`, `stream_dock.rs`, `composer/report.rs`, `swarm_card.rs` and `seat_row.rs`.
- Add a broken control in which Reject has no live-status guard.

The serde string form stays as it is (additive-only IR).

**Gate defect, separate:** the dream_c1 CPU-debt `resource_limit` tests and the index watcher test are load-sensitive and make `make land` nondeterministic on a shared box.

## Ranked inventory, risk descending

### Persistence and lifecycle with a likely live defect
| # | Machine | Where | Defect lead | Formal coverage |
|---|---|---|---|---|
| T1 | Task status strings | bbt-mux mcp/work_items.rs:1350-1473 | Reject accepted/done tasks (**confirmed; fixed 161ce9173**); callers disagree on open/closed | TaskAmendment* |
| D1 | Dream C1 `AtomicU8` run state | bbt-mux logic/dream_c1.rs:650-662, guard 1037 | Writes skip the forward-only `phase()` guard: registry/dream_c1.rs:2130,2182,2288; logic 6124-6140; COMPLETED can overwrite QUARANTINED | Journal abstract only |
| N1 | Native unlock attempt | bbt-mux mcp/journal.rs:63-74, 691 | `finish` does not check `started`/`position`, so completion can stay `None` forever | none |
| C1 | Seat checkpoint | bbt-mux logic.rs:2504-2510, checkpoint.rs:58-91 | Error arm at 8563 clears pending but not `Age.submitted` | none |
| R1 | Ring refusal backoff | bbt-mux logic.rs:2488-2489, 9035-9173 | One timestamp carries two meanings, so a Busy reply shortens a 2 s I/O hold to 1 ms; `ring_blocked` cache goes stale | SeatIngestion (abstracted) |
| P1 | Python execution status strings | harness python_execute/mod.rs:108, process.rs:88 | "running" overwrites a cancel; final write has no is-final check | harness-local model only |
| M1 | MCP server slot 0..3 int | harness mcp.rs:253-255, 779; relay.rs:140 | Cancel during handshake leaves it stuck at 0 forever | none |
| W1 | Window restore paint gate | term main.rs:2452-2455, 8244; live.rs:1196,1283 | No deadline; a clamped size or missing anchor leaves the window blank after relaunch | none |
| B1 | Backend connection | term main.rs:2285-2404, 5621, 9067 | BackendReady clears `backend_lost` with `live_commands=None`; guards inconsistent | none |

### Concurrency and UI coherence
| # | Machine | Where | Lead | Coverage |
|---|---|---|---|---|
| S1 | Supervisor seat termination (8 fields) | bbt-supervisor supervisor.rs:2348-2380 | Exited published only when 6 conditions hold together; companion `signal()` unowned at 2862 | partial |
| S2 | Capture writer pause flags | ring/writer.rs:44-69 | `storage_paused && !durable_paused` reachable (422, 1118) | CaptureStoragePressure |
| A1 | ACP agent startup/config/turn (11 fields) | harness backend/acp_agent.rs:165-235 | Config deadline expiry leaves `pending_model` set; late reply accepted | launch intent only |
| E1 | Engine profile change | harness engine/mod.rs:124-139, 832 | `profile_pending` not cleared while stopping | none |
| G1 | Gateway closed/faulted | harness gateway/mod.rs:68-72 | Two sources of "closed" | host.rs only |
| F1 | Focus ownership across surfaces | term main.rs:2309-2377, app_overlays.rs:858 | Two carets; stacked modals | key bit only |
| SP | Spawn placeholder | term starting.rs:24-38, 241 | Timeout sets `refused` but not `ambiguous` | unit tests |
| ST | Startup modal `error` sticky | term cockpit/startup.rs:60-113 | Failure modal shown on a healthy backend | implementation-only |
| PL | Plugin activation flags | bbt-plugin registry.rs:112-117 | `desired_enabled && needs_retrust` allowed | PluginInspection (other file) |
| PV | Provenance outbox / worker | bbt-mux provenance/engine.rs:73-77, worker.rs:236 | Republish on crash; inputs lost on failed `try_send` | partial |
| SR | Seat status reporter | harness frontends/seat.rs:137-174 | Duplicates harness_log.rs:288 with other names | twin covered |

### Lower risk (display or covered)
Composer mode string and image draft (bbt-ui composer.rs:74,212); `document_return` (main.rs:2337); `settings_attention` never cleared (main.rs:2328); `swarm_folder_pending` has no token (main.rs:2370); process timeout merged with cancel (harness process.rs:844); GUI frame `ready`+`error` (bridge.rs:149); bbt-rt command `pid`/`cancelled`/OnceLock; client `broken` latch; workspace scan `force`/`read` leftovers (routing.rs:415). Covered: dream bootstrap bools, Dream record, journal two-phase, harness turn_open, render arena, idle frame flags, selection, focus ack, attention.

## Pattern

The same shape repeats: a typed enum (or none), side flags next to it, string literals at the comparison sites, and no single `transition(state, event) -> Result<state>`. The general remedy, applied one machine per cut, has three parts:
1. One enum owns the state.
2. One function owns the transitions and refuses illegal ones.
3. The existing model's guards are mirrored in code, with a broken control.

Suggested order: T1 (next cut), D1, N1, M1, W1, P1, C1, B1.
