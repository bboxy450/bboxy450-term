---
id: "64G3Y0X3B5YWGJ2MZ2PNHAFHH4"
type: "research"
number: 95
title: "kittens events — what the operator's harness does, what the seat socket takes"
created: "2026-10-21T20:47:34Z"
updated: "2026-10-24T00:10:38Z"
---
# kittens events — what the operator's harness does, what the seat socket takes

2026-10-21 · research subagent for Vesper Greywind <ORACLE> · source `/Users/feral/mydev/kittens` (Rust, read at HEAD on this machine; line numbers are from that checkout).

[[vision]] [[mux-daemon]] [[harness-adapters]] [[mcp-control-plane]]

Frame: [[vision]] «Seats are sockets» — every seat is a unix socket, input in, events out, anything glues. kittens is the operator's own event-based harness; this file records its mechanism with `file:line`, then designs the seat socket on top of [[mux-daemon]] generations and [[messaging]] evidence states. Sections 1–2 are read from source. Sections 3–6 are designed here and are proposals, not tested code.

## 1. What kittens does (file:line)

### 1.1 Event model — the bus

| Fact | Evidence |
|---|---|
| One in-memory async bus; one Tokio router task owns id allocation and the subscription table; every call is command + oneshot ack; nothing persisted, replayed or networked | `src/bus.rs:1-14`, `:334-340` |
| Route is exact: `Route::Member(name)` or `Route::Topic(name)`, tagged `{"kind","name"}`; no wildcard, no inherited delivery, slashes are not a hierarchy | `src/bus.rs:27-36`, `:23-25` (name ≤ 256 bytes), `.bboxy/docs/messaging.md:39-51` |
| Envelope = `{id:u64, from, route, in_reply_to:Option<u64>, correlation_id:u64, payload:Value}`; root message is its own correlation, a caused one inherits | `src/bus.rs:47-60`, `:393-404` |
| Fanout clones one envelope; id allocated once before fanout so observers never disagree | `src/bus.rs:406-420` |
| `subscribe` acks only after registration, so anything published after the ack cannot be missed; `emit` returns once queued for every current subscriber, not once acted on | `src/bus.rs:183-184`, `:373-374`, `:204-206` |
| Per-subscriber queue is **unbounded** on purpose; slow observer costs memory, no silent drop. No replay for late subscribers | `src/bus.rs:285-290`, `.bboxy/docs/messaging.md:85-95`, `.bboxy/docs/reactive.md:104-106` |
| `tap` observes every envelope (journal), outside the route table | `src/bus.rs:173-181`, `:345-352` |
| Drop of a `Subscription` unsubscribes; router prunes closed receivers at emit time | `src/bus.rs:320-326`, `:409-415` |
| Ack semantics: the emitter gets the minted envelope back; there is no delivery or consumption ack at the bus layer | `src/bus.rs:207-227` |

### 1.2 Event types

| Fact | Evidence |
|---|---|
| Agent input is a tagged enum `type`: `profile`, `prompt{text}`, `image_prompt`, `activate{text,context}`, `cancel{turn_id}` | `src/types.rs:91-112` |
| Agent output events, tagged `type`: `profile_changed`, `thinking_delta`, `text_delta`, `tool_start{id,name,arguments}`, `tool_end{id,name,output,is_error}`, `tool_changes`, `turn_end{result{text,stop_reason,usage}}`, `ready`, `error{message}` | `src/types.rs:206-244` |
| Exactly one terminal event per accepted prompt (`turn_end` or `error`); `ready` is nonterminal | `src/agent.rs:36-40`, `src/types.rs:238-240` |
| An agent's inbox is `Member(<name>)`; its output topic is `agent/<name>`; every output carries the prompt envelope as cause | `src/agent.rs:31-34`, `:1518-1527`, `:1616-1627` |
| Journal record (the persisted shape) = `{timestamp, uuid, sequence, envelope}`; sequence is assigned by the journal worker (index+1), not by the bus, written to `~/.kittens/logs/<uuid>.jsonl` plus SQLite | `src/registry.rs:64-70`, `:724-756` |
| Hooks are shell programs on `before_tool`/`after_tool`/`turn_end` from `.kittens/config.json`, 10 s budget | `src/hooks.rs:1-24`, `:36-54` |

