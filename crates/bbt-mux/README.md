# bboxy450-term-mux — restartable logic

S1 moves the existing supervisor, its protocol and clients, ring/writer and
shared blob primitives into the lower `bbt-supervisor` crate. This crate keeps
logic, SQLite objects, the bus, index, adapters and messaging. Supervisor paths
are re-exported for existing callers; no PTY, capture or dispatch behavior changes.

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
| `protocol`, `supervisor`, `transport` | Compatibility re-exports from `bbt-supervisor`: one mio actor owns PTY masters, children, resize, writer leases and close deadlines; bounded u32-BE length/versioned JSON is the supervisor IPC. |
| `logic` | One logic IO worker attaches owned children, restores terminal checkpoints, captures ordered output and hosts seat endpoints. A supplied runtime handle runs the bus and asynchronous sockets. |
| `store/sqlite` | Per-object SQLite WAL/FULL databases, version 1, key/value, SQL and alarms. Connections are operation-scoped rather than held open for every object. |
| `store/blobs` | Re-export of `bbt-supervisor`'s shared SHA-256/zstd local blobs, fsynced temporary file and rename, digest verification and a 64 MiB decompression bound. |
| `store/events` | Immutable, Ed25519-signed, hash-chained event rows; permanent event-ID deduplication and conflict rejection; chain verification on reopen. |
| `ring` | Re-export of `bbt-supervisor`'s CRC32C capture, committed boundaries, checkpoint-aware hot retention, cold blobs, isolated replay and logical-row indexes. |
| `registry` | Account-owned ULID seat records, signed-role shape, location, mute, harness session and task references, close revisions and exact restore positions. |
| `index` | FSEvents/inotify invalidation, Markdown/YAML parsing, weighted wikilinks, SQLite mirror and durable event outbox, five petgraph projections. |
| `bus` | One bounded router over per-object event streams, consumer policies, replay, coalescing, explicit acknowledgments and request/reply. |
| `messaging` | Signed local envelopes, SQLite channel tree and inbox/outbox, current-grant release, receiver scheduling, evidence receipts and adapter delivery. |
| `mcp` | Stdio bridge to the same seat socket; logic owns read-tool schemas, current authority, durable operation results and signed control audit. |
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
composer readiness nor application acknowledgment. Seat sockets expose semantic
messaging requests through the common broker, without a raw-input operation.

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
W1.2b decouples capture from checkpoint progress. At the hot cap the capture writer
seals the complete CRC-framed file into a content-addressed cold blob and starts
a new bounded hot file. It never waits for logic or a new checkpoint to keep
reading. A full notification queue retains only the latest event; logic hydrates
the skipped capture sequences on resume. The ring-less W2.1 primitive still has
its original bounded queue behavior; production logic configures the ring.

W1.2c moves record encoding, hot-file IO, group commits, cold hashing/compression
and sealing onto **one disk writer per supervisor**, shared by all its seats.
The PTY actor only copies output into a bounded queue. It coalesces short reads
into at most 16 KiB records, flushing a quiet partial record after 2 ms, before
resize, or at EOF/exit. A poll deadline drives that flush without sleeping on
the actor. The queue admits at most 64 MiB per seat, 128 MiB across the
supervisor and 8192 jobs (including the job currently doing IO); byte charges
include record headers and checkpoint requests. A full queue is the only
capture-side backpressure. It wakes the actor when space returns. Notification
queues still coalesce independently and cannot stop capture. Recovery, history
and checkpoint requests are also deferred: both local and socket callers wait
for their reply without stopping PTY reads on the actor. Initial ring admission
still opens/validates its private directory before installing the writer handle.

`Status.capture` reports cumulative encoding/write/sync/marker/cold durations,
maximum seal time, queued bytes, peak queued bytes and full-queue episodes.
Worker failures return an explicit IO error rather than a complete history
claim; media-failure recovery is outside this pass. The writer is a standard IO
thread, not another Tokio runtime. Supervisor shutdown drains its queued work.

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
the durable `committed_sequence`. The supervisor writer groups output commits
at 1 MiB or its next 20 ms flush deadline; direct synchronous `Ring` users retain
the 64 KiB threshold. Resize, exit, checkpoint, sealing and
supervisor shutdown flush immediately. A flush orders data `sync_all` before the
CRC-protected marker and marker/directory sync. Capture-owner death may discard
the uncommitted group and queued memory; logic restart while the supervisor lives
does not. A request snapshots the captured cursor after preceding queued work,
and only a synced marker advances the durable cursor.
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
(up to the configured hot-file size), separately from the bounded capture queue.
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

