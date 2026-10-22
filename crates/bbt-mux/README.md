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
If there is no safe checkpoint boundary, PTY reads pause; a reserved checkpoint
budget lets catching-up logic publish replacements and resume capture.

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

Append order is complete record → `sync_all` → CRC-protected committed marker
→ marker/directory sync. The independently valid previous marker remains.
Compaction writes a new file first and publishes its marker before retiring
anything. Recovery rejects uncommitted or torn suffixes, falls back to the
previous valid boundary and reports the recovered cursor; suffix bytes go to
cold storage before truncation. Invalid committed data without a valid previous
boundary is an error, never a blank recovered screen. Strict media/power-loss
ordering (`F_FULLFSYNC`) and real power-cut testing are not claimed.

Eviction starts with a complete checkpoint and retains at least the newest two
checkpoints plus every later capture record; older checkpoints remain while
budget permits. `capacity_bytes` bounds the current hot file including encoded
checkpoint/index records. The previous boundary may pin one additional file of
at most that budget (plus one temporary file during compaction). Retired and
orphaned owned log files are compressed into content-addressed cold blobs with
a `cold.jsonl` digest reference before removal from the hot namespace. Cold
retention follows never-delete and is outside the hot budget. Record metadata
and the logical index are bounded by the hot record count, not a 1000-seat memory
or throughput promise. Ring files stay under private runtime state, never Git.

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
retained range when requested history was evicted. Cold history retrieval and
rendering arbitrary historical rows are later integrations. Native OSC cwd/status observation, hooks,
terminal query replies and renderer/GUI wiring remain integration work.

## Verification and measurements

`cargo test -p bbt-mux --locked` covers storage/signatures/corruption, consumer
replay/redelivery/gaps, registry undo, graph recovery, watcher-to-bus task events,
provenance signatures, real seat sockets and W2.1 PTY/lease/backpressure tests.
The subprocess fixture hard-kills logic, reattaches through the supervisor
socket and verifies the same live zsh PID and exact screen. W1.2 adds red-SGR
restore, incomplete parser checkpoints, torn suffix/CRC rejection, checkpoint
boundary eviction, 10k-line history roundtrip and cell-for-cell replay checks.
The live ring test fills a bounded ring while logic is absent, then proves
checkpoint publication unblocks capture and restores the final red tail without
a capture gap. A separate test verifies idle checkpoint/index publication. The viewport test
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