### 1.3 Transports

| Transport | Direction | Evidence |
|---|---|---|
| **Unix socket**, newline-framed JSON, 1 MiB line cap, oversize frame drained through its newline so the connection recovers | input only (prompts) + request/reply (mux) | `src/socket.rs:1`, `:14`, `:168-198` |
| Prompt frame `{version:1, text, to?, from?}`; a line not starting with `{`/`[` is plain text = prompt to root | in | `src/socket.rs:15-24`, `:203-216` |
| Per-line ack `{version:1, id?}` or `{version:1, error}`; acceptance means queued, not completed | out (reply) | `src/socket.rs:26-32`, `:340-357`, `README.md` «Prompt a running session» |
| Mux control multiplexed on the same socket by `"type":"mux"` → `{version, type, request, caller?}` / `{version, result, error}`; client timeout 30 s | request/reply | `src/socket.rs:304-333`, `:391-447` |
| Socket hygiene: refuse an existing path (dangling symlink on macOS), bind, chmod 0600, remember dev/ino, remove only the owned entry on shutdown | — | `src/socket.rs:69-85`, `:130-146` |
| Path scheme: `/tmp/kittens-sockets/<harness-uuid>.sock`, dir made private; `--socket PATH` overrides; UUIDv4 per CLI runtime | — | `src/registry.rs:188-192`, `.bboxy/specs/harness-registry.md:5` |
| Liveness = pid alive **and** `connect()` succeeds; otherwise `stale/unreachable` | — | `src/registry.rs:236-245` |
| **Events do not come out of the socket.** They come out of `--headless` stdout: first `{version:1,type:"ready",root,socket,harness}`, then `{version:1,type:"event",envelope}` for `Topic(agent/<root>)` only | out | `src/cli.rs:195-245`, `:433-434` |
| **`--listen`**: JSONL over owned stdin/stdout. Request `{version:1, id≤128B, op: command|watch|unwatch, …}` with strict field sets; replies `{type:"reply",id,result}` / `{type:"error",id,message}`; validation never echoes input | bidirectional | `src/listen.rs:36-91`, `:109-115` |
| `watch` subscribes an exact route; the reply is queued **before** the observer task gets the receiver (ack precedes first event); 64 watches max; `unwatch` flips an `active` flag so already-queued events are suppressed after the unwatch reply | — | `src/listen.rs:23`, `:303-339`, `:93-98`, `:189-196` |
| Event record `{version:1,type:"event",watch:<id>,envelope}` — the complete unchanged bus envelope | out | `src/listen.rs:236`, `.bboxy/docs/listen.md:70-80` |
| Controller ops on either path: `start{name,spec}`, `emit{route,payload}`, `mux{request}`, `profiles`, `list`, `stop{name}` | in | `src/control.rs:44-52`, `:64-82` |
| ACP over stdio (`--acp`) — not read in depth | — | `src/acp.rs` |
| HTTP: none for the bus. The only HTTP in the crate is the outbound JEV decision client (bearer, 10 s, 64 KiB cap) | — | `src/jev.rs:7-10`, `:21-45` |
| Design note aimed at us: "An external adapter will have to define its own ordering/reconnect semantics; none is smuggled into this local contract" | — | `.bboxy/docs/reactive.md:52-53` |

### 1.4 Input injection into adopted harnesses (kittens' own "seats")

