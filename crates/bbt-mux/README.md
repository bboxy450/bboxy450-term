# bbt-mux — supervisor and restartable logic

W2.1 owns PTYs; W2.2 adds durable objects, the seat registry, the repository
index, the bus and generation-bound viewport sockets. W1.2 adds supervisor-owned
byte rings, committed parser/model checkpoints and retained history. `Logic::start` accepts a
W2.1 in-process client or `logic::SocketSupervisor` for a separate process. It
receives the application's existing Tokio handle; production code creates no
runtime. Bootstrap, database work and terminal parsing belong to IO owners,
with debug guards against calling their blocking entry points on the UI thread.
The UI uses bounded asynchronous `LogicClient` messages.

## Ownership and modules

| Module | Mechanism |
|---|---|
| `protocol`, `supervisor`, `transport` | One supervisor thread and one mio poller own PTY masters, children, resize, writer leases and close deadlines. Bounded u32-BE length/versioned JSON is the supervisor IPC. |
| `logic` | One logic IO worker attaches owned children, restores terminal checkpoints, captures ordered output and hosts seat endpoints. A supplied runtime handle runs the bus and asynchronous sockets. |
| `store/sqlite` | Per-object SQLite WAL/FULL databases, version 1, key/value, SQL and alarms. Connections are operation-scoped rather than held open for every object. |
| `store/blobs` | SHA-256 addressed, zstd-compressed local blobs, fsynced temporary file and rename, digest verification and a 64 MiB decompression bound. |
| `store/events` | Immutable, Ed25519-signed, hash-chained event rows; permanent event-ID deduplication and conflict rejection; chain verification on reopen. |
| `ring` | CRC32C framed capture, fsynced committed boundaries, checkpoint-aware hot retention, cold blobs, isolated replay and logical-row indexes. |
| `registry` | Account-owned ULID seat records, signed-role shape, location, mute, harness session and task references, close revisions and exact restore positions. |
| `index` | FSEvents/inotify invalidation, Markdown/YAML parsing, weighted wikilinks, SQLite mirror and durable event outbox, five petgraph projections. |
| `bus` | One bounded router over per-object event streams, consumer policies, replay, coalescing, explicit acknowledgments and request/reply. |
| `streams`, `seat_socket` | Acknowledged snapshot/delta baselines, generation validation, bounded NDJSON admission and explicit unavailable-history ranges. |

## Supervisor and input boundary

`Supervisor::start` owns native portable-pty children. `LaunchSpec` carries the
program, arguments, cwd and launch environment overrides. The supervisor sets
`BBOXY_SEAT_ID` and `BBOXY_PROCESS_GENERATION` last. One actor polls every master
fd and child exit; no per-seat PTY reader/writer/wait thread is created.

An owner capability admits a logic connection. A new logic lease fences the
old one; an attachment binds a writer to that connection, seat and process
generation. Exact launch retries return the original child; an explicit
replacement increments the generation. Output, resize and exit share the
supervisor's ordered per-seat cursor. Low-level input IDs retain digest/receipt
state across logic replacement. An in-flight write completes or becomes
`PartialUnknown`; it is never automatically replayed. The lease proves neither
composer readiness nor application acknowledgment. Seat observation sockets
expose no input operation: adapters and the guarded common writer are W2.3/W2.5.

The supervisor's optional Unix listener uses a private owned 0700 parent, 0600
socket and a random admission capability. Frames are capped at 1 MiB before
allocation. Existing endpoints are never stolen; cleanup checks socket inode.
W2.1 bounds remain: 16 KiB output chunks, 64 KiB inputs, 128 queued channel
messages, 4 MiB encoded socket output, eight logic clients and at most 1024
seats. Backpressure stops PTY reads rather than accumulating an unbounded tail.

## Durable lifecycle and crash boundary

The registry separates open/closed-grace/closed-retained lifecycle from harness
activity. Closing records a durable intent, the default 60-second deadline and
split position before asking the supervisor to schedule termination. The
supervisor owns the monotonic timer, so killing logic does not extend grace.
Undo must match process generation, close revision and layout revision; an
expired, replaced or conflicting seat is not silently restored. Reconnecting
logic reconciles exits and re-submits any outstanding close intent.

