---
id: 01M5JGSHMT3P6DBTEEJNP62RFC
type: spec
title: C1 — live seats: the app becomes a client of logic
status: decided
links:
  - { to: "[[2026-10-22-c-cockpit]]", rel: epic }
  - { to: "[[architecture]]", rel: as-built }
  - { to: "[[2026-10-22-architecture-round-1]]", rel: decisions }
  - { to: "[[runtime]]", rel: constrains }
  - { to: "[[mux-daemon]]", rel: spec }
  - { to: "[[harness-adapters]]", rel: spec }
  - { to: "[[layering]]", rel: invariant }
created: 2026-10-22T19:18:18Z
updated: 2026-10-22T19:18:18Z
---
# C1 — live seats

Owner: Corvin Duskmantle <ARCHITECT>. Decided 2026-10-22. Binds decisions #1, #2, #6, #10, #11 of [[2026-10-22-architecture-round-1]]. Built on what [[architecture]] §1–4 shows exists: supervisor, logic, rings, snapshot and delta streams, close grace, launch overlays, all of it reachable only from tests today. **Done means** the seat half of the cockpit sentence in [[2026-10-22-c-cockpit]]: open the app, ⌘T a zsh, Claude or Codex seat, type, see live cells, see state in the sidebar, resize, scroll back, close and undo, kill the supervisor and watch every seat come back.

Lens: three processes on one Mac. Assumes macOS 14+, one account, local seats. Breaks when a seat lives on another host; [[2026-10-22-c-cockpit]] D1 adds the link, not the shape. LESS IS MORE: every piece below already exists or is the smallest thing that joins two existing pieces.

## 1. Shape

```mermaid
flowchart LR
  subgraph APP["app process: bboxy450-term"]
    UI["UI thread<br/>winit, layout, draw, present<br/>never waits"]
    IO["IO task on bbt-io<br/>owns gui.sock<br/>Replica per watched seat<br/>builds the next Model"]
    IO -- "Latest of Arc Model, swapped at frame start" --> UI
    UI -- "UiMsg: input, resize, watched set" --> IO
  end
  subgraph LOGIC["logic process: bboxy450-term logic"]
    GUI["gui endpoint<br/>seat-socket framing and admission"]
    VS["ViewStream per watched seat<br/>one Update in flight"]
    GUI --- VS
  end
  subgraph SUP["supervisor process: bboxy450-term supervisor"]
    PTY["PTYs, rings, leases, close grace"]
  end
  IO <-- "gui.sock, NDJSON" --> GUI
  LOGIC <-- "control socket" --> SUP
  APP -- "spawns and restarts (#11)" --> LOGIC
  APP -- "spawns and restarts (#11)" --> SUP
```