| Fact | Evidence |
|---|---|
| Owned PTY panes for codex/claude/pi/omp/kittens/argv; fresh kittens UUID per pane; one Host-owned mux manager with typed ops shared by TUI, CLI, model tool and ACP | `.bboxy/specs/adoption.md:7-12`, `:29-33`; ops enum `src/adoption.rs:30-140` (`input{pane,text,submit}`, `send{pane,text,message_id}`, `steer{…expected_turn_id}`, `interrupt`, `resize`, `focus`, `zoom`, `close`, `stop`, `cron_*`) |
| PTY submit = optional `ESC[200~` … `ESC[201~` bracketed paste then `\r`, in one writer transaction; "PTY write proves only written" | `src/adoption_terminal.rs:455-480` |
| Native first: Codex via owned app-server `unix://` + `thread/queue/add`; Claude via `--messaging-socket-path` JSONL `{"type":"user",…}` with **no write ack** (`WrittenReceipt`); never retry an ambiguous write | `src/adoption_codex.rs:1`, `:248-251`, `:453`; `src/adoption_claude.rs:70`, `:78-93`, test `:184`; `.bboxy/specs/adoption.md:23-28`, `:106-115` |
| Cron: `add{after_seconds|every_seconds|expression, command | target+message+to, count}`; firing = `{timer_id, message_id, target, message}`; delivery outcome vocabulary `Ran / BashFailed / AcceptedMessage / Sent / TargetExited / Failed / DeliveryUnknown`; "accepted-message does not mean completed harness work" | `src/cron.rs:10-24`, `:8`; `src/adoption_timers.rs:374-380`, `:449-471`, `:386` |

### 1.5 Auth, backpressure, disconnects

| Fact | Evidence |
|---|---|
| Auth is filesystem only: 0600 socket in a private dir, same UID. `from` on a prompt and `caller` on a mux frame are **caller-supplied** (default `"external"`) and not verified | `src/socket.rs:79`, `:282`, `:317`; `src/registry.rs:188` |
| Bus: unbounded queues (above). `--listen`: bounded mpsc(16) between parser and writer; the writer owns a complete `write_all` per record with no consumer deadline; slow stdout backpressures the whole request loop | `src/listen.rs:371`, `:432`, `:199-205`, `.bboxy/docs/listen.md:89-92` |
| Socket connections are independent tasks; peer close ends the task; listener cancel aborts all clients | `src/socket.rs:89-110`, `:336` |
| Journal persistence failure is recorded on the harness row (`logging_error`), lifecycle `failed`; harness rows and logs are kept after exit as archives | `src/registry.rs:757-765`, `.bboxy/specs/harness-registry.md:6-7` |
| Structured stderr diagnostics `{type, operation, actor≤256, detail≤512, io_kind, os_code}`, never credentials | `src/socket.rs:149-153` |
| `docs/computer.md`: Cua driver leases per app, cooperative, quarantine on unknown execution state. Not an event-bus concern; the only pattern worth noting is "a lease is released at a checkpoint, never by a kill" | `docs/computer.md:28-40`, `:63-70` |

## 2. What we take

1. **NDJSON, one object per line, 1 MiB cap, oversize drained.** `socket.rs:168-198`. Shell-gluable (`nc -U`, `socat`, `jq`); the length-framed protocol stays internal to supervisor↔logic ([[mux-daemon]] «Supervisor ↔ logic protocol»).
2. **A bare text line is an input.** `socket.rs:203-216`. `echo "status?" | bboxy seat send <id>` must work.
3. **Every frame carries a version; unknown fields are rejected; validation errors never echo the input.** `listen.rs:36`, `:84-86`, serde `deny_unknown_fields` throughout.
4. **Ack before first event; registration before ack.** `bus.rs:373-374`, `listen.rs:312-320`. A client that has its subscribe ack cannot miss what follows.
5. **Exact subscriptions, no wildcards.** `bus.rs:27-28`. Our filter is a list of exact `kind` names.
6. **Accepted ≠ completed, written ≠ consumed.** `README.md` socket section, `cron.rs:8`, `adoption.md:26-27`. Our ack and `input` receipt events carry [[messaging]] evidence states instead of a boolean.
7. **Socket hygiene**: refuse existing path, 0600, dev/ino identity, remove only what we bound. `socket.rs:69-85`, `:130-146`.
8. **Liveness is connect, not pid.** `registry.rs:236-245`. `bboxy seat ls` reports `live` only when the seat socket accepts.
9. **A `ready`/`hello` first frame** that names the endpoint and identity. `cli.rs:227`.
10. **Bounded structured diagnostics** on the server side. `socket.rs:149-153`.
11. **Cron delivery vocabulary** (`AcceptedMessage`, `TargetExited`, `DeliveryUnknown`) maps onto `recipient-stored`, `rejected: exited`, `native-unknown`.