The supervisor captures output/resize/exit into each seat generation's ring
before delivering it. Logic produces the complete parser/model checkpoint on
the supplied IO runtime; a fresh logic instance restores the newest committed
checkpoint and replays captured records through a fixed boundary before serving
its consolidated viewport. It neither respawns the child nor re-emits old PTY
replies, clipboard effects or messages. A legacy W2.2 checkpoint can seed a new
ring with an explicit pre-ring history gap. Supervisor death still ends its PTYs;
a new supervisor refuses to treat an old generation's ring as a continuing child.

### Ring format, retention and recovery (W1.2)

`LogicConfig.ring` sets the active hot-file budget (default 4 MiB), checkpoint
output interval (64 KiB) and idle delay (200 ms). Checkpoints also follow resize,
exit and at most 256 new capture records, bounding index publication for tiny
writes. Startup admits a private ring for the seat generation before capture,
then commits its initial checkpoint. Capture continues while logic is absent.
W1.2b decouples capture from checkpoint progress. At the hot cap the supervisor
seals the complete CRC-framed file into a content-addressed cold blob and starts
a new bounded hot file. It never waits for logic or a new checkpoint to keep
reading. A full notification queue retains only the latest event; logic hydrates
the skipped capture sequences on resume. The ring-less W2.1 primitive still has
its original bounded queue behavior; production logic configures the ring.

A record has a fixed 56-byte little-endian header: magic/version/kind, payload
length, capture sequence, process generation, seat ULID, byte start and CRC32C
(Castagnoli, over header excluding the CRC field plus payload). Output payloads
are raw bytes; resize/exit are typed JSON; checkpoints are zstd-compressed,
versioned full parser/model snapshots with cursor, both screens, palette, title,
CWD metadata and the logical index. Checkpoint records refer to their exact
last-applied capture cursor and do not invent a new capture event. Header,
length, cursor, byte offset, generation and checksum are validated before replay.
Records are capped at 1 MiB; checkpoints at 128 KiB compressed / 16 MiB decoded;
viewport dimensions and logical index entries are bounded too.

Capture writes complete records and exposes `captured_sequence` separately from
the durable `committed_sequence`. Output group commits occur after 64 KiB or on
the next supervisor tick after 20 ms; resize, exit, checkpoint, sealing and
supervisor shutdown flush immediately. A flush orders data `sync_all` before the
CRC-protected marker and marker/directory sync. Capture-owner death may discard
the uncommitted group; logic restart while the supervisor lives does not.
The independently valid previous marker remains. Rotation writes a new file
first and publishes its marker before retiring
anything. Recovery rejects uncommitted or torn suffixes, falls back to the
previous valid boundary and reports the recovered cursor; suffix bytes go to
cold storage before truncation. Invalid committed data without a valid previous
boundary is an error, never a blank recovered screen. Strict media/power-loss
ordering (`F_FULLFSYNC`) and real power-cut testing are not claimed.

The hot budget includes all encoded records. Sealed uncheckpointed segments use
the exact same framing. A content-addressed descriptor links each segment to its
predecessor and records capture/byte bounds, identity and data digest; the
committed marker references only that durable chain head. Payload and descriptor
are synced before publishing the new hot-file boundary. An orphan descriptor
cannot enter a committed chain accidentally. The newest two checkpoint blob
references remain independently reachable even when their framing is cold.

Recovery validates the descriptor chain, then hydrates and validates segments on
demand through the same CRC reader as hot records. One decoded segment is cached
(up to the configured hot-file size); no output-sized memory queue is created.
History pages can span hot and cold without a synthetic eviction gap. Bootstrap
and sequence-jump recovery coalesce intermediate checkpoint writes into one at
the recovered tail; ordinary live output retains its configured checkpoint
interval. Repeating an interrupted replay remains safe because sealed raw bytes
stay retained. A genuine
missing/out-of-range capture prefix still has an explicit gap. Pre-W1.2b archived
prefixes without a committed descriptor chain remain outside that guaranteed
range. Logical row metadata may be unavailable when not in the currently loaded
checkpoint/segment index; raw records are still replayable.

