# bbt-supervisor

The supervisor side of the existing process boundary, moved out of `bbt-mux`
without changing its wire format or behavior. One mio actor owns PTY masters,
children and writer leases; one disk writer owns ring IO. `protocol` and
`transport` retain the bounded versioned frames. `Client` is the in-process
client; `SocketSupervisor` and `SupervisorConnection` moved from mux logic for
the same private Unix socket protocol.

`ring` retains CRC32C capture, committed boundaries, cold segments, checkpoints,
logical-line indexing and effect-free replay. `store` holds its shared blob,
error, timestamp and digest primitives; SQLite, signed event storage, the bus,
index, adapters and messaging stay in `bbt-mux`. Existing mux paths re-export
the moved types. Ring integration tests remain in mux because they compare
replay with the logic viewport projection; PTY/lease tests moved here unchanged
apart from the crate import.

There is no Tokio dependency or runtime. `bbt-rt` is used with
`default-features = false`, retaining the same UI-thread registry and debug
guards. The full app enables its default `runtime` feature through mux.

Verify the dependency boundary with `cargo tree -p bbt-supervisor -e normal`.
Run `cargo test -p bbt-supervisor -p bbt-mux --locked` for the existing supervisor,
capture and logic coverage. Retained limits and mechanisms are described in
`../bbt-mux/README.md`.