## 3. What we do differently and why

| kittens | seat socket | Why |
|---|---|---|
| Input socket and event stream are two transports (socket in, headless stdout / listen stdio out) | **One socket per seat does both** | The vision's promise is "plug into a seat with nothing but a socket path". |
| Harness identity changes on resume (new UUID, new socket, `resumed_from`) — `SKILL.md` «Resume» | **Seat id is stable; `process_generation` moves** | [[mux-daemon]] «Durable seat identity»: PIDs are never identity, a resumed harness is a new generation of the same seat. A cron line written once must still point at the seat a week later. |
| No replay; late subscriber gets nothing (`bus.rs:285-290`) | **`since:{gen,seq}` replays from the retained ring/record log; a `gap` frame names what is missing** | Seats survive restarts and so do their clients' expectations. A reconnecting cron must not silently miss `done`. [[mux-daemon]] «Ring records»: record sequences already order non-output events. |
| Unbounded per-subscriber queues | **Bounded per-client queue; `state`/`lifecycle` coalesce latest-wins per key; `output` drops with a `gap` carrying ring offsets** | 1000 seats ([[vision]] «Persistence and scale»); a stuck `nc` must not grow logic memory. Same shape as the GUI's "one pending merged update per view" ([[mux-daemon]] «Logic ↔ GUI»). |
| `from`/`caller` are caller-supplied | **Actor derived from a broker-minted token; a `from` field is rejected** | [[mcp-control-plane]] «Caller, role and grants»: a caller-supplied role, name or env label is not authentication. |
| PTY submit writes paste+CR when asked (`adoption_terminal.rs:455-480`) | **Input never reaches the PTY without the adapter's `input_guard` and the input arbiter; native `wake` first** | [[harness-adapters]] §1 guard contract points 1–5; [[messaging]] «Harness delivery». The socket is glue, not a keyboard. |
| Bus envelope has `route`, `correlation_id`, `in_reply_to` | **Envelope has `seat_id`, `process_generation`, `seq`, `kind`; receipts carry `input_id`** | The seat *is* the route. We do not own the harness's turn, so we do not fake a causal chain through it; the only correlation we can prove is "this receipt is about that input". |
| Hooks are shell programs run by the harness | **Hook events are observations (`kind:"hook"`) that arrive via OSC 7502 / `mux.sock` and are re-emitted on the seat socket** | [[harness-adapters]] §4 already owns the shim. The seat socket republishes; it does not run programs. |

## 4. Seat socket design sketch

Everything here is designed in this file. Owner: the mux **logic** process ([[vision]] line 53); listeners return with logic after a restart, clients reconnect and read the new generation from `hello`.

### 4.1 Paths

```
<rundir>/                       $XDG_RUNTIME_DIR/bboxy450/ or the private state dir on macOS (HA §4 already uses this dir for mux.sock)
  mux.sock                      shim / extension inbound — unchanged, HA §4
  operator.token                0600, minted by the broker on GUI/daemon start; full scopes for the operator's own shell and cron
  seats/
    <seat_id>.sock              0600, one listener per seat, seat_id = ULID (HA §4 BBOXY_SEAT_ID)
    <seat_id>.token             0600, that seat's own actor token (its harness reads it via BBOXY_SEAT_TOKEN=<path>)
  http.sock | 127.0.0.1:<port>  optional, §4.7, off by default
```