`cargo test -p bboxy450-term-mux --locked` covers storage/signatures/corruption, consumer
replay/redelivery/gaps, registry undo, graph recovery, watcher-to-bus task events,
provenance signatures, real seat sockets and W2.1 PTY/lease/backpressure tests.
The subprocess fixture hard-kills logic, reattaches through the supervisor
socket and verifies the same live zsh PID and exact screen. W1.2 adds red-SGR
restore, incomplete parser checkpoints, torn suffix/CRC rejection, hot rollover
with cold history, 10k-line history roundtrip and cell-for-cell replay checks.
The W1.2b live test completes a 20 MiB writer while logic is stopped and a
notification queue is unread, logs total/max/p99 write latency before restart, then
checks the complete output digest and red tail after cold replay without a gap.
Another test stalls a live logic receiver and verifies sequence-jump catch-up.
Group-commit recovery and rejection of corrupt sealed payloads are covered too.
Idle checkpoint/index publication is also tested. The viewport test
also checks a metadata-only palette/cursor delta and flattened wire format.
A gated-writer fault fixture verifies nonblocking admission, an exact byte cap,
one full-queue episode and complete durable replay after the writer resumes.
The explicit optimized performance gate writes 128 MiB (twice the per-seat
queue budget) with logic stopped and running, asserts max write ≤50 ms and
both writer/durable throughput ≥30 MiB/s, and verifies tail recovery without
a gap and the same child PID:

```sh
cargo test -p bboxy450-term-mux --release --locked --test logic ring_capture_throughput -- --ignored --nocapture --test-threads=1
```

Run the two finite measurements explicitly:

```sh
cargo test -p bboxy450-term-mux --locked --test measurements -- --ignored --nocapture --test-threads=1
```

Measured locally on 2026-10-22 (no OS limit changes):

- 200 live PTYs plus 200 seat listeners and per-object SQLite: 8 baseline fds,
  408 observed (400 added); soft maxfiles 4096, hard 9223372036854775807.
- 10k synthetic document rows/100k directed links: recursive CTE 42.69–43.24 ms,
  petgraph DFS 3.37–3.61 ms; both reach 10k nodes in three trials.
- One durable doc edit to 1000 live in-process consumers: 2.78 ms. This is not a
  harness wake, network or GUI load measurement.

Actual command output, failures/corrections and acceptance pointers live in
`.bboxy/runs/w2-2-*.log`; W1.2 output is in `.bboxy/runs/w1-2-*.log`, capture
profiles/performance in `.bboxy/runs/w1-2c-*.log`; W2.1's earlier evidence is in `.bboxy/runs/w2-1-*.log`.
Not validated: real power loss, supervisor restart, native harness input or
state integrations, cloud/SSH, account grants, production key custody, 1000
running seats, macOS 14/Linux/mobile, hosted CI or renderer parity. No user
keys, credentials, harness trust settings or model transcripts were accessed.

## Local messaging (W2.4)

`LogicClient::messaging(seat, generation, request)` and seat-socket
`Action::Messaging` share `messaging::api::{Request, Response}`: send, paginated
inbox/outbox/thread/channel reads, receipts, explicit application acknowledgments,
tree listing, subscription/link edits, grants and mute. The socket capability
and current generation select the caller; neither role nor signing key is a
request field. Workers can edit only their own subscription links. Controllers
are limited to their swarm. The GUI's in-process `messaging_operator` method is
privileged and is deliberately absent from the seat socket. Native endpoint
binding is another trusted host/discovery method, never a peer operation.

**Custody seam:** `SeatKeys` supplies issuer, generation-bound leaf and signature
verification. `LocalSeatKeys` uses the already-supplied broker signing key as a
**local development issuer**, not an OpenPGP primary. HKDF-SHA256 derives a seed
with no salt and info `bboxy450/seat-leaf/v1 || ULID.to_bytes() || generation_be64`.
No new secret is persisted. Only public issued-grant records and the pinned issuer
are stored in the account object. Envelopes explicitly say `issuer_kind=local-dev`;
`gpg-root` is reserved and rejected by this provider. Full GPG certificates,
revocation/expiry and Keychain custody remain the identity implementation's job.
A different issuer cannot silently reopen the old local account. This seam is
not a claim that local development grants implement the complete GPG chain.

