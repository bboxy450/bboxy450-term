---
id: "1720WDHH64ZZM3D52STEYEKFPX"
type: "research"
number: 112
title: "State backend — SQLite per durable object, blobs by digest"
created: "2026-10-21T22:45:47Z"
updated: "2026-10-24T00:10:38Z"
---
# State backend — SQLite per durable object, blobs by digest

Status: research with a recommendation. Written 2026-10-21 by a research subagent for Vesper Greywind <ORACLE>. Facts dated as fetched or read on 2026-10-21; every "measured" line names its command, every "estimated" line names its assumption.

Links: [[mux-daemon]] [[messaging]] [[work-items]] [[mcp-control-plane]] [[vision]]

Operator question: "our backend for state should be sqlite probably, right? unless you have better ideas; research and make a call." Added scope: evaluate "one durable object per thing" (seat, swarm, account each an actor with its own SQLite file, ring and alarm; local directory now, Cloudflare Durable Object later) against one central database.

**One line:** SQLite through `rusqlite` 0.40.2 (`bundled`, SQLite 3.53.2, FTS5 and JSON1 compiled in), one small database file **per durable object** (account, swarm, seat) behind a `Store` trait, large bytes in a content-addressed `Blobs` store, one append-only signed event log per object, a rebuildable account-level index for cross-object queries. No CRDT. No graph database. No ORM.

---

## 1. Inventory: what the store must hold

Derived from the specs; the right-hand column is where it lives in the recommended layout.

| State | Source | Shape and access pattern | Object |
|---|---|---|---|
| Seat registry: `SeatRecord` with `seat_id, owner_device_key, location, harness_kind, harness_session_id, launch_spec, role_grant_reference, muted, policy_epoch, lifecycle, lifecycle_revision, close_record, process_generation, supervisor_boot_id, child_identity, ring_format_version, checkpoint_reference, committed_record_sequence` | `mux-daemon.md:54-65` | one mutable row, point lookup by seat id, rewritten on every lifecycle/generation/mute change | seat |
| Close lifecycle: close revision, grace deadline, restoration handle (swarm/tab/split position, ratios) | `mux-daemon.md:83-89` | mutable row plus one event per transition | seat (deadline mirrored to supervisor) |
| Logic lease, accepted input ids and receipts (rejected / fully written / partial-unknown), close expiry | `mux-daemon.md:29`, `:97`, `:104`, `:111` — owned by the supervisor | append per input, point lookup by input id, survive logic restart | **supervisor's own file**, not the seat object |
| Ring segments, checkpoints, committed-boundary marker | `mux-daemon.md:117-144` | own record log, CRC32C, not SQL | seat `ring/` (outside SQLite, see §5) |
| Operations journal: `op_id`, argument digest, status `accepted/applied/rejected/partial/unknown`, per-effect receipts | `mcp-control-plane.md:41`, `:175` | insert once, update status, point lookup by op_id; same id with different digest = conflict | account (control service owns it); seat-scoped effects copied as events |
| Audit ledger: attempt before effect, outcome after; actor account/device/seat, generations, grant epoch, tool, target ids, request digest, redacted summary, admission decision, timestamps, evidence; denials too | `mcp-control-plane.md:171-175` | append-only, paged read by cursor/target/op (`audit_read`) | account |
| Inbox: original envelope bytes and signature, `(author, id)` dedupe, receiver-local commit sequence, class, evidence state, held reason | `messaging.md` "Delivery, rooms and restart behavior", "Priority without deleting messages" | append, FIFO by class, scan "unattempted after seq", dedupe lookup | seat (the owner identity) |
| Outbox: envelope, state, attempts, receipts, held/blocked reason; retries reuse the same id | same | append, update state, scan pending | seat |
| Evidence states per message/attempt: sender-stored, recipient-stored, native-queued, native-written, native-unknown, application-acknowledged | `messaging.md` | append per attempt, never overwrite an unknown | seat |
| Presence shadows keyed `(account, seat)`, mute notices with policy epoch | `messaging.md` "Signed envelope sketch" | upsert last-reported state, read for sidebar | swarm |
| Grants: device grant, seat grant (role, class, swarm, destinations, muted, epoch, expiry), role definitions per account | `messaging.md` "Identity", `vision.md:35` | small, point lookup by digest/epoch, every call rechecks | account (definitions, device grants); seat (its current grant reference) |
| Reservation ledger for spawn caps (running, starting, reserved, closed-grace) | `mcp-control-plane.md:161-167` | counter rows under one transaction | swarm; aggregate in account |
| Layout: swarm → tab → split tree, layout revision | `mcp-control-plane.md:39`, `:56-58` | one JSON blob per tab or per swarm, revision check | swarm |
| Task messages mirrored to frontmatter; delegation state | `messaging.md` "Task messages", `work-items.md` Delegate row | the markdown is canonical; the store keeps the index and the last delivery evidence | git (`.bboxy/`) + account index |
| Artifact events `{artifact_id, seat, kind, digest, ts, grade, source}` | `work-items.md` Artifacts | append; newest N mirrored into frontmatter, rest in `.bboxy/log/` | seat event log → account index → `.bboxy/log/` export |
| `.bboxy` index: frontmatter fields, typed and weighted wikilinks, the five graphs (markdown, artifact, task, execution, messaging) | `work-items.md` "The graphs the terminal builds" | rebuilt from files on watch; graph queries; FTS over bodies | account **index**, derived, deletable |
| Settings cache, harness records, role definitions, templates, caps | `vision.md:34`, `:54`, `mcp-control-plane.md:161` | key/value JSON | account |
| Supervisor boot id, protocol/format versions | `mux-daemon.md:101` | one row | supervisor file |