Dir 0700. Listener bound only when the seat record exists in logic; removed on `closed_retained`/exit by dev/ino match. `bboxy seat ls` = list `seats/*.sock`, liveness by connect (kittens rule), prints `seat_id lifecycle gen harness role path`.

### 4.2 Framing

NDJSON, UTF-8, `\n` terminated, 1 MiB per line, oversize drained through the newline then an `ack{status:"rejected",error:"frame>1MiB"}`. Binary (raw output bytes) rides as base64 inside `payload` and only when the client opted in. First line in either direction is `hello`.

### 4.3 Envelope out (logic → client)

```json
{"v":1,"seat_id":"01J…","process_generation":7,"seq":4182,"ts":1760000000123,
 "kind":"state","in_reply_to":null,"payload":{…}}
```

- `seq` — per seat, per process generation, monotonic, assigned by logic from the same counter that orders non-output records in the ring ([[mux-daemon]] «Ring records»: "record sequences also order non-output events"). Resets with `process_generation`; a client keys replay on the pair.
- `in_reply_to` — the `input_id` this frame is a receipt for; `null` otherwise.
- `kind` — exact, closed set below; unknown kinds never appear to a client that did not subscribe to them.

| kind | payload | source |
|---|---|---|
| `hello` | `{seat_id, process_generation, supervisor_boot_id, lifecycle, lifecycle_revision, harness_kind, harness_session_id?, role, title, cwd, last_seq, retained:{from_seq,to_seq}, caps:[…]}` | first frame after a valid client `hello`; [[mux-daemon]] «Attach / Resume / Subscribe» fields |
| `state` | `{state: idle\|working\|waiting\|done\|error\|exited, waiting_kind?: permission\|question\|auth, detail?, grade: reported\|shim\|inferred, channel}` | [[harness-adapters]] §3a; 7501 vocabulary + `Grade` |
| `task` | `{agent_id, state, title?, ended: bool}` | 7501 `id=` reports → subagent rows, [[subagent-detection]] |
| `hook` | `{ev, d}` — the whitelisted 7502 JSON, `last_message` ≤ 2048 B | [[harness-adapters]] §4 |
| `title` / `cwd` | `{title}` / `{cwd}` | OSC 0/2, OSC 7 |
| `output` | `{from_offset, to_offset, bytes_b64}` or `{rows:[{index, text}], index_generation}` | ring bytes or semantic rows; opt-in only |
| `input` | `{input_id, evidence: recipient-stored\|native-queued\|native-written\|native-unknown\|application-acknowledged, native_id?, note}` | [[harness-adapters]] «Wake evidence mapping» R11; [[messaging]] «Delivery» |
| `message` | `{message_id, from_seat, priority, held: bool, evidence}` — metadata only, never body | [[messaging]]; body stays in the inbox store |
| `lifecycle` | `{lifecycle: starting\|live\|closed_grace\|closed_retained\|exited, lifecycle_revision, process_generation, deadline?, exit_code?}` | [[mux-daemon]] «Reversible close lifecycle» |
| `gap` | `{from_seq, to_seq, reason: evicted\|generation_changed\|slow_client}` | replay boundary or drop notice; never silently spliced |
| `ack` | `{req, status: accepted\|applied\|rejected\|unknown, error?, input_id?, result?}` | reply to a client request; vocabulary from [[mcp-control-plane]] «Schemas» |

Default subscription on connect: `hello, state, lifecycle, input, gap, ack`. Everything else is opt-in.

### 4.4 Frames in (client → logic)

