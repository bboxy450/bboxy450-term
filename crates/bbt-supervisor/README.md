# bbt-supervisor

The supervisor side of the existing process boundary, moved out of `bbt-mux`
without changing its wire format or behavior. One mio actor owns PTY masters,
children and writer leases; one disk writer owns ring IO. `protocol` and
`transport` retain the bounded versioned frames. `Client` is the in-process
client; `SocketSupervisor` and `SupervisorConnection` moved from mux logic for
the same private Unix socket protocol.

Child-status probes have a 100 ms cadence rather than following every ready-PTY
iteration; EOF and due kill escalation request an immediate probe.

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

## Shared capture journal (W1.2d, decisions 21–22)

Capture requires `Config.journal_directory = Some(absolute_machine_object_dir)`.
`None` permits uncaptured PTYs, but `AttachRing` returns
`JournalDirectoryRequired`; there is no temporary/default/discovered journal.
The application owns this directory outside every seat's purge/backup unit.

The single existing disk writer appends all seat generations to a
`journal-<ULID>.log`. Records use the same bounded BBR1 header and CRC32C as
rings. A nil-seat, kind `0xffff` tick-commit frame contains the preceding file
length. One ordering barrier publishes the whole tick (20 ms), without a
per-seat marker or directory sync. Full flushes occur on checkpoints, seals,
shutdown and the 5 s / 16 MiB machine-wide cadence. The byte limit is enforced
at admission, rather than discovered after accepting excess output. Queue,
tick-window or durable-credit pressure pauses admission and wakes the writer;
logic retries only rejected capture `Busy` replies, never ambiguous effects.
The `paused` display is delayed until pressure persists for one second.
Live catch-up consumes one history page per worker turn, retaining its cursor
and final event while control commands run between pages; replay has no effects.

Seat rings are derived. Live history pages merge their retained prefix with
bounded journal-tail references; checkpoint payloads remain opaque and are
acknowledged only after a journal full flush. Checkpoint journal records prefix
the payload with a covered journal ULID and offset (versioned 28-byte prefix), referring to the
parsed sequence, or a conservative earlier retained replay floor. Legacy
8-byte publication cursors remain readable. Idle work and seals compact tail
records into the existing cold-segment format; projection boundaries store the
covered journal filename/offset. Reopening scans CRC-valid whole ticks and
rebuilds a lagging projection. It rejects torn/uncommitted suffixes and never
replays terminal effects. Segments rotate at 64 MiB.

Loss has two separately measured spans: admitted minus committed (one admission
tick per seat), and producer-written minus admitted (the platform PTY buffer
plus the two-millisecond output-coalescing window). The SIGKILL fixture uses a
test-only mmap admission ledger to observe the first boundary independently of
the successful-write counts; it does not assert producer bytes against the
ring's tick guarantee. Power-loss behavior is a protocol bound, not a tested
power-cut claim.

Global writer busy time, commit-round p99 upper bound/max, undurable byte peak
and durable pauses repeat in each seat status; do not sum those fields across
seats. Per-seat queue counters and projected file timings retain their scope.

Journal writes use one bounded 1 MiB staging buffer, flushed before tick barriers
or a history read that needs pending bytes. History keeps validated logical-row
indexes separately and never reconstructs the terminal model merely to size or
annotate a page. Orderly shutdown full-flushes the journal and leaves projections
to recovery; it does not force all seat data through cold compaction. Old-process
journals are conservatively retained if their projection proof is unavailable.

The ignored real-process crash gate defaults to two seats. To exercise the
measured maximum on this Mac, run
`BBT_CRASH_SEATS=967 cargo test -p bbt-supervisor --release --lib capture_sigkill -- --ignored --nocapture --test-threads=1`.
It compiles a small native synthetic producer, stages each seat behind a start
file, kills the owning supervisor, and reopens every ring through production
recovery. Successful producer writes and the mmap admission ledger are separate
observations; per-seat prefix, CRC and both loss bounds are checked. This is a
supervisor-kill test, not a power-cut test. The fixture stops before exhausting
the 10 GiB free-disk reserve.

The 60-second firehose gate in `bbt-mux/tests/gates_seats.rs` retains the strict
writer-busy `<50%` and zero-`Busy` assertions at up to 400 seats. At the measured
maximum, those counters are reported along with liveness and commit latency,
per the Oracle's gate interpretation; there is no maximum-seat busy threshold.