Stays outside the store: PTY byte rings and checkpoints (own record log per `mux-daemon.md:121-132`); markdown items (`work-items.md`: "Git is the database"; "No database, no sidecar JSON" applies to the items themselves, the index is a cache); secrets (root key in gpg-agent/Keychain per `vision.md:46`; the store holds references and digests, `mux-daemon.md:69`). ICQ stores raw signing keys in SQLite with 0600 (`icq-0.20.2/src/store.rs:46`, `:240`); we do not copy that.

---

## 2. SQLite facts (verified)

| Fact | Evidence |
|---|---|
| `rusqlite` 0.40.2, MIT, "actively-developed", published 2026-08-08; `libsqlite3-sys` 0.38.2 bundles **SQLite 3.53.2** (sqlite3.c is 9.5 MB source, ~269k lines) | crates.io API 2026-10-21; `~/.cargo/registry/src/*/libsqlite3-sys-0.38.2/sqlite3/sqlite3.h:149`; `rusqlite-0.40.2/Cargo.toml:60-168` |
| Bundled build compiles `-DSQLITE_ENABLE_FTS5 -DSQLITE_ENABLE_JSON1 -DSQLITE_ENABLE_RTREE -DSQLITE_THREADSAFE=1` | `libsqlite3-sys-0.38.2/build.rs:159-166` |
| rusqlite features we need: `bundled`, `backup`, `hooks` (update/commit hooks for the event bus), `load_extension` (sqlite-vec later), `serialize`, `functions`, `window`, `session` | `rusqlite-0.40.2/Cargo.toml:75`, `:113-168` |
| WAL: "readers do not block writers and a writer does not block readers"; "there can only be one writer at a time"; readers see a snapshot | source: https://www.sqlite.org/wal.html |
| WAL requires all processes on one host (mmapped `-shm` wal-index); default autocheckpoint at 1000 pages (~4 MB); a checkpoint cannot complete while any reader overlaps, so a database with an always-present reader grows its WAL without bound; `wal_checkpoint(TRUNCATE)` or reader gaps fix it | same |
| Recursive CTEs: queue-based evaluation, `UNION` dedupes and stops cycles, `ORDER BY` in the recursive select turns FIFO/breadth-first into depth-first or a priority queue, `LIMIT` bounds recursion; "programmers should strive to use UNION ALL instead of UNION when feasible"; indexes "help performance for large graphs" | source: https://www.sqlite.org/lang_with.html |
| FTS5: external-content tables (`content=`) index another table and are kept in sync by three triggers; `bm25()` ranking (lower is better), `rank` column; `trigram` tokenizer gives substring and indexed `LIKE '%x%'`; prefix indexes `prefix='2 3'` | source: https://www.sqlite.org/fts5.html |
| Limits: max db 4294967294 pages (17.5 TB at 4 KiB pages); default max blob 1e9 bytes; `SQLITE_MAX_ATTACHED` default 10, hard max 125 | source: https://www.sqlite.org/limits.html |
| `VACUUM` needs up to 2× the file in free space, cannot run inside a transaction, is a write op; freed pages are reused without vacuum, the file just does not shrink; `auto_vacuum` fragments; `VACUUM INTO 'path'` makes a consistent minimal backup without touching the original | source: https://www.sqlite.org/lang_vacuum.html |
| `sqlite-vec` 0.1.9 (0.1.10-alpha.4 newest), MIT/Apache-2.0, "pre-v1, so expect breaking changes", a loadable extension, Rust crate `sqlite-vec` | crates.io API; https://github.com/asg017/sqlite-vec |
| ICQ 0.20.2 schema and approach: `Connection::open`, chmod 0600, `PRAGMA journal_mode=WAL; synchronous=FULL; busy_timeout=3000`, then `CREATE TABLE IF NOT EXISTS` for `identities, inbox(seq AUTOINCREMENT, owner, id, digest, envelope JSON text, harness, UNIQUE(owner,id)), outbox(owner,id,envelope,state,error,receipt), contacts, rooms, ignored, settings(name,data)`; migration = `PRAGMA table_info` then `ALTER TABLE ADD COLUMN` when a column is missing; sub-modules add their own tables (`trust/attention.rs:59-66`, `notifications.rs:95-97`, `settlement/journal.rs:295-305`, `trust/ledger.rs:197-203`) | `icq-0.20.2/src/store.rs:42-66` |
| ICQ inbox admission: one transaction per receive; `SELECT seq,digest ... WHERE owner=? AND id=?` → same digest returns the existing receipt `recipient-stored`, different digest is "message ID conflict"; capacity checks (`CAP = 10_000` rows, byte cap) inside the transaction; `INSERT`, `last_insert_rowid()` is the receiver sequence; `Notify` wakes workers after commit | `store.rs:17`, `:388-479` |
| ICQ outbox: `INSERT ... state='pending'`, retries read the same envelope by id | `store.rs:577-600` |
| ICQ queries JSON in place: `json_extract(envelope,'$.author')`, `json_extract(data,'$.context.kind')` | `store.rs:498`, `:946`; `focus.rs:138` |
| ICQ corruption handling: read-only open with `busy_timeout(250)`, `SELECT EXISTS(... sqlite_master ...)`, treat `ErrorCode::NotADatabase` as "no state", never repair during a read. No `.bak`, `integrity_check` or `wal_checkpoint` call exists in icq 0.20.2 (`grep -rn '\.bak\|integrity_check\|wal_checkpoint' src/` → none) | `focus.rs:60-81` |
| One `Arc<Mutex<Connection>>` for the whole process; all 64-identity / 64-room caps are `count(*)` checks in code | `store.rs:60`, `:233`, `:731`, `:925` |
| Tachyon (Swift, `~/mydev/tachyon`) is a **reader** of other apps' SQLite: `sqlite3_open_v2(..., SQLITE_OPEN_READONLY | SQLITE_OPEN_URI)` and `PRAGMA query_only=ON` against Cursor, Claude Desktop and OMP databases; it has no store of its own | `Sources/Tachyon/Providers/{Cursor,ClaudeDesktop,Omp}Provider.swift:262,933-941,106` |