```json
{"v":1,"op":"hello","token":"<hex>","client":"cron-status"}            first line, within 2 s or the connection is closed
{"v":1,"op":"subscribe","kinds":["state","task"],"since":{"gen":7,"seq":4000},"output":false}
{"v":1,"op":"input","req":"c1","text":"Status check: one line.","submit":true,"expected_process_generation":7,"priority":"next"}
{"v":1,"op":"key","req":"c2","key":"escape","expected_process_generation":7}
{"v":1,"op":"status","req":"c3"}                                          ack.result = hello body
{"v":1,"op":"screen","req":"c4","limit_rows":40}                          ack.result = screen_read result; scope "screen" required
{"v":1,"op":"unsubscribe"}
Status check: one line.                                                   bare text ≡ {"op":"input","text":…,"submit":true}
```

- `req` is a client string ≤ 128 B echoed in `ack`; reusable after its ack (kittens `listen.rs:39-41`, `.bboxy/docs/listen.md:54-55`).
- `expected_process_generation` is mandatory on `input`/`key`; mismatch → `ack{status:"rejected",error:"stale_generation"}` ([[mcp-control-plane]] «Schemas»: a target process mutation requires it).
- `submit` is explicit, never inferred (`seat_input_text` rule). `key` is from the finite catalogue of `seat_input_key`; no raw bytes.
- `priority` ∈ `next|later|now` (Claude vocabulary, [[harness-adapters]] §1); the broker clamps it to what the token's role allows.
- No `from`, no `role`, no `caller`. Rejected if present.

### 4.5 Input path through the guard

```
op:input ──► durable inbox record (recipient-stored) ──► ack{accepted, input_id}
         └─► adapter.attach() current?  ──yes──► adapter.wake(att, msg, prio) ──► input{native-queued | native-written | native-unknown}
                                        └─no──► adapter.input_guard(seat, arbiter)
                                                 Ready{token}      ──► supervisor WriteInput(input_id, bytes) ──► receipt ──► input{native-written | native-unknown}
                                                 NotReady(reason)  ──► input{recipient-stored, note:reason}   (stays queued; retried on next state change)
                                                 OperatorHandoff   ──► input{recipient-stored, note:"operator handoff"} + GUI pending badge
```

Rules carried verbatim from [[harness-adapters]] «Interactive-input guard — contract»: a ready indication is bound to seat + session + process start + attachment generation (1); native first, never type after an ambiguous native attempt (2); PTY fallback rechecks generation, foreground group, parser/render state and operator draft under the arbiter (3); literal paste in the advertised mode, journal `input_id` before writing (4); hookless harness → handoff, never a blinking-cursor guess (5). `application-acknowledged` is emitted only on the R11 evidence (`peer_idle_notice`, `turn/started`, extension echo), never on a quiet screen.

Socket writers are peers. The token's grant supplies role and destinations ([[identity]] `SeatGrant.role/destinations/control_scopes`); a worker-class token cannot claim `priority:"now"` or Oracle precedence ([[messaging]] Oracle decision on precedence).

### 4.6 Subscriptions, backpressure, disconnects

- One subscription per connection; `subscribe` again replaces it (ack first, then events, kittens ordering). ≤ 64 connections per seat (kittens `WATCH_LIMIT` precedent), 65th gets `ack{rejected,"too_many_clients"}` and close.
- Filters: `kinds` exact list; `output: true` enables `output`; `since` requests replay from retained records, answered by replayed frames then live frames, with a `gap` at any discontinuity (eviction, generation boundary). Without `since`, the stream starts live after `hello`.
- Per-client queue: 1024 frames or 4 MiB, whichever first. On overflow: `state`/`title`/`cwd`/`lifecycle` coalesce latest-wins per `(kind, key)`; `task` coalesces per `agent_id`; `input`/`ack`/`message`/`hook` are never dropped (they are small and are the evidence); `output` is dropped and a single `gap{reason:"slow_client", from_seq, to_seq}` is queued so the client can `since`-replay from the ring.
- Disconnect drops the subscription; accepted inputs still complete (kittens bus-tool rule "cancellation is not rollback", `src/bus_tool.rs:22-28`) and their receipts are retained in the ring for replay.
- Logic restart: listener gone → client reconnects → `hello` shows the same `process_generation` if the supervisor kept the child, a new one if not; the client treats a changed generation as a hard boundary and never splices `seq` across it ([[mux-daemon]] «Restart and reattachment»).