`capacity_bytes` bounds the active hot file, not retained cold storage. The
previous boundary can pin another hot file of that size, plus a temporary file
while rotating. Cold storage follows never-delete, with no total-cold ceiling in
this version. Introducing such a ceiling must record a durable gap and continue
draining the PTY; it cannot pause the agent. Physical storage failures/power loss
remain outside the validation claimed here. Sealed descriptors grow with history;
only one segment's bytes and bounded hot/checkpoint indexes are cached. Files stay
in private runtime state, never Git. Legacy orphan hot files are preserved into
cold storage before retirement.

The semantic index maps capture sequences to cumulative **logical hard-line**
ranges: parser-observed LF/VT/FF and IND/NEL escape operations, excluding bytes
inside OSC payloads. These are not rendered/reflowed physical rows. Resize,
alternate-screen changes, clear and reset advance a separate view epoch.
Each parsed record receives an index point; checkpoint publication makes those
points durable. The supervisor's not-yet-parsed tail explicitly has no row
index. W1.2 does not infer missing line counts from raw byte sizes.

## Event bus and repository index

Each object stream has its own `events.sqlite` and sequence. Subjects use `/`
tokens, `*` for one token and terminal `>` for one or more remaining tokens,
with 256-byte/16-token limits. `inbox/>` and `seat/*/output` are live fan-out
only (sequence zero); all other subjects append before delivery. Supplied leaf
keys sign records; verification establishes integrity, not account-grant trust.
The identity/grant layer must authenticate and authorize producers/consumers.

Consumers may be ephemeral or named/durable, start new/all/last-per-subject/
sequence/time, and use no acknowledgment or explicit acknowledgment. Live
queues are bounded (1–4096); state-like subjects coalesce and overflow produces
an explicit gap. Explicit deliveries use persistent pending rows, acknowledgment
waits and bounded redelivery counts with the original event IDs. Reaching the
redelivery limit retains the pending row for operator policy. `reply_to` inboxes
receive `no_responders` when no consumer matches. Durable event retention/cold
migration and external wake/webhook/network delivery are later layers.

Supervisor output, resize and exit, mux spawn and swarm join are producers.
The shared bus handle accepts adapter state and hook events; native observation
and composer readiness are left to W2.3. A real watched document edit produces
by-ID and by-path events; task/epic/bug status transitions also produce task
events. The index mirror and event drafts commit together. The outbox retries
the same IDs after restart and clears only after the bus confirms append.

The index reads `.bboxy/**/*.md`, rejects symlink traversal and caps documents
at 2 MiB. Broken frontmatter remains visible with a diagnostic. It handles
`[[target]]`, `[[target|display]]`, `[[target]]{rel:weight}`, `links:` records and
frontmatter relationship fields. Ambiguous aliases are not resolved by guess.
Markdown, task, execution, artifact and provenance graphs rebuild from SQLite.
A watcher coalesces changes into a full reconciliation; a watcher failure is
reported rather than silently serving a stale index.

Mux-authored document writes stamp `created_by`/`updated_by`, sign frontmatter
and body digests with DSSE PAE and append signed preparation/completion records
before/after the file effect. `.bboxy/log/<date>.jsonl` is a signed-record
projection. Anonymous external changes are observed without invented
attribution. Primary-signed daily checkpoints, export and repair of a projection
interrupted between SQLite commit and JSONL append are not implemented here.

## Seat observation and viewport streams

Each attached seat gets a private `/tmp/bbt-<uid>-<ULID>/<seat>.sock` namespace
(short enough for macOS sockaddr_un) with 0700 directory/0600 socket. Discovery
through the owner client yields its path, process generation and capability;
never log the capability. Restart creates a new namespace and capability.
After a hard kill the abandoned empty namespace/socket requires later janitor
policy; a new instance never unlinks or takes over it.

Every version-1 NDJSON request binds capability, seat and process generation.
`Attach`, `Snapshot`, `Subscribe`, `Applied`, `GetHistory`, `BusSubscribe`,
`PollEvents` and `Ack` share that boundary. Socket consumers are restricted to
their own ephemeral `seat/<id>/...` and `inbox/<id>/...` subjects. No account or
other-seat authorization is implied. At most 32 clients per endpoint, eight
consumers per session, 1 MiB per line, a two-second write deadline and bounded
reader channels limit slow/malformed clients. Oversized snapshots fail the
frame bound; chunking/negotiation is not implemented.