The "claim" of the ICQ brief that it uses a `.bak` pattern is **not** in 0.20.2 source; recorded as not found.

---

## 3. Candidates

Access patterns to score against: append-heavy logs (inbox, outbox, audit, events), point lookups by ULID, graph traversal over 100k edges, FTS over messages and markdown, 1000 objects, Rust, one binary, macOS 14 first, cloud later.

| Candidate | Version / license (crates.io 2026-10-21) | Fit for our patterns | Multi-process | Maturity | Binary cost | Verdict |
|---|---|---|---|---|---|---|
| **SQLite / rusqlite** | 0.40.2 MIT; SQLite public domain | all four: tables, indexes, recursive CTE, FTS5, JSON1; same engine Cloudflare DOs expose | WAL, one host, one writer per file, many readers | 25 years; the DO storage engine | ~1.5–2 MB object code for the bundled amalgamation (estimated from 9.5 MB source; not measured) | **pick** |
| redb | 4.4.0 MIT/Apache-2.0, updated 2026-10-13 | KV only; MVCC, ACID, savepoints; no SQL, no FTS, graph by hand | not documented; single process assumed | "stable file format", maintained | small, pure Rust | no: we would rebuild indexes, FTS and query planning |
| fjall | 3.1.12 MIT/Apache-2.0 | LSM KV; fast batch writes; keyspaces; optional transactions | "A single database may not be loaded in parallel from separate processes" | stable format policy, MSRV 1.90 | small, pure Rust | no: same gap as redb; LSM compaction is wrong for a 1000-small-files layout |
| heed / LMDB | 0.22.1 MIT; LMDB OpenLDAP Public License | KV, zero-copy, MVCC; no SQL/FTS | yes: LMDB "supports concurrent read and write access from multiple processes", one writer, readers lock-free; map size must be preset; long readers grow the file | LMDB mature (Meilisearch uses heed; recalled, not fetched) | small | no: pre-sized maps per object × 1000 objects, no SQL |
| sled | 0.34.7 stable, 1.0.0-alpha.124 newest, last update 2024-10-11 | KV | "doesn't support multiple open instances" | README: "beta", "unstable", format changes before 1.0, "if reliability is your top priority, use SQLite" | pure Rust | no |
| rocksdb | 0.25.0 Apache-2.0 | LSM KV, column families | single process | mature C++ | compiles RocksDB C++ in `librocksdb-sys`, multi-minute builds, tens of MB (known; not measured here) | no |
| duckdb | 1.10506.1 MIT | columnar analytics; weak for point updates and small transactions | single writer process | mature | large C++ build | no for state; maybe later for analytics over exported event logs |
| turso (ex-limbo) | 0.8.3 MIT (0.9.0-pre.1) | Rust SQLite rewrite, file-format compatible "not at 100% yet", MVCC `BEGIN CONCURRENT`, FTS experimental, multi-process WAL experimental | experimental | FAQ: "not yet reached 1.0", "keep independent backups" | pure Rust | **watch**: same file format, swap-in candidate after 1.0 |
| sqlx 0.9.0 / sea-orm 2.0.4 | MIT/Apache-2.0 | async pool + compile-time checked SQL / ORM | n/a (uses SQLite) | mature | pulls tokio runtime into the store, async over a synchronous embedded engine | no: rusqlite + a `spawn_blocking` actor is simpler and matches the single-threaded DO model |
| rusqlite_migration 2.6.0 / refinery 0.10.0 | Apache-2.0 / MIT | numbered migrations over `user_version` / a table | n/a | mature | tiny | optional; hand-rolled is 30 lines (§4) |
| cr-sqlite (vlcn) | no crates.io crate; GitHub MIT, no visible releases, nightly Rust to build | CRDT columns (LWW, counter, fractional index) for multi-writer merge; writes 2.5× slower | loadable extension on every connection | "main may not be 100% stable" | extension | **not needed**: objects migrate, they do not merge (§7f) |
| sqlite-vec | 0.1.9 MIT/Apache-2.0 | `vec0` virtual tables for embeddings | extension | pre-v1 | ~small | later, behind `load_extension`, for semantic search over the index |
| petgraph | 0.8.3 MIT/Apache-2.0 | in-memory graph algorithms | n/a | mature, 566M downloads | tiny | **yes**, for the in-memory graph rebuilt from the index (§6) |