### 4.7 HTTP mapping (optional, off by default)

Local only; remote stays on the authenticated Quinn mux connection ([[mcp-control-plane]] «Boundary and transport»). Same frames, no new semantics:

| HTTP | Socket op |
|---|---|
| `GET /seats` → JSON list | `bboxy seat ls` |
| `GET /seats/<id>` | `status` |
| `GET /seats/<id>/events?kinds=state,task&output=0` as `text/event-stream`; `id: <gen>.<seq>` on every event; `Last-Event-ID` ≡ `since` | `subscribe` |
| `POST /seats/<id>/input` body `{text, submit, expected_process_generation, priority?}` → `ack` JSON | `input` |
| `POST /seats/<id>/key` | `key` |
| `Authorization: Bearer <token>` | `hello.token` |

### 4.8 Auth

- Gate 1: `LOCAL_PEERCRED`/`SO_PEERCRED` uid must equal the daemon's uid.
- Gate 2: `hello.token` is a 32-byte random capability minted by the broker ([[mcp-control-plane]] «Caller, role and grants»), stored 0600 in `<rundir>/seats/<seat_id>.token` (the seat's own actor) or `<rundir>/operator.token`. The token is an **actor** credential, not a per-socket password: connecting to seat B with seat A's token succeeds iff A's grant lists B (or B's swarm) in its observe/input destinations. Scopes: `observe`, `input`, `screen`; the default seat token has `observe,input`; `screen` must be minted explicitly (`bboxy seat token <seat> --scope screen`).
- Tokens are rebound per process generation of the *holder* (a resumed harness gets a rewritten `<seat_id>.token`); old tokens stop working, same as MCP bridge capabilities.
- Same-UID is not isolation; a process that can read the token dir can impersonate. This is the standing Gap from [[mcp-control-plane]] and [[messaging]], not a new one.

## 5. Glue examples

Paths abbreviated: `R=$XDG_RUNTIME_DIR/bboxy450`, seat ids are ULIDs. `bboxy seat …` is the thin CLI wrapper proposed above (kittens `--send` precedent, `socket.rs:361-389`); the raw form shows what it does.

**5.1 cron pokes a seat** — every 30 min ask the Executor seat for a one-line status, through the guard, and log the receipt evidence.

```sh
# crontab
*/30 * * * * bboxy seat send 01JEXEC… "Status: one line, blockers only." --token "$R/operator.token" >> ~/.local/state/bboxy450/cron-poke.log 2>&1

# what `bboxy seat send` does on the wire
{ printf '{"v":1,"op":"hello","token":"%s","client":"cron"}\n' "$(cat "$R/operator.token")"
  printf '{"v":1,"op":"status","req":"s"}\n'                       # read current process_generation from ack.result
  printf '{"v":1,"op":"input","req":"i","text":"Status: one line, blockers only.","submit":true,"expected_process_generation":7}\n'
  sleep 2; } | socat - UNIX-CONNECT:"$R/seats/01JEXEC….sock" | jq -c 'select(.kind=="ack" or .kind=="input")'
# → {"kind":"ack","payload":{"req":"i","status":"accepted","input_id":"01JIN…"}}
# → {"kind":"input","in_reply_to":"01JIN…","payload":{"evidence":"native-queued","native_id":"q-42"}}      (Codex)   or
# → {"kind":"input","in_reply_to":"01JIN…","payload":{"evidence":"recipient-stored","note":"NotReady: permission prompt open"}}
```
`accepted` is admission, the `input` frame is the evidence; the log line shows which, exactly as kittens' cron distinguishes `AcceptedMessage` from completion.

**5.2 a script tails a seat's state** — alert when an Oracle seat has been `waiting` on a permission for more than 60 s.