Wire v1 signs every field except the signature as two fixed-order JSON tuples
(the order is in `wire::Envelope::signing_bytes`), prefixed by
`bboxy450/message/v1\0` and the tuple's u64 big-endian byte length. ULIDs emit
canonical strings; counters are canonical decimal strings, timestamp seconds
are bounded to exact JSON integers, signature is unpadded base64url. Names,
text, wikilink attachments and the complete envelope are bounded. Signed class
must match the issued role; a subsequent demotion never retains Oracle priority.
Delivery text labels peer content as data granting no local execution permission.

Each swarm stores `(path, kind, target, filter, rate, paused, grants)` rows and an
in-memory routing projection. A terminal `*` visits direct sinks; `**` descends
folders and folder bridges. Active-path cycle detection terminates link loops;
multiple arrival paths are retained but each `(message, seat, agent)` has only
one delivery. The closest explicit seat ACL replaces inherited rights (`[]`
revokes); every denial names the folder and missing right. Controller edits and
receipt transitions enter a durable bus outbox in the same transaction as the
state change. Stable draft IDs make publish/restart replay idempotent. Removed
subscriptions are not recreated by bootstrap. Resolution is capped at 16,384
nodes, 65,536 visits, 64 link depth/arrival paths and 4096 sinks per message.

Signed envelopes, inbox records and sender/recipient receipts commit before
scheduling or any native write. Receiver-local FIFO queues prioritize the
verified Oracle class, with a 10/s, burst-30 token bucket; excess joins worker
FIFO, and a waiting worker gets a dispatch after four Oracle messages. These
are current local defaults. Rate-controlled links wait for their window and
batch at most 16 messages / 40 KiB of content. There is at most one in-flight
attempt per receiving seat and 32 asynchronous native tasks on the supplied
runtime. An attempt is durably journaled before effects. Restart turns an
unfinished attempt into `native-unknown` and never automatically resubmits it.
Already queued/written/unknown work is not replayed. Explicit recipient ack is
the only producer of `application-acknowledged`; a late native completion cannot
regress it. A known rejection before mutation stays `recipient-stored` with a
retry delay; no ambiguous native attempt falls back to a paste.

Authenticated GUI direct control can still reach a muted seat one to one; its
terminal-issued grant digest binds that origin. Peer requests have no such flag
and cannot claim the exception.

Mute commits the record, policy epoch, initiating controller and stable signed
notice intent atomically in the account object. The swarm outbox recovers that
intent after a crash. Notices are metadata and do not wake a harness. Inbound
and outbound held work keeps original IDs/signatures across restart; release
revalidates the sender, current role, same-swarm target, path ACL, filters and
pause flags. Revoked or removed routes remain held with a reason. Unmute does
not promise held count becomes zero, and cannot recall an in-flight native write.

PTY fallback obtains supervisor-owned foreground-group/termios/input-epoch,
draft and output-cursor facts. It additionally requires the adapter's fresh
positive readiness after the last input, a caught-up screen, no pending dialog,
tool, synchronized output or draft, and the W2.3 single-use token. The actor
rechecks the full context immediately before admission, and again before a
blocked first write. Raw input changes the epoch; draft editing is tracked
conservatively (backspace does not guess that a draft became empty). A prefix or
uncertain write remains unknown. Custom hookless harnesses remain pending for
operator handoff. Known active subagent targets can be explicitly relayed via
main; an ended/unreported target is held, never redirected. Native subagent
addressing and model consumption are not claimed.

Pages are capped at 128 items / 512 KiB; tree pages use path cursors, message
pages local commit sequence, receipt pages receiver-local row cursors. Channel
reads remain restricted to messages in the caller's inbox/outbox (or an Oracle's
swarm view with channel read permission); joining later does not reveal an old
private inbox. This pass provides the local client/broker path, not the composer
UI, messaging MCP tools, networking, external identity enrollment or multi-account routing.

## Read-only MCP control (W2.5a)

