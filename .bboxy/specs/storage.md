---
id: "5R1Z7ZZEYHQT7F2TD5GJWM6E64"
type: "spec"
number: 113
title: "Storage — decided"
created: "2026-10-21T22:45:47Z"
updated: "2026-10-24T00:10:38Z"
---
# Storage — decided

Owner: Vesper Greywind <ORACLE>. 2026-10-21. Decided from [[state-backend]] (research). Operator: "sqlite probably, unless you have better ideas; store everything, every size; enterprise-grade compatible; durable objects, maybe on-premise." Back to [[vision]] Storage shape · [[mux-daemon]] · [[messaging]] · [[work-items]] · [[provenance]].

## The call

**SQLite, one small database per durable object.** `rusqlite` 0.40 with the bundled SQLite 3.53 (FTS5 and JSON1 compiled in), WAL mode. Objects: account, swarm, seat, supervisor, and the per-repo `.bboxy` index. Each object is a directory: `object.sqlite`, `events.sqlite`, `blobs/`, and for seats the byte ring. The directory is the unit of backup, migration and deletion.

Why not one central database: the supervisor already needs its own file for crash isolation; the object directory is what moves to cloud or another machine; a 1000-seat single file needs double space to vacuum, pins the WAL, and one corrupt page kills everything.

## Traits

- `Store`: get, put, list, sql, alarm. Implementations: SQLite file (now), Cloudflare Durable Objects storage API (sketch in the research; later).
- `Blobs`: put by digest, get, list. Implementations: local content-addressed directory (now), `object_store` 0.14 over R2, S3, MinIO (later). Everything large goes here: rings beyond the hot window, lab PNGs, transcripts, binaries, PR diffs, zstd-compressed.
- `Events`: one append-only `events.sqlite` per object, Ed25519-signed rows, hash-chained; the [[provenance]] activity log is written here first and projected to `.bboxy/log/*.jsonl`. This is what an enterprise tails.

**In memory, with semantic lookup (operator, 2026-10-21).** The index object is the durable backing; the working set is in memory: the six graphs in `petgraph`, every markdown's frontmatter and body, seat cards, recent messages. Alongside it an embedding index: every markdown, task, artifact, message and seat card is embedded once on change by a local encoder (icq's pocket-encoder, the MiniLM-L3 q4 model the roadmap already names, 384-d, no network), vectors persisted with `sqlite-vec` in the index object and loaded into memory at start. Lookup is hybrid: lexical tiers first (the launcher's), semantic as the tie-breaker and as "related", reciprocal-rank fused. Full-text search (FTS5 over frontmatter and body, chunked by heading) sits next to the vectors; a question hits both and the results are fused. The `how` and `find` MCP tools and the launcher's Go to read this index. Any seat can ask in words: "the task about the byte ring", "who worked on transport", "notes that cite mosh". Rebuild from the SQLite objects is always possible; memory is a cache of the durable store, never the only copy.

No ORM. Migrations: hand-rolled version table per object as icq does. Graph queries: `petgraph` in memory rebuilt from the index tables for ranking and paths; recursive CTEs only for simple ancestry. No CRDT: objects migrate, they do not merge. No graph database.

## Store everything, never delete

Retention is operator policy per object kind, default forever. Measured on this machine: Claude transcripts touched in 24 h, 119 MB across 30 files; Codex session files, 700 MB. Estimated PTY output per active seat: ~86 MB per day raw, 10–20 MB zstd. At 1000 seats that is tens of GB per day compressed; the `Blobs` tiering to object storage is what makes "forever" affordable, not a smaller number.

**`seat_purge` reconciled with never-delete:** purge terminates the process and releases live resources; the ring, checkpoints and events are moved to cold blobs, never deleted by the terminal. Actual deletion is an out-of-band operator action on the filesystem or bucket. [[mcp-control-plane]] wording updated.

## Multi-process rules

Supervisor owns `supervisor/object.sqlite` and every seat's ring; logic owns the other object files; the GUI opens nothing, it reads through the state stream. Two writers on one file never happen by construction.

## Cloudflare notes recorded

Durable Objects storage is SQLite-backed with alarms and single-threaded objects; per-object limits are stated as 10 GB on one page and 1 GB on another (**Gap**); no documented single-object export or move, so cloud→local migration of an object goes through our own `Blobs` export, not Cloudflare's.

## Gaps

- **Gap:** the per-object limit conflict above.
- **Gap:** recursive-CTE latency at 100k edges estimated, not timed; measure in W2.2.
- **Gap:** `maxfiles` soft limit 256 on macOS versus one or more open SQLite files per object at 1000 seats; raise the limit at startup or pool connections. Measure in W2.2.

Links: [[state-backend]] [[vision]] [[mux-daemon]] [[messaging]] [[work-items]] [[provenance]] [[mcp-control-plane]]

## Decision 2026-10-22 — graph reads in memory

Measured in W2.2 (`.bboxy/runs/w2-2-measurements.log`, 10k docs / 100k edges): a SQLite recursive CTE reachability takes 42.7–43.2 ms; petgraph in memory takes 3.4–3.6 ms. SQLite stays the durable source of truth. `orient`, `graph_read` and the other graph queries answer from an in-memory petgraph projection, rebuilt from SQLite on start and kept current by the bus.