```sh
bboxy seat tail 01JORAC… --kinds state --since latest |
jq --unbuffered -r 'select(.kind=="state") | "\(.ts) \(.payload.state) \(.payload.waiting_kind // "-") \(.payload.grade)"' |
while read -r ts state kind grade; do
  if [ "$state" = waiting ] && [ "$kind" = permission ]; then t0=${t0:-$ts}; else t0=; fi
  [ -n "$t0" ] && [ $((ts - t0)) -gt 60000 ] && osascript -e 'display notification "Oracle seat waiting on permission" with title "bboxy450"' && t0=
done
```
`--since latest` is `subscribe` without `since`; `--since 7.4000` becomes `{"since":{"gen":7,"seq":4000}}` and the script sees a `gap` frame if the ring no longer holds that range. `grade` lets the script ignore `inferred` states if it wants proof-grade signals only.

**5.3 a seat subscribes to a sibling** — the Oracle seat's harness watches its Executor and pokes itself when the Executor finishes.

```sh
# run from inside the Oracle seat; BBOXY_SEAT_ID / BBOXY_SEAT_TOKEN are in its environment (HA §4)
EXEC=01JEXEC…
bboxy seat tail "$EXEC" --kinds state,lifecycle --token "$BBOXY_SEAT_TOKEN" |
jq --unbuffered -c 'select((.kind=="state" and (.payload.state=="done" or .payload.state=="idle")) or (.kind=="lifecycle" and .payload.lifecycle=="exited"))' |
while read -r ev; do
  bboxy seat send "$BBOXY_SEAT_ID" "Executor $EXEC reports $(jq -r '.payload.state // .payload.lifecycle' <<<"$ev") at gen $(jq -r .process_generation <<<"$ev"). Verify its worktree before trusting the report." --token "$BBOXY_SEAT_TOKEN"
done
```
Authorization comes from the Oracle seat's grant listing the Executor as an observe destination; the same loop with a worker-class token pointed at the Oracle seat gets `ack{rejected,"not_permitted"}` at `hello`. The self-poke lands through the Oracle seat's own guard, so it waits if the Oracle is mid-tool. This is the bus-level shape of "MCP is one client of the same event bus" ([[vision]] line 53): a future `seat_watch` MCP tool is this loop without the shell.

## 6. Gaps

- **Gap:** fd budget — 1000 seat listeners plus clients on macOS (default `ulimit -n` 256) — no measurement; may force one multiplexed socket with a `seat_id` field instead of one listener per seat. Decision deferred to the [[mux-daemon]] implementation.
- **Gap:** whether `seats/<id>.sock` and `mux.sock` (HA §4 shim inbound) merge into one listener — no data; kept separate here because the shim path is already specified.
- **Gap:** replay across a generation boundary — serving old-generation frames on `since` with a `gap{generation_changed}` marker is proposed, unproven against the ring's checkpoint eviction rules.
- **Gap:** `output` kind byte rate at 1000 seats with several tailing clients — unmeasured; default-off is the only mitigation specified.
- **Gap:** HTTP/SSE — no identified consumer; stays off until one appears.
- **Gap:** token custody under same-UID — identical to [[mcp-control-plane]]/[[messaging]] standing gap; no new mitigation here.
- **Gap:** per-harness behaviour of a socket `input` while the harness is `working` (Claude queue vs Codex `thread/queue/add` vs pi/omp) — the R11 table covers wake evidence; which op the adapter chooses for a plain socket prompt is per-adapter and untested.
- **Gap:** kittens `src/acp.rs` (ACP stdio) and `src/tui/mux/*` were not read; ACP may carry a second event schema worth comparing. Not needed for this design.
- **Gap:** ssh-forwarding a seat socket (`ssh -L local.sock:remote.sock`) works mechanically but moves the token over the forward; remote seats are specified to go through the Quinn mux connection instead, and nothing here tests the forward.