Not a candidate: a graph database (Neo4j, Kùzu, SurrealDB). Reason in §6.

---

## 4. Patterns

**Event-sourced versus mutable rows.** Do both, per object, in one transaction: append the event, update the current-state row. The event log is the thing enterprises tail and the thing provenance signs; the row is what the GUI and MCP serve. No replay-from-zero requirement: a projection that is lost is rebuilt from the object's own `events`, and the index is rebuilt from `.bboxy/` files. Pure event sourcing would push every `self_status` through a fold; pure mutable rows would lose the "who did it, under which key" that [[work-items]] Provenance demands. ICQ uses mutable rows plus `inbox.seq` as its only log (`store.rs:47`); we add the explicit log because audit and artifact history need it.

**Schema migration.** Hand-rolled, as ICQ does but with a version: `PRAGMA user_version` holds the schema number; `migrations: &[&str]` is an array of SQL batches; on open, run every batch above the current version inside one transaction and set `user_version`. Each object class (account, swarm, seat, supervisor, index) has its own array. `rusqlite_migration` 2.6.0 does exactly this if the hand-rolled version grows; `refinery` and `sqlx migrate` add nothing for an embedded single-writer file. Never `ALTER` by probing `table_info` (ICQ `store.rs:53-59`): a version number is a fact, a column probe is a guess. Format-incompatible objects are refused and left in place, as `mux-daemon.md:156` requires for checkpoints.

**One database per machine, per repo, or per swarm?** Per **object**, where object ∈ {account, swarm, seat, supervisor, index}. Reasons: (1) it is the unit that moves to a cloud host or a DO, so local and cloud code share one `Store` trait; (2) corruption or a runaway WAL is contained to one seat; (3) the supervisor must own its own durable file anyway (`mux-daemon.md:29`: input IDs, lease) and must not share a writer with logic; (4) one writer per file is exactly SQLite's model, and DO's. Per repo makes no sense: the repo's state is `.bboxy/` in git; the index for a repo is a cache keyed by repo root inside the account index. Per machine (one file) is the simplest local shape and is what ICQ does; its cost appears the day a seat moves hosts and the day one 1000-seat file needs a VACUUM with 2× free space.

**Backup and corruption.** On open: `PRAGMA quick_check` (fast; `integrity_check` only on operator request or after a crash flag). On a failed open or `NotADatabase`: rename the file to `state.sqlite.corrupt-<ts>`, open fresh, rebuild what is rebuildable (index, projections), report the loss for what is not (inbox, audit) rather than silently continuing, matching `mux-daemon.md:140`'s "reject a torn suffix" rule. Nightly `VACUUM INTO` per object into `blobs/` (it is just another digest-named blob) gives a consistent snapshot without stopping writes. Checkpoint policy: `wal_autocheckpoint` default plus a `wal_checkpoint(TRUNCATE)` when the object goes idle, so a 1000-seat machine does not hold 1000 × 4 MB WALs. ICQ's `.bak` pattern is not in its source (§2); do not cite it.

**Durability pragmas.** `journal_mode=WAL`, `synchronous=NORMAL` for seat and index objects (durable across process crash, may lose the last transactions on power loss — acceptable for a derived index and for an inbox whose sender retries by id), `synchronous=FULL` for the supervisor file and the account audit/ops journal (an accepted op must survive power loss or `operation_get` lies). `busy_timeout=3000` like ICQ. `foreign_keys=ON`. Page size default 4096.

---

## 5. The "one durable object per thing" shape

### 5a. Objects and layout

```
~/Library/Application Support/bboxy/            (run dir; 0700; never in a repo)
  account/
    state.sqlite  (+ -wal, -shm)                 registry, grants, roles, settings, ops, audit, alarms
    events.sqlite                                 append-only signed log (see 5d)
  index/
    <repo-root-hash>.sqlite                       .bboxy index: items, links, FTS; deletable, rebuilt from files
  swarm/<ulid>/
    state.sqlite                                  layout, tabs, reservation ledger, presence shadows
    events.sqlite
  seat/<ulid>/
    state.sqlite                                  seat record, inbox, outbox, attempts, artifact events
    events.sqlite
    ring/                                         supervisor-owned segments + checkpoints (mux-daemon format v1)
  supervisor/
    state.sqlite                                  boot id, leases, input ids + receipts, close deadlines
  blobs/
    sha256/ab/cd/abcd…                            every large thing, by digest (5b)
```

