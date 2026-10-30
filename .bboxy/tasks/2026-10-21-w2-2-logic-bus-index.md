---
id: "13M0EBQRXQX9B5S94118AMJKHA"
type: "task"
number: 116
title: "W2.2 — bbt-mux: logic process, event bus, .bboxy index, state streams"
created: "2026-10-21T23:15:56Z"
updated: "2026-10-24T00:10:38Z"
---
# W2.2 — bbt-mux: logic process, event bus, .bboxy index, state streams

Owner: Vesper Greywind <ORACLE>. Assignee: free Executor. Status: SUPERSEDED (8fd15659). Previous brief wording: OPEN (W2.1 done; no other dependency). Specs: [[mux-daemon]] (seat registry, logic ↔ GUI state streams), [[events]] (the bus), [[storage]] (Store/Blobs/Events traits, SQLite per object), [[work-items]] (frontmatter, wikilinks with `{rel:w}` annotation, six graphs), [[provenance]] (activity log).

Allowed: `crates/bbt-mux/**`, `Cargo.lock`, `.bboxy/runs/w2-2-*.log`.

1. **Store traits and SQLite objects.** `store/{mod,sqlite,blobs,events}.rs`: `Store` (get/put/list/sql/alarm), `Blobs` (put by digest/get/list, local dir, zstd), `Events` (append-only signed rows, per-topic seq, hash chain); one directory per object under the run dir; version table migrations. Tests: round trips, chain verification, torn-write rejection.
2. **Seat registry** on the account/swarm objects: ULID seat records (name, harness kind, role {name,class}, where, mute, generation, task refs), durable seat id ↔ harness session mapping, close grace state machine with the 60 s default and undo. Attaches to W2.1's supervisor client.
3. **Event bus.** The [[events]] envelope, topics, per-topic sequence from `Events`, subscriptions with patterns, `since` replay and `gap`, bounded coalescing queues; producers for `seat/*` (from supervisor output and state), `mux`, `swarm/*`; delivery to in-process subscribers and to the seat unix socket (NDJSON envelope per [[kittens-events]] design in [[vision]] Seats are sockets).
4. **.bboxy index.** Watch `.bboxy/**/*.md` (FSEvents via `notify` crate), parse frontmatter and wikilinks (both `[[slug]]`, `[[slug|display]]`, `[[slug]]{rel:w}` and the `links:` list), build the markdown/task/execution/artifact graphs in `petgraph`, persist to the index object, emit `doc/<id>` and `doc/path/<glob>` events on change; `updated_by` stamping and the provenance digests from [[provenance]] §1 on writes the mux performs.
5. **State streams.** `Attach/Subscribe/Snapshot/Delta/Applied/GetHistory/Gap` over the local unix IPC with the generations and baseline rules from [[mux-daemon]]; a headless test client attaches, receives a snapshot of a live zsh seat, kills logic, re-attaches, screen identical.
6. Measure and log: recursive-CTE vs petgraph on 10k synthetic docs / 100k links; open-file count at 200 seats against `maxfiles`.

Verify: `cargo test -p bbt-mux --locked` green; `cargo clippy -p bbt-mux --all-targets -- -D warnings`. Report with the two measurements.