`bboxy450-term mcp` returns before window, GPU or runtime setup. It reads
`BBOXY_SOCK`, `BBOXY_CAPABILITY` (64 hex characters), `BBOXY_SEAT_ID` and
`BBOXY_GENERATION` from its launch environment. These identify one admitted seat
generation, not a caller-supplied role. No capability is put in the catalogue,
tool results, audit or diagnostics. The private CLI uses blocking IO; it is not
called on the UI thread.

The bridge implements four newline-delimited JSON-RPC methods: `initialize`,
`ping`, `tools/list`, `tools/call`. Initialized notifications have no response.
Frames are bounded at 1 MiB, arguments at 64 KiB and tool results at 512 KiB.
The bridge forwards `Catalogue` / `Call {op_id, tool, args}` to the existing
capability-bound seat socket; logic supplies both catalogue and results.
No resources, prompts, sampling, screens or mutations are exposed in this pass.

The five tools are `orient`, `self_status`, `seat_list`, `seat_get`, `graph_read`.
Workers read seat metadata in their own swarm and their own task/markdown graph
items. Oracle-class seats can read account seat metadata and execution/task/
markdown graphs. The communication graph awaits the messaging tool pass.
`orient` includes bounded task briefs/verify commands, own execution context and
tree, and the current role/class. `unread_count` is explicitly unavailable
(`null` plus reason); `pending_application_ack` is separate delivery evidence.
No read count is inferred from a delivery receipt.

For each call, logic rechecks the supervisor's current generation and the local
issuer's current grant epoch/class before reading or returning a retained result.
Results at `control/op/<ULID>` and the control-audit outbox commit in one seat
SQLite transaction, including concurrent duplicate calls. Reuse under changed
arguments or authority is rejected. Audit publication is restartable, signed and
deduplicated by event ID at `seat/<id>/control`; it records tool, actor, outcome
and argument digest, never argument payloads. Outbox reads preserve call order.
These are read-only dispatch closures; effectful tools must journal admission
before effects when they are added.

A client can supply `params._meta["bboxy450/op_id"]` to retry an exact operation;
otherwise the bridge mints a ULID per call. It never retries automatically. A
transport error preserves that ID in `error.data` with `status: unknown`; it does
not assert the call failed before commit. Successful/error tool results include
the operation ID in `structuredContent` and the equivalent text content.
Authority still uses W2.4's explicit `local-dev` issuer behind `SeatKeys`; this
is not the deferred GPG certificate chain. No identity implementation is changed.

The integration tests exercise framing, schemas, current authority, scoped graph
reads, restart/replay, signed audit and application-ack counts. The explicit live
test launches installed Claude and Codex with per-launch MCP configuration and
synthetic tasks, discards raw client output, checks the returned role and signed
audit seat/generation, and compares the named user config files before/after:

```sh
cargo build -p bboxy450-term --locked
BBOXY_TEST_EXE="$CARGO_TARGET_DIR/debug/bboxy450-term" cargo test -p bboxy450-term-mux --locked --test mcp mcp_installed_claude_and_codex_call_orient -- --ignored --exact --nocapture
```

Installed Codex forwards the four `BBOXY_*` names through its per-launch
`mcp_servers.bboxy.env_vars` setting. The overlay/skill wiring and the full
no-user-config comparison belong to W2.5b; messaging, task/artifact and input/
spawn tools remain later passes. Protocol references: [MCP lifecycle](https://modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle),
[stdio transport](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports),
[tool results](https://modelcontextprotocol.io/specification/2025-11-25/server/tools),
and [Codex MCP configuration](https://learn.chatgpt.com/docs/extend/mcp?surface=cli).

## Dependency name

The Rust library name remains `bbt_mux`. Use the existing import name with a versioned package alias:

```toml
bbt-mux = { package = "bboxy450-term-mux", version = "0.1.0" }
```

## Respectful slopping

Project-owned contributions use [Respectful Slop License 1.0-Draft](LICENSE).
Retained upstream material keeps its original terms; earlier grants are not
withdrawn. [ATTRIBUTION.md](ATTRIBUTION.md) identifies the scope, source and reuse
examples; [NOTICE](NOTICE) preserves factual upstream notices.
Slop respectfully. Keep the credit attached.