- **Processes.** The app spawns `bboxy450-term supervisor` and `bboxy450-term logic` as private entry points of the one binary, like `hook` (specs/README S5). The control socket lives at `default_socket_path()` (`M/transport.rs:118-131`, its first caller); the run dir under Application Support. Each process owns its own bbt-rt runtime, which closes the two-runtimes gap in [[architecture]] §8 by never sharing a process. Decision #1 moves the supervisor into `bbt-supervisor` after W2.4; C1 does not wait for that split.
- **One GUI endpoint on logic**, `{root}/gui.sock`, with the seat socket's framing and admission (`M/seat_socket.rs:19, 139-175, 213`: NDJSON ≤ 1 MiB, version, capability) and a GUI action set: `ListSeats`, `Watch{seat, view}`, `Unwatch{seat}`, `Applied{seat, view, sequence}`, `Input{seat, id, bytes}`, `Resize{seat, cols, rows}`, `Spawn{spec}`, `ScheduleClose{seat}`, `UndoClose{seat, close_id}`, `GetHistory{seat, from, to}`. One connection multiplexes every seat, so 1000 seats cost one fd. Unwatched seats push only `seat/{id}/state` events over the same connection (the endpoint's bus filter, `seat_socket.rs:224`, widened to `seat/*/state` for the GUI); no grids.
- **`bbt-proto` is born here** (decision #6): `View`, `Screen`, `Snapshot`, `Delta`, `Update` from `M/streams.rs`, `SeatRecord` from `M/registry.rs`, the GUI action and reply enums. It depends on bbt-ui-model for ids until O2 moves ids down. Order becomes `… bboxybook bbt-proto bbt-mux bboxy450-term` in the same commit ([[layering]]).

## 2. Binding (decision #2)

```mermaid
sequenceDiagram
  participant UI as UI thread
  participant IO as IO task
  participant L as logic
  participant S as supervisor
  UI->>IO: UiMsg::Input{seat, bytes}
  IO->>L: Input{seat, id, bytes} (operator input, no guard)
  L->>S: Action::Input (dedup by id + SHA-256)
  S-->>L: Event::Output seq
  L->>IO: Update::Delta rows
  IO->>IO: Replica::apply, new Arc Screen for that seat
  IO->>UI: publish Model, dirty bit, one Wake
  UI->>UI: take latest Arc, draw only panes whose Screen changed
  IO->>L: Applied{seat, view, sequence}
```

- The IO task holds a `Replica` per watched seat (`M/streams.rs:187`, first production use) and the back `Model` (`UM/model/window.rs:117`, first production use). An applied Update yields a new `Arc<Screen>` for that seat only; the Model holds Arcs, so publishing a new Model is cheap: `Latest<Arc<Model>>` (`R/queues.rs:63-84`, force_push) plus a dirty bit. The UI thread takes the latest Arc at frame start; the previous Arc stays valid while the IO task builds the next. No lock, no wait, no grid clone. That is the double buffer.
- **Backpressure is the ack.** Logic sends one Update per (seat, view) and waits for `Applied` (`M/streams.rs:121-178`, exists). The IO task acks after it has published, never before; a slow app gets fewer, larger deltas and logic coalesces to the latest state. The UI's pane set decides `Watch` and `Unwatch`; the IO task sends them when the set changes.
- **UI to IO** goes through the existing bounded `UiMsg` queue (`R/lib.rs:71-89`); producers never block. A full queue hands the message back (`R/queues.rs:44-48`) and the UI retries next frame; input is never dropped.
- **Operator keystrokes bypass the input guard.** The guard (`M/adapters/guard.rs:36-78`) is for injected text (S3, [[harness-adapters]]); a human typing into a focused pane goes straight to the PTY. Retries are safe through the supervisor's id and SHA-256 dedup (`M/supervisor.rs:1083-1170`).
- **Resize** clamps in logic to 512×256 instead of ending the logic thread ([[architecture]] §8, `M/logic.rs:966-967`).
- **Drawing.** A pane draws `Screen` rows through the cells pipeline (`RD/cells`). Glyph coverage beyond ASCII, shaping and fonts are T3 in [[2026-10-22-t-terminal-fidelity]], not C1. Damage: redraw only panes whose `Arc<Screen>` pointer changed.

## 3. Restart and resume (decision #11)

```mermaid
flowchart TD
  X[supervisor or logic exits] --> A[app sees the child exit]
  A --> B{which one?}
  B -- logic --> C[restart logic: AcquireLogic, re-attach; seats never notice]
  B -- supervisor --> D[restart both]
  D --> E{seat record has a harness session id?}
  E -- yes --> F["launch resume_command: claude --resume, codex resume"]
  E -- no --> G[spawn a fresh zsh]
  F --> H[ring replayed into Playback for scrollback]
  G --> H
  H --> I["row shows the restarted notice; pane shows the gap marker"]
```

- The app owns both children and watches their exit. Logic death is already proven in tests (`M/supervisor.rs:527-547` lease re-acquire); C1 makes the app do it. Supervisor death: every seat record with a harness session id resumes through `resume_command` (`M/adapters/mod.rs:192`, first production caller); plain zsh respawns; the ring replays for scrollback; the row carries the `restarted` signal ([[ux-prototype]]). Unflushed bytes are lost and that is accepted; `Screen.gap` already carries the marker.
- The GUI endpoint root changes per logic instance (`M/logic.rs:486-497`); the app learns it from one JSON line logic prints at startup, never from a fixed path.

## 4. Scrollback, close, undo

- `GetHistory{from, to}` pages (`M/seat_socket.rs:30-58`) render lazily when a pane scrolls above its viewport; never prefetched.
- `ScheduleClose` and `UndoClose` are the supervisor's (`M/protocol.rs` Action, `M/supervisor.rs:758-795`); the GUI endpoint forwards them; 60 s default ([[vision]] undo close).

## 5. Gates before C1 is accepted (decision #10)

| Gate | Fixture | Measure | Budget |
|---|---|---|---|
| memory per seat | 1000 headless seats under the T1 firehose, none watched | RSS of supervisor + logic | set by the first run, recorded here |
| bus fan-out | 1000 seats, an Oracle burst of 30 messages at 10/s | p99 delivery with the weighted fair queue | set by the first run |
| supervisor fds and poller | 1000 PTYs under the firehose, open-file limit raised at start | p99 record write latency | set by the first run |
| sidebar and minimap | 1000 rows, 50 watched panes | frame time under the hitch meter, zero hitches idle | 8.33 ms at 120 Hz |

A budget is a number written into this table by the first measurement ([[runtime]]), then it binds. Measured, not guessed.

## 6. Invariants and acceptance

Invariants, each with its command:
- The UI thread never waits on IO: `sh scripts/ui-lint.sh` runs in CI from the first C1 commit (today it never runs), and the `--smoke` idle probe runs in CI expecting 0 frames idle.
- Layering: `sh scripts/layering.sh` with `bbt-proto` in the order.

Acceptance, the spec signs off:
1. `bboxy450-term` starts, spawns supervisor and logic; ⌘T opens a zsh seat; `echo hi` typed into it appears in the pane. Test: an integration test drives the IO task against a real logic and supervisor with a zsh seat and asserts `hi` in the Replica's `Screen`.
2. `kill -9` logic: output continues after restart. `kill -9` supervisor: the zsh seat respawns, a Claude seat resumes with `--resume`, the restarted notice shows. Test: the same harness, both kills.
3. The four gate rows in §5 hold measured numbers.
4. `cargo test --workspace --locked`, `sh scripts/ui-lint.sh` and `sh scripts/layering.sh` exit 0 in CI.

## 7. Out of scope

MCP and the per-seat wiring kit (W2.5, W2.5b), messaging in the UI (W2.4), providers (D1), fx (W3.3), theme as data (#5, O2), moving ids out of bbt-ui-model (O2), text beyond ASCII (T3).

## 8. Order for briefs (the Oracle's call, one concern each)

C1a processes and the GUI endpoint: spawn, startup handshake, `ListSeats`, state bits live in the sidebar, no grids. C1b watched grids: `bbt-proto`, Replica in the IO task, the Arc Model swap, cells drawn, input, resize. C1c restart and resume, scrollback, close and undo. C1d the four gates with their fixtures.
