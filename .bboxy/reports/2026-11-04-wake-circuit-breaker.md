---
type: report
title: Automatic wake circuit breaker — landed, installed and verified
task: '[[bugs/2026-11-04-automatic-wake-storm]]'
commits:
- 5b6110d36a6c4a0c0315a70528b86c9d8bffe076
- 2628bae972e812bcec32a9e0d08b9cb692e26a89
- b3071eb976e971e89c6aae5de0a3cc2ce9d0b146
- 6e6d07146243c1af210128afe9164429ac5d942f
- 5e346de40cddfa10751fa9b828f7f63368fb6053
- b5424152b7ebf0e1b24fcb37bc35cb7040f1ae8c
- d79ab7f3f6c5a1f1e3efd238573c9e22834df716
- 301a5df8332ae2fea350f90e8096d723c364d165
- 5ffa80924e14a0f1b187364b5d8d6486319f88e1
- ebecca279040c0aea4528895c1c154a60c4ffe21
- 728ba585b438256e2cc7e5495a0a23baa82b4fe5
verify:
  command: make land
  exit_code: 0
author: seat:01M6MADHFD19P4TKM7P7X6QESA
created: 2026-11-05T00:35:55.640Z
id: 01M6MJ4ECG7Y7WK4TW0JAJFHWC
updated: 2026-11-05T00:35:55.640Z
number: 93377961363739
created_by: seat:01M6MADHFD19P4TKM7P7X6QESA
updated_by: seat:01M6MADHFD19P4TKM7P7X6QESA
authorship:
  signer: seat:01M6MADHFD19P4TKM7P7X6QESA
  kind: seat
  public_key: Rg4zwPSh/8Jytfz7nuhB5zfZyLtAZj9dFmnXz9d411U=
  process_generation: '1'
provenance:
  digest: sha256:4a859a7de12c85d68af9d4701bdf021ec364a9a561c4a19fc4889bd6e403bef6
  frontmatter_digest: sha256:111b6f051d723f44d463a885b70fe5c3a19e857966f2526bda761f0abe8308c7
  signer: seat:01M6MADHFD19P4TKM7P7X6QESA
  sig: TJ+ocq+ghtnbsbn5cfTEjFE29cYExDUsanieSDqbM3GkFke7BuWy989kWu8lmZbeHbOv29ydHrapSmeohWT7CA==
  log: .bboxy/log/2026-11-05.jsonl#131547
---
The automatic-wake circuit breaker is landed on main at `5b6110d36a6c4a0c0315a70528b86c9d8bffe076` and installed in the approved Apple Development signed `/Users/feral/Applications/bboxy450-term.app`. Its Info.plist names that source and harness `baf77427df00ce99d4a9a392ebc8bef7e1b5478c`. Running processes/sessions were preserved. Protection becomes active when the backend starts on the new binary; an app-bundle replacement alone does not replace a mapped backend.

Brief: [[notes/2026-11-04-wake-circuit-breaker]]. Finding: [[bugs/2026-11-04-automatic-wake-storm]]. Vesper had a finite backlog of 24 self notices: one delegation, 22 amendments, one report. The real API regression recreates them without inference.

Acceptance and mechanisms:

- `crates/bbt-mux/src/messaging/ledger/circuit.rs:52` — self-authored messages retain signed inbox records but never automatically wake their own seat, overriding wake policy. The bounded legacy sweep at line 144 also retires pending old self wakes.
- `crates/bbt-mux/src/messaging/ledger/circuit.rs:80` — recipient-local durable fixed-window budget: eight automatic native admissions per 60 seconds. Before a ninth, latch open and retain further automatic messages in the inbox. Time, restart, ACK, policy edits and replay do not rearm it. A newly admitted direct operator conversation resets it; operator traffic and other recipients remain independent.
- `crates/bbt-mux/src/messaging/ledger.rs:1025` — reserve credit and journal the attempt atomically before native IO. `ledger/circuit.rs:131` refunds only a proven pre-effect outcome in its original epoch. Unknown/possibly queued effects keep credit. The retained-through sequence fence prevents pre-trip backlog replay after rearm, including multiple 256-row suppression pages.
- `crates/bbt-mux/src/messaging/service.rs:3059`, `src/mcp/attention.rs` and receipts expose latch/count/limits and explain inbox retention/operator rearm.
- `crates/bbt-mux/src/messaging/service/circuit_tests.rs` — seven private Messaging/SQLite API witnesses cover the 24 self notices, eight/ninth admission, reopen, unknown versus pre-effect outcomes, recipient isolation, operator reset/replay, zero/rollback/rollover time, stale epochs and 300-row legacy self/peer backlogs.
- `crates/bboxy450-term/src/sidebar_chat.rs:48`, `src/main.rs`, `src/messaging.rs:1278` — own the task retry abort handle and cancel sleeping timers when their generation/pending owner retires. The paused Tokio witness at `sidebar_chat.rs:307` observes zero disposed-timer callbacks and exactly one current callback. Existing generation admission guards remain. `headless.rs:694` records only bounded event discriminants/source IDs on idle failure; this identified the stale TaskRetry callback without logging message content.
- `crates/bboxy450-term/tests/formal_contracts/wake_circuit_model.rs`, `task_panel_model.rs`, Makefile and `verification/{contracts,corpus,coverage}.toml` map four circuit safety families and timer disposal liveness to native witnesses and deliberately broken controls.