One directory per object: `tar` it, `rsync` it, or load it into a DO. Two SQLite files per object (state and events) so the append-only log can be shipped and verified without the mutable state, and so a corrupt state file never takes the log with it.

Count at 1000 seats: ~2000 SQLite files, each 3 fds when open (db, -wal, -shm). Open connections lazily with an LRU of, say, 256; a seat with no traffic has its connection closed and its WAL checkpointed. macOS `launchctl limit maxfiles` soft default is 256 (read 2026-10-21 on this machine; the current shell has `ulimit -n` 1048576 because the profile raises it) — the daemon raises `RLIMIT_NOFILE` itself at start.

### 5b. `Store` trait

```rust
pub trait Store: Send {
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;
    fn put(&mut self, key: &str, value: &[u8]) -> Result<()>;
    fn delete(&mut self, key: &str) -> Result<bool>;
    fn list(&self, prefix: &str, after: Option<&str>, limit: usize) -> Result<Vec<(String, Vec<u8>)>>;
    fn sql(&mut self, stmt: &str, params: &[Value]) -> Result<Rows>;      // same text on both backends
    fn txn<T>(&mut self, f: impl FnOnce(&mut dyn Store) -> Result<T>) -> Result<T>;
    fn alarm_get(&self) -> Result<Option<u64>>;                            // one alarm per object, epoch ms
    fn alarm_set(&mut self, at_ms: u64) -> Result<()>;
    fn alarm_delete(&mut self) -> Result<()>;
}
```

`SqliteFileStore`: one `rusqlite::Connection`, WAL, the pragmas in §4, a `kv(key TEXT PRIMARY KEY, value BLOB)` table for get/put/list, arbitrary tables through `sql`, the alarm as one row in `kv` under `__alarm` plus an in-process timer wheel in the object host that re-arms every object's alarm at startup by scanning `alarm_get`. Synchronous API; the object host runs each object on one actor task so a `Store` call never contends — the same single-threaded-per-object rule DOs enforce.

`DoStore` sketch (Cloudflare, Rust via `worker` 0.8.7 Apache-2.0, workers-rs): `get/put/delete/list` → `ctx.storage().kv` (synchronous KV API on SQLite-backed objects); `sql` → `ctx.storage().sql().exec(stmt, bindings)`, consumed synchronously before any await; `txn` → `transactionSync`; alarms → `getAlarm/setAlarm/deleteAlarm`. Facts, source https://developers.cloudflare.com/durable-objects/api/sqlite-storage-api/ and `/platform/limits/` (2026-10-21): FTS5, JSON and math functions available; SQL statement ≤ 100 KB; row/string/BLOB ≤ 2 MB; 100 columns per table; 100 bound parameters; `LIKE` pattern ≤ 50 bytes; ~1000 req/s soft limit per object; "Each individual Object is inherently single-threaded"; one alarm per object, at-least-once, retried with exponential backoff from 2 s up to 6 times (`/api/alarms/`); 30-day point-in-time recovery through bookmarks; storage per object **10 GB on Paid per the limits page, "currently 1 GB, planned 10 GB at GA" per the concepts page — the two pages disagree, recorded as such**. Pricing (`/platform/pricing/`): 25 billion rows read and 50 million rows written per month included on Paid, then $0.001 and $1.00 per million; 5 GB-month included then $0.20/GB-month; storage billing started 2026-01-07. Migrations (`/reference/durable-objects-migrations/`): classes are declared in `exports` with `storage: "sqlite"`; rename and transfer move whole classes within one account; **no documented export of one object's SQLite file to local, and no move of a single object between accounts or colos**. Consequence: the DO backend is a `Store` implementation, but "migrate an object to the cloud" is our own export/import over the event log plus `VACUUM INTO`-style snapshots, not a Cloudflare feature. The 2 MB row cap and 100 KB statement cap are the constraints our schema must already respect locally so the same SQL runs on both.

### 5c. `Blobs` trait

```rust
pub trait Blobs: Send + Sync {
    fn put(&self, bytes: impl Read, kind: &str) -> Result<Digest>;      // sha256; tmp file + rename; idempotent
    fn get(&self, digest: &Digest) -> Result<Option<Box<dyn Read>>>;
    fn has(&self, digest: &Digest) -> Result<bool>;
    fn list(&self, after: Option<&Digest>, limit: usize) -> Result<Vec<(Digest, u64, String)>>;
}
```

