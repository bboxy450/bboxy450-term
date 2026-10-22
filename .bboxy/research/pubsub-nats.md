# Pubsub — NATS and the state of the art, and the minimal bus under it

2026-10-21 (`date` → Wed Oct 21 16:51 PDT 2026) · research subagent for Vesper Greywind <ORACLE> · for Oracle review.

[[events]] · [[messaging]] · [[networking]] · [[storage]] · [[vision]] · [[kittens-events]]

Operator: "for the pubsub system research state of the art, NATS and systems like that; LESS IS MORE and KISS; we should design our events system so messages are just an abstraction on top of the event bus."

Read before writing: [[events]] (one envelope, topics, per-topic seq, subscriptions with since/gap, ext in, webhook out), [[messaging]] (signed envelope, six evidence states, Oracle > worker with token bucket, mute, task bodies), [[networking]] §4 (own gossip over quinn, phase 2/3), [[storage]] (`events.sqlite` per object, Ed25519-signed hash-chained rows, never delete, `Store.alarm`), [[kittens-events]] (the operator's bus: exact routes, ack-before-first-event, bounded queues with `gap`, seat socket §4), [[vision]] «Working rules» and «The composability test».

Every fact below carries `source:` and a version or date. Lines marked **measured** were produced on this machine by the command shown. Lines marked INFERRED are mine.

## 1. NATS — core and JetStream

`nats-server` v2.15.1, released 2026-10-09, Go, Apache-2.0, 20.9k stars (`source: https://api.github.com/repos/nats-io/nats-server/releases/latest`, `.../repos/nats-io/nats-server`, fetched today).

**Measured** (`curl -sL …/v2.15.1/nats-server-v2.15.1-darwin-arm64.tar.gz`, `shasum -a 256` = `ed0fe392…` matching the release `SHA256SUMS`; `./nats-server -a 127.0.0.1 -p P -m M`, then `GET /varz`; `-js -sd store` for the second row):

| Mode | Binary | RSS idle (`/varz.mem`) | `ps -o rss` | Defaults from the live server |
|---|---|---|---|---|
| core | 17,658,562 B Mach-O arm64 (6.8 MB tar.gz) | 18,333,696 B = 17.5 MiB | 18,032 KB | `max_payload` 1,048,576 · `max_control_line` 4096 · `max_pending` 67,108,864 · go1.27.2 |
| `-js` | same | 18,546,688 B = 17.7 MiB | 18,304 KB | `jetstream.max_memory` 18 GiB, `max_storage` 10.9 GiB (derived from the machine), 0 streams |

The protocol page says `max_control_line` default 1024; the live server says 4096. Real beats paraphrase.

### 1.1 Facts

| Area | Fact | source |
|---|---|---|
| Subjects | `.` separates tokens, "cannot be part of a token". `*` "matches exactly one token". `>` "matches one or more tokens" and "can only appear at the end of a subject". Publishers always name a full subject; wildcards are for subscriptions. Any UTF-8 except whitespace; `$…` and `_INBOX` reserved; guidance "~16 tokens and under 256 characters", "start general, get specific", "plan for wildcards". | https://docs.nats.io/nats-concepts/subjects |
| Queue groups | Subscribers sharing a queue name: each message to "one randomly chosen member"; not load-aware; members join/leave with no config; "messages are not persisted". JetStream push consumers reuse core queue groups. Services API default group `q`. | https://docs.nats.io/concepts/queue-groups, https://docs.nats.io/reference/faq |
| Request-reply | Pure pubsub: requester subscribes to a unique inbox subject, publishes with reply-to, responder publishes to it. No-responders: "the server reports the missing service right away instead of waiting for the timeout". Scatter-gather = several responders without a queue group; `request()` returns the first reply. | https://docs.nats.io/concepts/request-reply |
| Client protocol | Text, CRLF. `PUB <subject> [reply-to] <#bytes>\r\n[payload]\r\n`; `SUB <subject> [queue group] <sid>`; `UNSUB <sid> [max_msgs]`; `MSG <subject> <sid> [reply-to] <#bytes>\r\n[payload]`; `HPUB`/`HMSG` add a header block; `INFO`/`CONNECT` JSON; `PING`/`PONG`; `+OK`/`-ERR`. TCP, WebSocket and (the page says) UNIX domain socket. | https://docs.nats.io/reference/reference-protocols/nats-protocol (v2.15) |
| JetStream stream | A stream captures `subjects` (wildcards allowed, `orders.>`). `retention`: `Limits` (default), `Interest`, `WorkQueue`. Limits: `max_messages`, `max_bytes`, `max_age`, `max_message_size`, `max_messages_per_subject`, `max_consumers` (all unlimited by default). `discard`: `Old` (default) / `New`. `storage` File/Memory. `num_replicas` 1 default, max 5. `duplicate_window` 2m default: "the server rejects another message with the same `Nats-Msg-Id` header". Also `sealed`, `deny_delete`, `deny_purge`, `allow_rollup`, `allow_direct`, `mirror`, `sources`, `republish`, `subject_transform` (2.10), `compression` (2.10), `first_sequence` (2.10), `allow_message_ttl` (2.11), `subject_delete_marker_ttl` (2.11). Stream sequence is per stream; "replaying a stream is a read", nothing removed. | https://docs.nats.io/nats-concepts/jetstream/streams; field list https://docs.rs/async-nats/0.50.0/async_nats/jetstream/stream/struct.Config.html; https://docs.nats.io/nats-concepts/jetstream/js_walkthrough |
| JetStream consumer | `durable_name` set → server remembers acked progress; else ephemeral, reaped after `inactive_threshold`. `deliver_policy`: All, Last, New, ByStartSequence, ByStartTime, LastPerSubject. `ack_policy`: Explicit, None, All. `ack_wait` → redelivery; `max_deliver` caps retries; `backoff` custom delays. `filter_subject` / `filter_subjects` (2.10) with wildcards. `replay_policy` Instant/Original. `max_ack_pending` (default 1000) pauses delivery. Pull: `batch`, `expires` (~30 s). `priority_policy`/`priority_groups` (2.11). | https://docs.rs/async-nats/0.50.0/async_nats/jetstream/consumer/struct.Config.html; https://docs.nats.io/nats-concepts/jetstream/consumers |
| KV | "a bucket is a JetStream stream named `KV_<bucket>` whose subjects are `$KV.<bucket>.>`"; key = last token, value = latest message on that subject; `put` is a publish, `get` reads the last message; watch = a consumer (snapshot then live); revisions = stream sequence, compare-and-swap, per-key TTL, history depth = `max_messages_per_subject`. | https://docs.nats.io/nats-concepts/jetstream/key-value-store |
| Object store | "a single JetStream stream named `OBJ_<bucket>`"; `put` = chunk messages then one metadata message; `get` replays chunks and verifies a SHA-256 digest; links; watch. | https://docs.nats.io/learn/object-store |
| Leaf nodes | A server that dials out to a hub and bridges interest across one hop: "the leaf carries only subjects that have interest on the other side"; the hub sees one connection, not the hundred clients behind it; trees of leaves; account-scoped bridging; a leaf may run its own JetStream under a distinct domain. | https://docs.nats.io/running-a-nats-service/configuration/leafnodes |
| Supercluster | Gateways join "two independent clusters into one logical system without combining them into a single mesh"; one gateway connection per remote cluster per server; interest-only forwarding; queue geo-affinity; gateways carry "interest across regions, not stream copies". | https://docs.nats.io/running-a-nats-service/configuration/gateways |
| JetStream clustering | Raft. Meta group elects a meta leader that places streams and consumers; "each stream gets its own Raft group"; each consumer too; write acked on majority; R1 default, R3 "production floor", at most five copies; odd counts. | https://docs.nats.io/running-a-nats-service/configuration/clustering/jetstream_clustering |
| Embedding | Go only: "if your application is in Go … you can even embed a NATS server inside your application"; `server.NewServer(opts)`, `Start()`, `ReadyForConnections`, client via `nats.InProcessServer(ns)`. No Rust server exists: GitHub search `nats server language:rust` returns clients and unrelated repos (`source: https://api.github.com/search/repositories?q=nats+server+language:rust`, today); crates.io has no `nats-server` crate. | https://docs.nats.io/running-a-nats-service/clients; https://gist.github.com/kaustavdm/73560c70744cc5e08f3d08cc43a8f6af |

### 1.2 What NATS gets right that a tiny embedded bus copies

1. **One grammar for everything.** Hierarchical subject, `*` one token, `>` tail. Subscriptions, stream capture, consumer filters, KV keys and object chunks are all the same match. We have exactly this need: `seat/<id>/*`, `doc/path/.bboxy/specs/>`, `task/*/report`.
2. **The stream is the only persistence primitive.** KV and object store are streams with a naming convention and `last_per_subject`. Our `events.sqlite` per object ([[storage]]) is that stream; the doc frontmatter "current value" and the GUI's "current state of every seat" are `last_per_subject` reads, not separate APIs.
3. **Consumer = named cursor + filter + ack policy.** Durable iff named; ephemeral dies with the connection; start at seq, time, new, all or last-per-subject. This is [[events]] `since` plus [[kittens-events]] §4.6 replay, formalised.
4. **Dedupe by message id at the stream**, idempotent re-publish. [[messaging]] already requires "duplicate retries yield one inbox record".
5. **Request-reply is a reply subject**, nothing else; no-responders is a server-side negative reply. Our `ack` frames and the control plane's `req` → `ack` are this.
6. **A wire protocol you can speak with `nc`.** `PUB subject n\r\n…` is the same spirit as kittens' bare text line; our NDJSON seat socket ([[kittens-events]] §4.2) already is the composability-test transport. Keep `max_payload` 1 MiB: it equals kittens' line cap.
7. **Interest-based forwarding** (leaf, gateway): carry across a link only subjects somebody on the other side subscribed to. That is the phase-2 cloud relay rule for [[events]] «Cloud» in one sentence.
8. **Core vs JetStream in one grammar.** Some subjects are fan-out only, some are persisted. We need exactly two tiers: `inbox/>` and `seat/*/output` are fan-out (the byte ring already stores output), everything else is appended.

### 1.3 What NATS has that we must not build

Clustering and Raft (meta, per-stream and per-consumer groups), replicas, supercluster gateways, leaf-node protocol, accounts/NKeys/JWT (our grants chain from the account root key, [[identity]]; NATS enforces nothing we need), mirrors and sources (one `source` may return as the cloud relay in phase 2, as a consumer with `deliver: transport`), subject transforms, republish, headers (envelope fields), push consumers with flow control, `Interest`/`WorkQueue` retention (never-delete makes them moot; `Limits` → cold blobs is the only policy), rate limiting, sampling, KV and object store as APIs (we have docs and `Blobs`), WebSocket and MQTT listeners, the text protocol itself (NDJSON on the seat socket is the equivalent).

## 2. Comparable designs — one row each, the one idea worth stealing

| System | Version / date | The one idea | source |
|---|---|---|---|
| Kafka | 4.0 docs | A partition is an append-only log; **the consumer owns its position**: "the position of the consumer gives the offset of the next record", "Kafka allows the consumer to manually control its position, moving forward or backwards"; retention is per topic and "not deleted after consumption". → our consumer cursor is one integer, rewindable, independent of retention. | https://kafka.apache.org/intro; https://kafka.apache.org/40/javadoc/org/apache/kafka/clients/consumer/KafkaConsumer.html |
| Redis Streams | Redis 8.x docs | Entry id `ms-seq`, monotonic even on clock jumps; `XREAD … $` = only new; consumer group `>` = never delivered, explicit id = my pending list (PEL) until `XACK`; `XAUTOCLAIM`/`CLAIM min-idle` reclaims a dead consumer's pending; `MAXLEN ~` approximate trim. → PEL is exactly "delivered, not acknowledged"; `ack_wait` + reclaim is the redelivery model. | https://redis.io/docs/latest/develop/data-types/streams/; https://redis.io/docs/latest/commands/xreadgroup/ |
| MQTT 5 | OASIS 5.0 (2019) | **Retained message**: server "MUST replace any existing retained message for this topic and store" it "so that it can be delivered to future subscribers"; zero-byte payload clears it (§3.3.1.3). Wildcards `+` one level, `#` tail incl. parent (3.1.1 §4.7.1.2–3). Shared subscriptions `$share/<group>/<filter>` = queue group. Message Expiry Interval per message (§3.3.2.3.3). → retained = `last_per_subject`; per-message TTL is a payload field, not a bus feature. | https://docs.oasis-open.org/mqtt/mqtt/v5.0/os/mqtt-v5.0-os.html; https://docs.oasis-open.org/mqtt/mqtt/v3.1.1/os/mqtt-v3.1.1-os.html; https://www.hivemq.com/blog/mqtt5-essentials-part7-shared-subscriptions/ |
| ZeroMQ | libzmq docs (latest) | No broker; **patterns as socket types**: REQ/REP lockstep, PUB fan-out that drops at the high-water mark ("shall instead be dropped until the mute state ends"), PUSH round-robin / PULL fair-queue, XPUB/XSUB forward subscriptions upstream. → we keep a broker (the logic process) and never drop silently; what we take is "subscriptions are messages", so a relay forwards interest, not everything. | https://libzmq.readthedocs.io/en/latest/zmq_socket.html |
| Apache Pulsar | docs/next | **Tiered storage**: sealed ledger segments are "safely offloaded to external storage", metadata repointed, local copy deleted after a delay (default 4 h), "the consumer receives the data as if it were still in BookKeeper". → our cold segments go to `Blobs` ([[storage]]) and a `gap{cold: digest}` tells a replaying client where. | https://pulsar.apache.org/docs/next/concepts-tiered-storage/ |
| Fluvio | crate 0.50.1 2025-07-04, Apache-2.0; docs 0.18.1 | Rust, SC + SPUs as a cluster, topics/partitions/replicas, Kubernetes Helm. A server system, not a library. Nothing to take beyond "Rust can do this". | https://crates.io/api/v1/crates/fluvio; https://www.fluvio.io/docs/fluvio/concepts/architecture/overview |
| Apache Iggy | crate 0.11.1-edge.1 2026-10-02, Apache-2.0; Incubator since 2025-02-04 | Rust, "persistent message streaming platform"; partitions in segment files named by 20-digit start offset with `.log` + `.index` (offset, timestamp); server-side consumer and group offsets under `offsets/`; QUIC/TCP/WS/HTTP. Standalone server, no in-process mode. → the `.index` of (offset, timestamp) per segment is what SQLite gives us for free. | https://iggy.apache.org/docs/introduction/architecture; https://incubator.apache.org/projects/iggy.html |
| libp2p gossipsub | spec v1.0/v1.1; crate 0.50.1 2026-10-19 MIT | Mesh degree D=6 (D_low 4, D_high 12), eager push to mesh, lazy IHAVE/IWANT to the rest, GRAFT/PRUNE, `seen_ttl` 2 min dedupe by message id, peer scoring in v1.1. No ordering or delivery guarantee. → phase 3 only; already the shape of [[networking]] §4. | https://github.com/libp2p/specs/blob/master/pubsub/gossipsub/gossipsub-v1.0.md, …/gossipsub-v1.1.md |
| iroh-gossip | 0.101.0 2026-06-15 MIT/Apache-2.0 | HyParView + PlumTree over an iroh `Endpoint`, 32-byte `TopicId`, `broadcast(bytes)`, no persistence. Not usable over quinn. Comparison only ([[networking]] §4, §9). | https://crates.io/api/v1/crates/iroh-gossip |
| tokio `broadcast` / `watch` | tokio 1.54.1 2026-10-21 MIT | `broadcast`: bounded, power-of-two capacity, every receiver sees every value, a slow receiver gets `RecvError::Lagged(n)` and continues from the oldest retained — **a free `gap` signal**; `subscribe()` sees only later values. `watch`: "only retains the last sent value", `changed()` / `borrow_and_update()` — `last_per_subject` for one key. → in-process primitives; neither persists, neither matches wildcards, so the router sits above them. | https://docs.rs/tokio/latest/tokio/sync/broadcast/index.html; https://docs.rs/tokio/latest/tokio/sync/watch/index.html |
| Erlang/OTP `pg` + mailbox | kernel 11.0.4 / OTP 29.1.2 | `pg`: named process groups, `join`/`leave`/`get_members`, "strong eventual consistency", monitors deliver `{Ref, join, Group, Pids}`; groups vanish when empty, dead members auto-removed. Mailbox: asynchronous, per-pair ordering preserved, selective `receive`, no stated bound. → a subscription is a group membership that dies with the process (kittens' drop-unsubscribes); we bound the mailbox and say so with `gap`. | https://www.erlang.org/doc/apps/kernel/pg.html; https://www.erlang.org/doc/system/ref_man_processes.html |
| Cloudflare Queues + DO alarms | docs today | Queues: "at least once", duplicates possible, "generate a unique ID … use that ID as a primary key" to dedupe; limits 128 KB/message, retention up to 14 days, batch 100, 100 retries, 5,000 msg/s/queue. DO alarms: "a single alarm at a time", `setAlarm(ms)`, persisted with storage, "guaranteed at-least-once", 6 retries exponential from 2 s; for many timers "store them in storage and have `alarm()` process due events". → our cron/`ext` ticks are one `Store.alarm` per object over a due-times table ([[storage]] already names `alarm`). | https://developers.cloudflare.com/queues/reference/delivery-guarantees/; https://developers.cloudflare.com/queues/platform/limits/; https://developers.cloudflare.com/durable-objects/api/alarms/ |
| Postgres LISTEN/NOTIFY | PostgreSQL 18 docs | Payload "shorter than 8000 bytes", delivered at commit, only to currently listening sessions, same payload collapsed within a transaction, 8 GB queue then commit fails; "put it in a database table and send the key of the record". → the event is a one-line pointer, never the file ([[events]] «Docs side by side»). | https://www.postgresql.org/docs/current/sql-notify.html |
| NATS embedding from Rust | async-nats 0.50.0 2026-07-20 Apache-2.0; nats-server Go | No Rust server. Embedding = spawn the 17.7 MB Go binary as a child and talk `async-nats` over loopback. Measured idle cost above: 17.5–17.7 MiB RSS before the first stream. | §1 measured; https://crates.io/api/v1/crates/async-nats |

## 3. Rust crates for an embedded bus

| Crate | Version · date | License | Maturity | Persists | Read |
|---|---|---|---|---|---|
| `async-nats` | 0.50.0 · 2026-07-20 | Apache-2.0 | official, 49.9M downloads, repo pushed today, core + JetStream | no — client only | the client for a future `bboxy bus bridge nats://` adapter; nothing for the bus itself |
| `nats` (sync) | 0.26.0 · 2026-03-19 | Apache-2.0 | deprecated in favour of async-nats | no | skip |
| `watermelon` | 0.6.1 · 2026-10-10 | MIT OR Apache-2.0 | third-party pure-Rust client with a `no_std` protocol crate, targets server ≥ 2.12 | no | the protocol crate shows a NATS subject matcher is small; otherwise skip |
| `ratsio` | 0.4.1 · 2022-01-17 | MIT | dead | no | skip |
| embeddable NATS-like Rust server | — | — | none found (GitHub search, crates.io) | — | does not exist |
| `tokio::sync::broadcast` / `watch` / `mpsc` | tokio 1.54.1 · 2026-10-21 | MIT | the runtime we already use | no | `mpsc` per consumer, `watch` for single-key state; `broadcast`'s `Lagged` is the model for `gap` |
| `postage` | 0.5.0 · 2022-04-14 | MIT | unmaintained 4.5 years | no | skip; tokio covers it |
| `bus` | 2.4.1 · 2023-08-20 | MIT OR Apache-2.0 | jonhoo; "lock-free, bounded, single-producer, multi-consumer, broadcast", sync blocking readers, no async | no | skip |
| `eventador` | 0.0.18 · 2021-02-15 | Apache-2.0 | dead; Disruptor ring, `TypeId` subscriptions | no | skip |
| `fluvio` | 0.50.1 · 2025-07-04 | Apache-2.0 | client SDK of a cluster product | server does | skip |
| `iggy` | 0.11.1-edge.1 · 2026-10-02 | Apache-2.0 | client SDK of an incubating server | server does | skip |
| `rumqttd` | 0.20.0 · 2025-09-29 | Apache-2.0 | embeddable Rust MQTT broker (bytebeam) | retained + in-flight only | the only embeddable Rust broker found; brings MQTT's `+`/`#` and QoS, not our envelope or SQLite log — skip, noted for completeness |
| `rusqlite` | 0.40.2 · 2026-08-08 | MIT | decided in [[storage]] | yes | the log |

(`source: https://crates.io/api/v1/crates/<name>` for every row, fetched today.) Conclusion: no crate is the bus. The bus is ~2k lines of ours over `tokio` + `rusqlite`, and the only pubsub crate that enters the tree later is `async-nats`, as an optional bridge.

## 4. Design — the minimal bus (proposal)

Lens: [[vision]] LESS IS MORE and the composability test; JetStream's model with everything that serves a cluster removed. Breaks when: a swarm needs competing consumers (queue groups, §4.9) or cross-machine ordering (never promised).

### 4.1 Subject grammar

NATS semantics exactly: tokens, `*` matches one token, `>` matches one or more tokens and only at the end, publishers name full subjects, subscribers use patterns, no whitespace, case-sensitive, ≤ 256 bytes, ≤ 16 tokens.

**Separator: `/`, not `.`** (deviation from the brief, grounds): every topic in [[events]] is already written with `/`; doc paths (`.bboxy/specs/events.md`), channel names and cloud object names contain `.`, so a `.` separator forces escaping in three producers, while `/` is a token character nowhere. With `/`, `doc/path/.bboxy/specs/*` is the filesystem glob it looks like, and `>` is `**`. A bridge to a real NATS is a one-character rewrite. If the Oracle keeps `.`, the cost is ~20 lines of percent-escaping in `doc/path` and the examples here read `doc.path.%2Ebboxy.specs.*`.

**Kind is the last token.** `subject = <topic>/<kind>`: `seat/01J…/state`, `doc/01J…/modified`, `task/01J…/report`, `swarm/01J…/stream/message`, `ext/github/pr_opened`, `mux/seat_spawned`. The subscription `filter: kind list` in [[events]] disappears: it is the pattern. The envelope keeps `topic` and `kind` as fields for readability; the bus matches on `subject` only. Producers never publish on a bare parent (`swarm/<id>` becomes `swarm/<id>/presence`, `swarm/<id>/audit`), so `swarm/<id>/>` is complete.

**Two tiers, one grammar** (NATS core vs JetStream): `inbox/>` and `seat/*/output` are **fan-out subjects**, delivered to live consumers and never appended (the byte ring is output's store; a reply nobody is waiting for is worthless). Everything else is a **stream subject** and is appended before fan-out.

### 4.2 Stream per object, one sequence per stream

Each durable object's `events.sqlite` ([[storage]]) is one stream capturing the subjects rooted at that object: the seat object `seat/<id>/>`; the swarm object `swarm/<id>/>`, `task/*/>` for its tasks, `dm/<seat>/>` for its seats; the `.bboxy` index object `doc/>`, `artifact/>`; the supervisor `mux/>`; the account object `ext/>`, `cloud/>`.

```sql
CREATE TABLE events (
  seq      INTEGER PRIMARY KEY,         -- the stream sequence; rowid
  id       BLOB NOT NULL UNIQUE,        -- 16-byte ULID; dedupe
  ts       INTEGER NOT NULL,            -- unix ms, producer clock
  subject  TEXT NOT NULL,
  source   TEXT NOT NULL,               -- seat/doc/ext/github/…
  payload  BLOB NOT NULL,               -- JSON (or zstd for large)
  signer   BLOB NOT NULL, sig BLOB NOT NULL, prev BLOB NOT NULL  -- [[storage]] Events: Ed25519, hash chain
);
CREATE INDEX events_subject_seq ON events(subject, seq);
CREATE TABLE consumers (name TEXT PRIMARY KEY, filter TEXT, cursor INTEGER, acked INTEGER, deliver TEXT, policy TEXT, updated INTEGER);
CREATE TABLE pending (consumer TEXT, seq INTEGER, delivered_at INTEGER, deliveries INTEGER, PRIMARY KEY(consumer, seq));  -- Redis PEL
CREATE TABLE stream (first_seq INTEGER, cold TEXT);  -- retention state
```

**Sequence is per stream, not per topic** (change to [[events]] «one sequence per topic»): JetStream, Kafka and Iggy all order the log once and let filtered consumers skip. Per-subject order is a sub-order of stream order, so nothing is lost, and `since` is one integer instead of a map. `(subject, seq)` is indexed for `last_per_subject` and filtered replay.

### 4.3 Consumer

```
consumer = { name?: str,                      -- durable iff present; else dies with the connection
             filter: [pattern],               -- §4.1 patterns, OR-ed
             start: new | all | last_per_subject | seq:<n> | time:<ms>,
             ack: none | explicit,            -- explicit → pending table, ack_wait, max_deliver
             deliver: socket | wake | gui | webhook | transport,
             queue_max: frames/bytes }        -- default 1024 / 4 MiB ([[kittens-events]] §4.6)
```

- `ack: none` for `socket` and `gui`: the cursor advances when the frame is written; overflow coalesces latest-wins per subject for state-like kinds and emits `gap{from_seq,to_seq,reason:slow_client}` for the rest — the rules already in [[kittens-events]] §4.6.
- `ack: explicit` for `wake` and `webhook`: a delivered seq enters `pending`; the adapter's evidence ([[messaging]] native-queued / native-written / native-unknown / application-acknowledged) is written as a `receipt` event and the matching ack clears `pending`; `ack_wait` without ack → redeliver up to `max_deliver`, then a `receipt{state: dead}` and the Oracle seat is told. `native-unknown` is never auto-redelivered ([[messaging]]): the pending row stays, marked, visible.
- `start: last_per_subject` on `seat/*/state` is how the GUI, a reconnecting client or a new Oracle seat gets the current state of everything in one read. No snapshot API.
- Named consumers are the seat auto-subscriptions in [[events]]: a delegated seat gets consumer `seat:<id>:task:<task>` with filter `[task/<id>/>, doc/<task doc>/>, doc/<derived_from…>/>]`; it expires with the grant.

### 4.4 Router: publish, match, fan out

One router task in the logic process (kittens `bus.rs` shape, [[kittens-events]] §1.1): it owns the subscription table and the per-stream SQLite handle. `publish(subject, payload)` → sign, chain, `INSERT` (one WAL transaction; `UNIQUE(id)` violation returns the existing `seq`: idempotent) → match every live consumer's patterns → push one `Arc<Envelope>` into each matching bounded `mpsc`. Subscribe acks after registration, so nothing published after the ack is missed (kittens rule). Matching is a linear scan of patterns per publish until measured otherwise (**Gap** below); a token trie is the known fix.

### 4.5 Request-reply

`reply_to: inbox/<seat>/<nonce>` on the envelope; the seat holds one fan-out consumer on `inbox/<seat>/>` for its lifetime. The replier publishes to `reply_to`. If a publish with `reply_to` matches zero consumers, the router publishes `inbox/<seat>/<nonce>` with `kind: no_responders` immediately (NATS). The control plane's `req` → `ack` ([[mcp-control-plane]], [[kittens-events]] §4.4) is this with the socket as the inbox.

### 4.6 Dedupe and retention

Dedupe: `UNIQUE(id)` per stream, forever rather than JetStream's 2-minute window — SQLite makes the index cheaper than a window and [[messaging]] wants "duplicate retries yield one inbox record" after a restart, which a window cannot promise. Relayed or mirrored events keep `(origin_stream, origin_seq)` in `provenance`; a mirror adds `UNIQUE(origin_stream, origin_seq)`.

Retention: per object policy `{max_age?, max_bytes?, max_per_subject?}`, default none — [[storage]] «never delete». When a limit is hit the oldest rows are moved as one zstd segment to `Blobs`, `stream.first_seq` advances, `stream.cold` lists `(from_seq, to_seq, digest)`. A `since` below `first_seq` → `gap{reason: evicted, cold: digest}`; the client may fetch the segment. `max_per_subject` is the only knob likely to be set in v0.1 (coalesced `doc/*/modified`, `seat/*/state`).

### 4.7 Messages as an abstraction on top

| [[messaging]] concept | Bus form |
|---|---|
| message to a swarm / DM / channel | event `swarm/<id>/stream/message`, `dm/<seat>/message`, `channel/<name>/message` (phase 3); `payload` = the signed messaging envelope, byte-exact, verified before append |
| sender-stored | the seat's own stream: `seat/<id>/outbox/message` appended first (the outbox is a subject, not a table) |
| recipient-stored | the swarm/dm stream's `seq` for that message |
| native-queued / -written / -unknown / application-acknowledged | `receipt` events: `dm/<seat>/receipt`, `swarm/<id>/stream/receipt` with `{message_id, state, native_id?}`; a message's status is the fold of its receipts; the GUI badge is a consumer on `*/receipt` |
| presence, mute/unmute notices, task bodies | `swarm/<id>/presence`, `swarm/<id>/mute`, `task/<id>/delegate|release|report|accept|reject` — the `body` variants become kinds; the signed envelope is unchanged |
| inbox / outbox tables | gone: outbox = `seat/<id>/outbox/>` plus the **forwarder**, a consumer that republishes each outbox message onto its destination subject under the current grant; inbox = the seat's wake consumer cursor plus `pending` |
| mute | two paused consumers: the seat's wake consumer on `swarm/<id>/>` and its forwarder on `seat/<id>/outbox/>`; cursors stop, the log keeps everything, the GUI's consumer is untouched; unmute resumes both, the forwarder re-checks the grant per message and leaves a revoked one visibly blocked ([[messaging]] «Swarm mute») |
| priority Oracle > worker, token bucket | inside the wake consumer's dispatcher: it reads in `seq` order into two class queues keyed by the signed `priority_class`, Oracle bucket 10/s burst 30, FIFO by `seq` within a class; `report` bypasses the bucket. A scheduler on a consumer, not a bus feature |
| dedupe by author + UUID | `UNIQUE(id)` in the destination stream, where `id` is the messaging envelope's message UUID |
| per-seat serialisation | one wake consumer per seat, one dispatcher, the adapter's input arbiter |
| task frontmatter mirror ([[work-items]]) | a consumer on `task/*/>` in the index object writing `assignee`/`status`/`updated` |

Evidence states, mute and priority therefore cost the bus nothing; they are consumers and payloads.

### 4.8 Composability test

Autoresearch ([[vision]]): the Oracle seat's harness runs `bboxy bus sub 'artifact/*/claimed' --since <seq> | jq …` from a shell, or a named consumer `oracle:experiments` with `deliver: wake`; the task doc edit is a `doc/<id>/modified` event; the next task is a file write that produces `task/<id>/delegate`. Every arrow in the vision table is a `publish` or a consumer. `bboxy bus pub <subject> <json>` and `bboxy bus sub <pattern> [--since N|--last-per-subject]` are the two verbs; the seat socket of [[kittens-events]] §4 is the same frames addressed to one seat.

### 4.9 Not built, by name

Clusters, Raft, replicas, supercluster gateways, leaf-node protocol, accounts/NKeys/JWT, mirrors/sources (except one relay consumer in phase 2), subject transforms, republish, `Interest`/`WorkQueue` retention, push flow control, rate limits, sampling, message headers, KV/object APIs, text protocol listener, WebSocket/MQTT ports, **queue groups** (no competing consumers in v0.1: delegation is addressed to a seat; when a swarm wants a pool of interchangeable executors it is a `group: name` field on the consumer and a random pick in the router, ~50 lines, added on evidence), gossip (phase 3, [[networking]] §4 unchanged).

### 4.10 Size and crates

Order of magnitude **2k lines** plus tests: subject match ~100; store (append, sign, chain, range read, `last_per_subject`, consumers, pending, dedupe) ~450; router and bounded queues with coalescing and `gap` ~450; consumer start/ack/redeliver/replay ~350; request-reply and no-responders ~80; retention to cold segments ~200; webhook out ~150; `bboxy bus pub|sub` CLI ~150. The seat-socket adapter is already specified in [[kittens-events]] §4 and is the `deliver: socket` path.

Crates: `tokio` 1.54 (runtime, `mpsc`, `watch`), `rusqlite` 0.40 bundled (decided), `serde`/`serde_json`, `ulid`, `ed25519-dalek` (already for [[provenance]]), `zstd` (already for rings), the HTTP client the GitHub ingress already uses for webhook out. No pubsub crate.

## 5. Embed `nats-server` vs own ~2k-line bus over SQLite

| | Embed nats-server 2.15.1 (child process + `async-nats` 0.50) | Own bus (§4) |
|---|---|---|
| Can it be linked into the Rust binary | No. Go only. It is a third process beside supervisor and logic, to spawn, supervise, port-assign, notarize and ship | Yes; it is a module of the logic process |
| Binary / idle memory | +17.7 MB Mach-O, 17.5–17.7 MiB RSS before any stream (**measured**), plus the Go runtime's sawtooth | ~0; rusqlite is already linked |
| Source of truth | Two: JetStream file store and our signed, hash-chained `events.sqlite` ([[storage]] requires the latter for provenance; JetStream rows are not Ed25519-signed by seat keys). Either dual-write or treat JetStream as a cache | One |
| Authorization | NATS accounts/permissions know nothing of seat grants; all seats would share one account and we enforce grants in our own layer anyway — NATS enforces nothing we need | Grants checked at subscribe and publish in the router, as [[kittens-events]] §4.8 |
| What we get free | Subject matching, consumers with every deliver/ack policy, dedupe window, KV/object store, mirrors, clustering, a battle-tested server and ecosystem (CLI, exporters) | Only what we write; §4 is a strict subset of JetStream's model, written against our envelope |
| What we pay | Version coupling to a Go release train, a Unix-socket or loopback hop per event, operational surface (store dir, limits, monitoring port), and the composability test gains nothing: `nc`/`jq` talk to our seat socket either way | ~2k lines and the bugs in them; JetStream's semantics are the spec to test against |
| Cloud / enterprise | Leaf nodes and gateways are the strongest argument: an enterprise that already runs NATS plugs in natively | A `bboxy bus bridge nats://…` adapter over `async-nats` (~200 lines) is the "webhook out" peer; subject grammar identical by construction, so it is a rewrite of one character per subject |
| Verdict | Right if we were a service mesh with many hosts and no provenance requirement | Right for one logic process per machine owning signed per-object logs; it is what the [[storage]] decision already implies |

Recommendation: **own bus, NATS's grammar and consumer model verbatim, nothing of its topology.** Keep `async-nats` out of the tree until an enterprise bridge is requested.

## 6. Gaps

- **Gap:** fan-out at 1000 seats × a few patterns each — linear match per publish vs token trie; no measurement (carried from [[events]]). Measure in W2.2 with synthetic subscriptions before adding a trie.
- **Gap:** `/` vs `.` separator is an Oracle decision; this file recommends `/` with grounds in §4.1 and names the ~20-line cost of `.`.
- **Gap:** per-stream sequence (§4.2) changes [[events]] «one sequence per topic» and the `since` shape in [[kittens-events]] §4.3 from `{gen, seq}` to a single stream `seq` per object; the generation boundary rule stays as a `gap{reason: generation_changed}`. Needs the Oracle's edit of both specs.
- **Gap:** cold-segment fetch (`gap{cold: digest}` → read from `Blobs`) has no client API written; nothing in v0.1 evicts, so it is deferred with the retention knob.
- **Gap:** queue groups / competing consumers are excluded from v0.1 on the reading that delegation is addressed; the composability test does not need them today. Re-check when a swarm wants an executor pool.
- **Gap:** `ack_wait` and `max_deliver` defaults for `wake` consumers are not set here; they interact with [[harness-adapters]] readiness and the never-retry rule for `native-unknown`. Decide with the adapter contract.
- **Gap:** cross-stream subscription (a seat in swarm A subscribing to `doc/>` in the index object) is a router concern across several SQLite handles in one process; fine locally, unspecified for a remote object until the phase-2 relay consumer exists.
- **Gap:** the NATS protocol page's claim of UNIX-domain-socket transport was not tested; irrelevant unless embedding is reconsidered.
- **Gap:** no nats-server measurement under load or with streams populated; the idle numbers above bound only the floor.

Links: [[events]] [[messaging]] [[networking]] [[storage]] [[vision]] [[kittens-events]] [[identity]] [[provenance]] [[work-items]] [[mcp-control-plane]] [[harness-adapters]]