Commands and actual results:

- `make test-wake-circuit-breaker`: exit 0; seven circuit and 37 existing integration tests passed, one existing ignored. Mutating only the ninth-admission guard from >= LIMIT to > LIMIT caused three assertion failures (make exit 2 / nextest 100); restoring it passed. Log/source hashes: `.bboxy/runs/2026-11-04-wake-circuit-targeted.json`.
- `make test-task-panel`: exit 0; panel render, 30 messaging UI tests and paused timer callback witness passed. `make test-boundaries`: exit 0; 39 private process/ctl/shell/lifecycle tests passed, one existing signing test ignored. Evidence: `.bboxy/runs/2026-11-04-wake-circuit-idle.json`.
- `make land`: exit 0; strict all-targets Clippy, 493 app tests, 353 mux tests and 15 document corpus tests passed. Existing skips: 12 app, 10 mux. All eleven implementation/prerequisite commits have valid approved signatures.
- Native/Yarn/Java TLC within that gate: exit 0; all 387 models, 34 crates and 161 contracts matched complete states, labeled transitions and expected verdicts. Four circuit good controls pass (14 states/65 edges each); all four broken controls fail safety. Timer ownership passes (3 states/2 edges); its broken control fails liveness (2 states/2 edges). Source-bound evidence SHA256 `0da4d1d20ec441d27590f95be4520547b89ae4477749d4f72a6551d9df791421` is retained under the common Git directory's `bboxy-formal-evidence/`.
- `make smoke` on landed source: exit 0; private unsigned app/throwaway identity, windows=0, no operator input or inference. Ten-second idle: frames=0, callbacks=0 (10.005s); children reaped. The earlier failing probe had one TaskRetry callback; the ownership fix preserves the zero-callback assertion.
- `make check-formal-evidence`, `make test-licenses`, `make install`: each exit 0. Install strictly verified the approved signed bundle, retained previous Contents for rollback and preserved backend owners. Exact logs/hashes, installed executable hash and formal receipt: `.bboxy/runs/2026-11-04-wake-circuit-landed.json`. Raw logs/transcripts remain outside Git.

Required gate prerequisites were repaired without dropping assertions: malformed/missing independent formal inventory and check metadata; parent coverage for five existing harness contracts; test-only IO/paused-runtime lint annotations; duplicate Tab interpretation moved into the single keyboard adapter; a collapsible paint condition; focused GUI formal export using the full corpus manifest and failing ordinary tests without an environment variable. Focused GUI exports now use a dedicated variable/private temporary directory. Eight unresolved KB references were recovered from exact authenticated canonical bytes, subsequently also landed upstream independently; IDs/hashes are in `.bboxy/runs/2026-11-04-wake-circuit-record-recovery.json`.

Limits: finite circuit models use one recipient, limit two, two reset epochs and one possible native effect; production limit eight, persistence and larger backlogs have real API witnesses. Safety needs no fairness. Timer disposal assumes fair owner observation; cancellation cannot retract an already queued callback, so generation fencing remains. This bounds automatic messaging wakes, not dollars/tokens, slow loops or model-internal loops without messages. Full repository CI, visual parity and live inference replay were not run; canonical diff-scoped landing, process boundaries and private headless smoke were run. Verification describes final combined source, not per-intermediate-commit full-workspace checks.