A viewer retains one acknowledged baseline, one sent snapshot and one latest
pending frame. Deltas carry changed rows and cursor/modes/palette/style/title/
cwd metadata. `Applied` must match the sent view generation and sequence;
wrong/stale baselines are rejected atomically. Resize changes view generation
and requires a snapshot. `GetHistory{from,to}` uses inclusive generation-bound
capture sequences and returns a bounded `History` page with records, optional
logical-row metadata, a next cursor and an explicit gap plus the supplied hot
retained range when requested history is unavailable. Sealed cold segments are
hydrated here; rendering arbitrary historical rows remains a later integration.
Native OSC cwd/status observation, hooks,
terminal query replies and renderer/GUI wiring remain integration work.

## Verification and measurements

### Harness adapters (W2.3)

`adapters::HarnessRecord` is an open-string record for command, tile, icon,
default role, wake method, state source and hooks. `Session` instantiates the
`Adapter` trait; built-in Claude, Codex and zsh records select native behavior.
Custom records retain inferred cadence state and operator handoff for submission.
`LogicClient::spawn_harness` creates private launch overlays, persists the native
session mapping and retains overlay files through close grace. Actual exit
retires them; a logic restart preserves the live child's files. Ambiguous spawn
does not delete an overlay that a possibly running process may still need.

Claude gets an additive role prompt and an absolute `bboxy450-term hook` command
in a 0600 settings overlay. Codex gets `developer_instructions` and inline hooks,
with current CamelCase event names (the decoder also accepts the earlier snake
spellings). No user files, trust hashes or bypass flags change. Hook approval is
the operator's native Codex review. Zsh gets a private `ZDOTDIR` which sources the
user's files and adds OSC 133 A/B/C/D and OSC 7. B is emitted at the actual prompt
end, C before execution. Overlay cleanup waits for process exit, not pane close.

Each live term has an unknown-OSC callback. OSC 7501 probes receive the same
payload with BEL on the same supervisor writer lease; replies queue behind an
existing write and keep their stable input IDs. A new launch observes its first
captured bytes once, including a probe arriving before view attachment. Restart
replay never emits probe replies or rebuilds fresh readiness from old output.
7501 is reported grade; 7502 is shim grade; cadence/21337 are inferred. Running
subagent rows keep start order and leave on clear/idle; Codex gets child rows
only from hooks. The PTY of arrival owns a hook regardless of its `seat=` label.
Live adapter changes publish `seat/<id>/state` on the existing R1-backed bus.

Claude's owned sessions file must match PID, process start and native session.
Logic polls that record off the UI thread. Known Codex thread IDs get read-only
status snapshots on the supplied R1 handle, and `native_event` accepts bound
app-server notifications; generated snapshots are identified as such internally.
Neither channel reads transcripts. Display idle/done is never a ready token:
readiness needs the native file/API or our zsh prompt marker, plus the input
arbiter's foreground/draft/termios/render checks. Tokens expire, bind all process
and attachment fields, and are consumed before a paste. The PTY helper journals
a stable ID before calling the sole writer; partial/unknown results stay unknown
and are not pasted again. Multiline/tab text without bracketed paste and terminal
control bytes require handoff rather than accidental command submission.

Native attach checks the process identity and endpoint. Claude writes a bounded
JSON line with next/later/now; Codex initializes the Unix WebSocket, verifies the
exact loaded thread and uses queue/add or a turn-ID-fenced steer. Now requires
the acting caller's interrupt authority; the target's role grants none. Written
and queued are evidence levels, never model consumption. The message layer must
persist native outcomes before retry/restart; the adapter does not automatically
fall back after an ambiguous write. MCP/GUI input arbitration is a later client
of these APIs, not an additional writer in this brief.

The private hook entry runs before window/runtime/GPU setup. It accepts bounded
stdin (or Codex notify JSON argv), emits raw selected fields under `raw`, caps
background tasks at 32 plus total, writes at most 4 KiB to `/dev/tty`, and sends
larger/unavailable-tty payloads through the generation/capability-bound seat
socket up to 64 KiB. It exits quietly, has a 50 ms IO deadline, and never spools.

