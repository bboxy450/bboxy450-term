---
id: 01M6F5SB0B6NZRCNRRV7P2W8VE
number: 5002226
type: task
title: 'Rich pane A3/A4: launch harness seats with --socket, connect and persist the session, deliver seat_input_text as an idempotent UserTurn over the socket (zero PTY bytes)'
status: open
role: executor
links:
  - { to: "[[app-harness]]", rel: implements }
allowed_paths:
- crates/bboxy450-harness-wake-up/src/launch.rs
- crates/bboxy450-harness-wake-up/src/guard.rs
- crates/bboxy450-harness-wake-up/src/lib.rs
- crates/bbt-mux/src/logic.rs
- crates/bbt-mux/src/mcp/seats.rs
- crates/bbt-mux/src/adapters/
- crates/bbt-mux/src/
- crates/bbt-proto/src/
- tests/
- scripts/
- Makefile
- verification/
- .bboxy/reports/
verify: make test-harness-native
created: 2026-11-02T23:55:00Z
updated: 2026-11-02T23:55:00Z
---
ROLE: EXECUTOR. Repo bboxy450/bboxy450-term, own worktree off origin/main. Spec: `.bboxy/interfaces/app-harness.md` "Phase 1b rich-pane amendment" (landed 898fc456) and the harness ICD `bboxy450-harness/.bboxy/interfaces/rich-pane.md` §2, §5, §6, §9 (main ≥ c4246de). This is Side A steps 3–4 only: **launch, connection, delivery.** No pane, no scene, no rendering (A1/A2 are a later task).

**Prerequisite.** Harness B1 has landed: `bboxy450-harness seat --socket <path>` binds the path, rebinds it after `/profile`, unlinks on exit. `make test-harness-native` (`Makefile:1894`) must run against a harness build that has it; pin how the test finds the binary and say which commit you tested against.

## Steps

1. **Launch kit.** In the `bboxy450-harness` branch of `crates/bboxy450-harness-wake-up/src/launch.rs:515`, append `--socket <path>`. The path is `<runtime dir>/h-<seat ulid>.sock`, where the runtime dir is the one that holds the supervisor control socket (same lifetime). Refuse the launch with a typed reason if the path exceeds 96 bytes; never truncate. On restart, when `harness_session_id` is persisted for the seat, also pass `--resume <id>`.
2. **Persist the session.** `crates/bbt-mux/src/logic.rs:6162` persists `harness_session_id` for claude and codex only. Add `bboxy450-harness`: the value is `Hello.meta.session` from step 3, updated on every new `Hello` (a `/profile` switch gives a new session on the same path).
3. **Connection.** One async task per harness seat (the shape of the clients in `crates/bboxy450-harness-wake-up/src/native.rs:460`, newline frames, bounded line size), never on the UI or logic thread: connect to the path, send `Subscribe` (harness gateway ICD §1a/§4; framing in harness `src/gateway/unix.rs`), read `Hello`, then envelopes and the 15 s heartbeat. State per seat: `Unbound` → `Connected{session, epoch, head, last_frame_ms}` → `Closed` → reconnect with backoff, at most one attempt per 2 s. A frame over 256 KiB closes the connection. Accept `Hello` with protocol major 0 or 1; refuse higher with a typed notice.
4. **Readiness and delivery.** In `guarded_input` (`crates/bbt-mux/src/logic.rs:8669`), dispatch on kind `bboxy450-harness` **before** the PTY arbiter and the composer precheck (`:8718`). Readiness is admission: `Connected` and `last_frame_ms` younger than 20 s. Deliver `OpEnvelope{ v: the Hello's protocol, op_id: the bboxy message ULID itself (deterministic, so a resend is a duplicate), session: Hello.meta.session, op: UserTurn{ blocks: [Text{text}], mode: Queue } }` and await the correlated ack for at most 5 s. Map to `input_result` (`crates/bbt-mux/src/mcp/seats.rs:476`):

   | Outcome | Result |
   |---|---|
   | `Ack` or `Ack{duplicate}` | submitted; receipt carries the op_id |
   | `Refused::Busy` | `not_submitted`, cause `harness-busy`, retryable |
   | `Refused::Closed`, `UnknownSession` | `not_submitted`, cause `harness-session-closed` |
   | `Unbound`, or `Connected` without `Hello`, or stale | `not_submitted`, cause `harness-socket-unavailable` |
   | ack timeout | `unknown`; the same op_id is resent on retry |

   **Zero PTY bytes in every row.** This path never reaches the PTY arbiter, `guard::check` (`wake-up/src/guard.rs:81`) or the `WakeMethod::Operator` pending row (`logic.rs:8633-8645`). The adapter record (`crates/bbt-mux/src/adapters/mod.rs:91`) keeps `StateSource::Cadence`; deriving state from `Status` frames is out of scope.
5. **Tests** (`make test-harness-native`, rich case; no operator, private HOME, scripted provider):
   - spawn with `--socket`; `Hello` observed; `harness_session_id` persisted;
   - `seat_input_text` → submitted → `TurnEnded` in the harness log; the log's `cause` equals the message ULID;
   - resend the same message → submitted (`duplicate`), exactly one turn in the log;
   - unlink the socket → `harness-socket-unavailable`; restore the seat → reconnect within 2 s;
   - `/profile echo` through the PTY face → a new `Hello` session on the same path; a resend of the earlier message → `harness-session-closed`;
   - detach and reattach the connection → durable seq list `1..=head`, dedup counter 0;
   - a byte counter on the seat's PTY writer proves **0 bytes** across every case above.
   - **Broken controls:** (a) a fresh op_id on resend → two turns → the test must fail; (b) the PTY fallback re-enabled for this kind → counter > 0 → the test must fail.
6. **Formal.** The model `HarnessSocketDelivery` lives in the harness (its brief). Here, add a `verification/contracts.toml` entry naming this test as the implementation witness, with the assumptions (one connection per seat, ack within 5 s or `unknown`, op_id = message ULID).
7. Run `make test` once on the final commit (one full gate at a time on this machine; the Oracle runs or reuses it).

**Report** in `.bboxy/reports/` with the harness commit tested against, each step's file:line and mechanism, `make test-harness-native` and `make test` exits, the broken-control outputs, and the PTY byte counter values.