Everything over a few KB goes here and the SQLite row stores `(digest, size, kind, zstd: bool)`: ring segments once the supervisor retires them, checkpoints, transcripts, PNGs from the lab, binaries, PR diffs, nightly `VACUUM INTO` snapshots, exported event-log pages. `LocalBlobs` = `blobs/sha256/ab/cd/<hex>` with zstd (`zstd` 0.14.3, BSD-3) for text kinds. `ObjectStoreBlobs` later over `object_store` 0.14.2 (MIT/Apache-2.0, Apache arrow-rs) which already speaks S3, R2 and MinIO through one API; R2 facts (source https://developers.cloudflare.com/r2/pricing/ and `/r2/platform/limits/`, 2026-10-21): $0.015/GB-month Standard, $0.01 Infrequent Access, egress free, 5 GiB single-part, 10,000 parts, 10 GB-month free. Content addressing makes the local store and the bucket the same namespace; sync is `has()` plus `put()`.

### 5d. "Store everything, every size, never delete" — what it costs

Rule as given: retention is operator policy; the system never deletes on its own. Numbers:

| Stream per seat | Figure | Basis |
|---|---|---|
| Harness transcript (Claude Code `.jsonl`) | **~4 MB per session file; 119 MB across the 30 files touched in the last 24 h on this machine** | measured 2026-10-21: `find ~/.claude/projects -name '*.jsonl' -newermt <yesterday> \| xargs du -ch` → 119M, 30 files. Whole-file sizes, so an upper bound on one day's delta |
| Codex session files touched in 24 h | **700 MB total** | measured: same command over `~/.codex/sessions` |
| Raw PTY bytes | **~86 MB/day/seat** | estimated: 10 KiB/s while emitting (the assumption `cloud-and-mosh-exec.md:74` already uses), emitting 30 % of an 8-hour day. TUI redraw-heavy; not measured |
| Same, zstd | **~10–20 MB/day/seat** | estimated 5–10× on escape-sequence-heavy text; not measured |
| Checkpoints | depends on cadence; a viewport-sized grid plus parser state is tens of KB each | `mux-daemon.md:134`; not measured |
| Events, audit, inbox/outbox rows | < 5 MB/day/seat at 1 message/s with 1 KB envelopes plus receipts | arithmetic |

Per seat per day: **roughly 100–150 MB raw, 30–50 MB with zstd on the text kinds.** At 1000 seats that is 100–150 GB/day raw, 30–50 GB/day compressed — 11–18 TB/year compressed. Local "never delete" at 1000 seats is therefore impossible on a laptop and is an R2 bill: 15 TB × $0.015 ≈ $225/month at the end of year one, egress free. At 10 seats (the realistic single-machine swarm) it is 300–500 MB/day, 110–180 GB/year compressed, fits a laptop. **Gap:** PTY byte rate and zstd ratio are unmeasured; measure them on a real Claude Code and Codex seat before setting ring and blob defaults.

### 5e. The per-object signed event log

`events.sqlite`, one table:

```sql
CREATE TABLE events(
  seq      INTEGER PRIMARY KEY,          -- object-local, dense
  ts       INTEGER NOT NULL,             -- unix ms, informational
  kind     TEXT NOT NULL,                -- seat.lifecycle, input.receipt, message.evidence, artifact.event, op.attempt, op.outcome, grant.epoch, …
  actor    TEXT NOT NULL,                -- seat:<ulid> | device:<key> | operator
  body     TEXT NOT NULL,                -- JSON, bounded (≤ 64 KiB; larger payloads go to blobs by digest)
  prev     BLOB NOT NULL,                -- sha256 of previous row (zeros for seq 1)
  hash     BLOB NOT NULL,                -- sha256(seq‖ts‖kind‖actor‖body‖prev)
  sig      BLOB NOT NULL                 -- Ed25519 by the object's key (seat key; device key for swarm/account)
) STRICT;
```

Append-only by construction: no `UPDATE`/`DELETE` statements exist in the code path, and `PRAGMA query_only` is set on every reader connection. Enterprises tail it in three ways, all the same bytes: `bboxy events tail <object> --since <seq>` over the seat socket (`mux-daemon.md:38`), a JSONL export per day into `blobs/` (and so into their bucket), or `ATTACH` the read-only file. Verification is `gpg --verify`-grade: the chain and the seat grant from [[messaging]]'s identity chain. This is also the provenance graph's source (`work-items.md` Provenance: item ← activity ← seat ← grant ← root), so provenance costs nothing extra.

### 5f. Multi-process access — who owns which file

| Process | Writes | Reads | Never |
|---|---|---|---|
| Supervisor | `supervisor/state.sqlite`, `seat/*/ring/` | nothing else | opens any logic-owned file |
| Logic | `account/`, `swarm/*/`, `seat/*/state.sqlite` and `events.sqlite`, `index/`, `blobs/` | `supervisor/state.sqlite` read-only for diagnostics only; normal path is the framed protocol (`mux-daemon.md:97`) | writes ring files |
| GUI | nothing | nothing on disk; snapshots and deltas over the state stream (`mux-daemon.md:31`: "not the durable seat owner") | opens any SQLite file |
| MCP bridge, scripts | nothing | over the seat socket / broker | opens any SQLite file |
| `bboxy doctor`, enterprise tailers | nothing | read-only connections (`SQLITE_OPEN_READ_ONLY`, `query_only=ON`, like Tachyon) | |

WAL rules that make this safe: one host (true: every file is under the run dir of one machine; remote seats have their own run dir on their host); one writer per file (true by the table); readers never block the writer (WAL); a long-lived reader stops checkpoints, so tailers must use short transactions and the owner runs `wal_checkpoint(TRUNCATE)` on idle. Read-only openers need the `-wal` and `-shm` to exist or the directory writable (SQLite ≥ 3.22 rule, wal.html) — the owner creates them at open, so fine. Supervisor death and logic death are independent because they never share a writer, which is the point of the two-file split. The cost accepted: no transaction spans two objects. Every cross-object fact is an event in each object plus an idempotent op in the account (`op_id` + digest), which `mcp-control-plane.md:163-165` already requires for spawn ("Process creation is not a transaction across three OS children").

### 5g. Does this remove the need for a CRDT?

Yes for everything we own. An object has exactly one writer at a time — locally the logic lease, in the cloud the single DO instance — and moving an object is a hand-off (export snapshot + event log tail, then the lease moves), not a merge. That is the same single-writer model DOs use and the same lease model `mux-daemon.md:97` already requires for the supervisor. cr-sqlite solves the problem we do not have: concurrent writers to the same rows on disconnected devices. What still merges: the markdown items in `.bboxy/`, and git already does that with the last-write-wins-plus-conflict-marker rule in `work-items.md` Gaps; and the derived index, which is rebuilt, not merged. Presence shadows are last-writer-wins by publication order per `messaging.md`, a one-column LWW that needs no library. Revisit only if a seat must be writable from two devices at once with no network between them, which no spec asks for.

---

## 6. Graph queries at our scale

Scale from the brief: 10k markdowns, 100k wikilinks, 1000 seats; five graphs, all derived from the same index (`work-items.md` "The graphs the terminal builds").

**Recursive CTE in SQLite.** With `edges(src, dst, rel, weight)` indexed on `src` and on `dst`, a bounded `WITH RECURSIVE reach(id, depth) AS (VALUES(?,0) UNION SELECT dst, depth+1 FROM edges JOIN reach ON src=id WHERE depth<4) SELECT …` is an index-nested-loop per frontier row. At 10 edges/node average and depth 4, that is ≤ 11k row visits, sub-10 ms class on a laptop. The whole 100k-edge table is ~5 MB; even `SELECT * FROM edges` into memory is one-digit milliseconds. **Not measured here** — stated from the documented algorithm (queue, one recursive-select per dequeued row, index-assisted) and the row counts. Where SQL loses: weighted shortest path, PageRank-style "related" ranking, connected components, betweenness; those are loops over the frontier in SQL and ten lines with petgraph.

**In-memory petgraph rebuilt from the index.** Load `nodes` and `edges` once (100k rows, well under 100 ms), keep a `petgraph::Graph` (or `StableGraph`) in the logic process, update it incrementally on the same watcher events that update the index. Dijkstra, Tarjan SCC, topological order for the task graph, decayed-weight recomputation for the messaging graph are library calls. Memory: 100k edges × ~32 bytes + 10k nodes ≈ 4 MB.

**Decision: SQLite is the store and the query layer for everything paged and exact** (`task_list`, `artifact_list`, "what is blocked", "children of", FTS with `bm25`); **petgraph is the compute layer for ranking and paths**, rebuilt from the index at startup and kept warm. Both read the same tables; neither is a second source of truth.

**When a graph database would earn its place:** when the graph no longer fits memory (hundreds of millions of edges), when traversals must span many machines, or when ad-hoc multi-hop pattern queries by users are the product. None applies: our graphs are per-repo and per-account, bounded by human-written markdown and by seat count, and every query shape is known in advance. A graph DB adds a server or a C++ library, a second query language, a second file format to back up, and nothing we cannot do with one table and one in-memory crate. Disagree with evidence only if a measured recursive-CTE query over the real `.bboxy/` index exceeds a frame budget; that measurement is cheap and should be the first thing the Executor's test writes.

---

## 7. Recommendation

**SQLite, per object, behind `Store`; blobs by digest; one signed event log per object; account-level derived index; petgraph in memory; no ORM, no CRDT, no graph DB.**

Against the single central database, plainly: a single `bboxy.sqlite` with a `seat_id` column on every table is the simpler v0.1 and is what ICQ does. It loses on three facts, not taste: (1) the supervisor must already own a separate durable file, so "one database" was never one; (2) `vision.md:92` forbids local-only shortcuts in the mux boundary, and the object directory is the migration unit to a VM or a DO while a row range is not; (3) a 1000-seat file needs a 2× free-space `VACUUM`, a single WAL that any tailer can pin, and a single corrupt page that takes every seat down. The costs of per-object are real and small: lazy connection LRU, fd limit raised at start, cross-object queries served by the index instead of a `JOIN`. If the operator overrules, the `Store` trait is the hedge: `SqliteFileStore` opened on one shared file with a `seat_id` prefix on keys is a one-day change and nothing above it moves.

### Schema skeleton (key columns only; all tables `STRICT`, ids are ULID text, times unix ms)

```sql
-- account/state.sqlite
kv(key PK, value BLOB)                                   -- settings cache, templates, caps, __alarm
roles(name PK, class CHECK(class IN('oracle','worker')), colour, mark, prompt, may_send_to JSON, updated)
devices(device_key PK, grant BLOB, epoch, expiry, revoked_at)
seats(seat_id PK, swarm_id, device_key, location, harness_kind, lifecycle, muted, policy_epoch, process_generation, updated)  -- registry projection of seat objects
ops(op_id PK, actor, tool, target, arg_digest, status CHECK(status IN('accepted','applied','rejected','partial','unknown')), created, updated)
op_effects(op_id, n, kind, receipt JSON, PRIMARY KEY(op_id,n))
audit(seq PK, ts, actor_account, actor_device, actor_seat, process_generation, grant_epoch, tool, op_id, target, request_digest, summary JSON, decision, evidence JSON)
alarms(object PK, at_ms)                                 -- host-side mirror for re-arming on start

-- swarm/<id>/state.sqlite
kv; layout(tab_id PK, tree JSON, revision); reservations(kind PK, count); presence(account, seat, name, harness, state, muted, policy_epoch, seq, PRIMARY KEY(account,seat))

-- seat/<id>/state.sqlite
seat(id PK, record JSON, lifecycle_revision, process_generation, committed_record_sequence, updated)   -- SeatRecord per mux-daemon.md:54
close(revision PK, deadline_ms, restoration JSON, outcome)
inbox(seq PK, author, id, digest, class, envelope BLOB, received, held_reason, UNIQUE(author,id))
outbox(id PK, destination, class, envelope BLOB, state, held_reason, created)
attempts(message_id, n, route, state CHECK(state IN('sender-stored','recipient-stored','native-queued','native-written','native-unknown','application-acknowledged')), receipt JSON, ts, PRIMARY KEY(message_id,n))
artifacts(artifact_id PK, kind, ref, digest, claimed, task, created)
artifact_events(seq PK, artifact_id, kind, digest, ts, grade, source)
blobs(digest PK, size, kind, zstd, created)              -- what this object owns in blobs/

-- supervisor/state.sqlite  (synchronous=FULL)
boot(id PK, started); lease(logic_id PK, acquired, fenced_at)
inputs(input_id PK, seat_id, process_generation, state CHECK(state IN('rejected','written','partial-unknown')), bytes_written, ts)
close_deadlines(seat_id PK, revision, deadline_ms)
rings(seat_id, generation, committed_sequence, checkpoint_digest, PRIMARY KEY(seat_id,generation))

-- index/<repo>.sqlite  (derived, deletable)
items(id PK, type, title, status, path, slug, parent, assignee, role, created, updated, frontmatter JSON, body_digest)
links(src, dst, rel, weight, PRIMARY KEY(src,dst,rel))   -- both directions indexed
items_fts USING fts5(title, body, content='items_body', tokenize='trigram')   -- external content + 3 triggers
execution(seat_id, task_id, status, delegated_by, PRIMARY KEY(seat_id,task_id))
messaging_edges(src_seat, dst_seat, class, count, last_ts, PRIMARY KEY(src_seat,dst_seat,class))
```

### Migration approach

`PRAGMA user_version` per file; one `const MIGRATIONS: &[&str]` per object class; apply above-version batches in one transaction on open; refuse to open a file whose version is newer than the binary knows (report, do not downgrade). Format changes to `events.sqlite` are forbidden — add columns only, never reinterpret, because external tailers hold copies.

### Dependencies

`rusqlite = { version = "0.40", features = ["bundled", "backup", "hooks", "functions", "serialize", "load_extension"] }`, `petgraph = "0.8"`, `ulid = "3"`, `zstd = "0.14"`, `sha2`, `ed25519-dalek` (already needed by [[messaging]]). Later: `object_store = "0.14"`, `sqlite-vec = "0.1"`, `worker = "0.8"`.

---

## Gaps

**Gap:** PTY output rate and zstd ratio per harness are unmeasured; §5d is arithmetic over the 10 KiB/s assumption already in `cloud-and-mosh-exec.md:74`. Measure on one Claude Code and one Codex seat for a day before fixing ring, blob and retention defaults.

**Gap:** recursive-CTE latency over the real `.bboxy/` index is not measured; the claim in §6 is from the documented algorithm and row counts. First test in the implementation task.

**Gap:** Durable Object per-object storage limit is 10 GB on the limits page and "currently 1 GB" on the concepts page (both fetched 2026-10-21). Which applies to a new SQLite-backed namespace today is unverified.

**Gap:** no documented way to export a single Durable Object's SQLite to a file or move one object between accounts. Local↔cloud migration must be our own snapshot-plus-event-log hand-off; the DO is a `Store` backend, not a migration mechanism.

**Gap:** `workers-rs` (`worker` 0.8.7) exposure of the synchronous `ctx.storage.kv` and `sql.exec` APIs is not verified from its docs; the `DoStore` sketch assumes parity with the JS API.

**Gap:** bundled SQLite object-code size and `librocksdb-sys` build time are stated from experience, not measured in this tree.

**Gap:** the operator rule "never delete" conflicts with `seat_purge` (`mcp-control-plane.md:157`), which deletes a seat's ring and checkpoints. Decide whether purge is the one operator-only deletion the policy allows or whether purge moves the data to blobs and only removes the seat object.

**Gap:** which key signs swarm and account event logs (device key, or a dedicated object key certified by the root) is an [[identity]] decision.