Codex protocol checks use the installed generated schema and the official
[app-server documentation](https://learn.chatgpt.com/docs/app-server) for
initialization, thread status and expected-turn steering. Hook spelling and
parent `session_id` versus child `agent_id` follow the official
[hook documentation](https://learn.chatgpt.com/docs/hooks); a parent session is
never guessed to be a missing subagent ID. `w2-3-codex-schema.log` records the
installed schema fields without runtime session data.

Exercised: the retained Claude 7501 capture through the real term callback;
matching/mismatching sessions-file fixtures and pending-dialog rejection with
zero writes; a real zsh prompt and preexec; private launch/cleanup under logic;
same-PTY probe response before two seconds and suppression after restart;
capability admission; local native fixture sockets for Claude priorities, Codex
queue/steer/unknown replies; input-token fencing/journaling; and the actual hook
executable's silence, latency and large-payload fallback. No live Claude/Codex
model session was created for this task.

Not exercised live: role-prompt/snapshot behavior on Claude resume, settings
merge/policy, hook trust/inline configuration on installed Codex, actual Codex
subagent events, native socket auth/acceptance, queue behavior while a model is
active, native idle subscriptions or model-consumption correlation, harness
resume/enrollment, operator-draft/foreground synchronization or a consumable
native ready handshake. Arbiter negative facts must come from the input broker;
the fixture values do not certify a real TUI's readiness. Off-PTY hook fallback
path/capability changes on logic restart need refresh; old inherited environment
cannot discover the replacement socket. Shell prompt themes that replace
precmd/preexec hooks or `ZDOTDIR`, C1 OSC support in upstream term-core,
Linux/macOS14, and 1000-seat adapter load remain unvalidated. None of these gaps
authorizes a trust bypass or submission after an unknown outcome.

`cargo test -p bbt-mux --locked` covers storage/signatures/corruption, consumer
replay/redelivery/gaps, registry undo, graph recovery, watcher-to-bus task events,
provenance signatures, real seat sockets and W2.1 PTY/lease/backpressure tests.
The subprocess fixture hard-kills logic, reattaches through the supervisor
socket and verifies the same live zsh PID and exact screen. W1.2 adds red-SGR
restore, incomplete parser checkpoints, torn suffix/CRC rejection, hot rollover
with cold history, 10k-line history roundtrip and cell-for-cell replay checks.
The W1.2b live test completes a 20 MiB writer while logic is stopped and a
notification queue is unread, logs total/max write latency before restart, then
checks the complete output digest and red tail after cold replay without a gap.
Another test stalls a live logic receiver and verifies sequence-jump catch-up.
Group-commit recovery and rejection of corrupt sealed payloads are covered too.
Idle checkpoint/index publication is also tested. The viewport test
also checks a metadata-only palette/cursor delta and flattened wire format.

Run the two finite measurements explicitly:

```sh
cargo test -p bbt-mux --locked --test measurements -- --ignored --nocapture --test-threads=1
```

Measured locally on 2026-10-22 (no OS limit changes):

- 200 live PTYs plus 200 seat listeners and per-object SQLite: 8 baseline fds,
  408 observed (400 added); soft maxfiles 4096, hard 9223372036854775807.
- 10k synthetic document rows/100k directed links: recursive CTE 42.69–43.24 ms,
  petgraph DFS 3.37–3.61 ms; both reach 10k nodes in three trials.
- One durable doc edit to 1000 live in-process consumers: 2.78 ms. This is not a
  harness wake, network or GUI load measurement.

Actual command output, failures/corrections and acceptance pointers live in
`.bboxy/runs/w2-2-*.log`; W1.2 output is in `.bboxy/runs/w1-2-*.log`; W2.1's earlier evidence is in `.bboxy/runs/w2-1-*.log`.
Not validated: real power loss, supervisor restart, native harness input or
state integrations, cloud/SSH, account grants, production key custody, 1000
running seats, macOS 14/Linux/mobile, hosted CI or renderer parity. No user
keys, credentials, harness trust settings or model transcripts were accessed.
