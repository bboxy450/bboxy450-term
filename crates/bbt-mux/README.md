# bbt-mux — supervisor foundation (W2.1)

One `Supervisor` owns one named thread and one `mio::Poll`. Every seat's
`portable-pty 0.9.0` master, child handle, resize, read and write lives in that
actor. The poller registers each master's `as_raw_fd()` and uses nonblocking
reads/writes; no per-seat reader, writer or wait thread is created. Child exit is
checked with `try_wait` on the same thread, at a 20 ms maximum idle interval.

This implements the first layer of [[mux-daemon]] and [[terminal-core]]. Ring
storage/checkpoints (W1.2), harness adapters/readiness (W2.3), and the eventual
supervisor/logic process split are later layers. A supervisor lease is **not**
proof that a harness composer is ready for input.

## Module map

- `src/protocol.rs`: ULID seat/request/input/lease identities, u64 process
  generations, launch overrides, versioned requests, replies and ordered events.
- `src/supervisor.rs`: native PTYs, the single poller, logic/writer lease fencing,
  input receipts/deduplication, child exit/termination and the in-process client.
- `src/transport.rs`: length-prefixed framing, partial-frame decoding, private
  Unix listener setup, socket output bounds and the default runtime path.
- `tests/supervisor.rs`: live `/bin/zsh -f` fixtures, including 50 simultaneous
  seats and a PTY that refuses input long enough to exercise a partial write.

## In-process use

```rust,no_run
use bbt_mux::{Config, LaunchSpec, Supervisor, Ulid};

let supervisor = Supervisor::start(Config::default())?;
let logic = supervisor.connect()?;
let mut launch = LaunchSpec::new("/bin/zsh", std::env::current_dir()?);
launch.args.push("-f".into());
let seat = logic.spawn(launch)?;
let writer = logic.attach(seat.seat, seat.generation)?;
// The caller/adapters must establish input readiness before this low-level call.
let receipt = logic.input(&writer, Ulid::new(), b"echo hi\n")?;
// receipt proves PTY writing only, never shell/model consumption.
# Ok::<(), Box<dyn std::error::Error>>(())
```

`LaunchSpec` sets program, arguments, absolute cwd and explicit environment
overrides. `BBOXY_SEAT_ID` and `BBOXY_PROCESS_GENERATION` are assigned last by the
supervisor so a supplied override cannot change the attribution. Only explicit
launch overrides are retained, not a snapshot of the inherited environment.

## Leases, ordering and input evidence

`AcquireLogic` requires the supervisor owner's random 32-byte admission
capability. It replaces the previous logic lease and invalidates all previous
writer attachments. `Attach{seat,generation}` issues a new writer lease for
that process. Both leases are checked against the owning connection, not merely
against caller-supplied token strings. `Input`, `Resize` and `Kill` require the
attachment. Stale logic, writer and process generations return distinct errors.

First launch uses generation 1. Exact repeated launch specs return the original
child instead of spawning another; an exited/drained seat can be explicitly
replaced with generation + 1. A reconnect lists/attaches existing children.
Dropping a logic client does not restart or kill its child. Dropping/shutting
down the supervisor kills and reaps its owned live children; there is no promise
of process survival across supervisor death in this in-process milestone.

Output, successful resize and exit events share one monotonically increasing
per-seat `seq`. Captured output precedes the published exit; no timestamp merge
is used. `Attach` supplies `first_available_sequence` and `next_sequence` so a
caller cannot interpret an unavailable earlier prefix as replayed history.
There is no retained-history API yet. Events already delivered into an old
client's channel/socket cannot be replayed to its replacement without W1.2.

Input IDs are retained in memory for the whole process generation, including
across logic replacement. A retry with the same bytes returns the same stored
receipt; different bytes under the same ID are rejected. A SHA-256 digest avoids
retaining completed input plaintext. Only one input is in flight per seat;
replacing a lease cannot start a second write around it. Pending writes finish
or time out without being restarted. Query `InputStatus` with the original ID.

Receipts are `Pending`, `PtyWritten`, or `PartialUnknown`, with the exact written
prefix length. `Input` returns its current receipt; `InputStatus` observes a
pending write's later result. A timeout/error never automatically pastes its
unwritten suffix or presses Enter. None of these receipts claims application
acknowledgment. A supervisor crash loses the in-memory IDs/receipts; durable
input bookkeeping is not implemented here.

`Kill` sends SIGHUP without blocking the poller, escalates to SIGKILL after
250 ms, and returns `Terminating`; the actual outcome is the later `Exited`
event. It addresses the owned child, not a caller-supplied PID. Process-tree,
close-grace and harness-resume policies are not implemented by this primitive.

## Unix framing and bounds

Default v0.1 transport is bounded in-process channels. To enable Unix IPC, pass
`Config { socket_path: Some(transport::default_socket_path()?), .. }`.
On macOS the default is
`~/Library/Application Support/bboxy450-term/run/mux.sock`; Linux uses a private
`bboxy450-term` directory below `$XDG_RUNTIME_DIR`, with a home-state fallback.
The parent must be owned by the current uid, non-symlink and private (0700); the
socket is 0600. An existing endpoint is never unlinked to take ownership. Drop
removes only the same socket inode this supervisor bound.

The same poller accepts and serves socket clients; there is no extra socket or
per-client thread. Frames are a **u32 big-endian byte length**, then a UTF-8 JSON
`Frame { version: 1, body }`. Client body is `Request`; server body is `Message`
(correlated reply or event). Length is checked **before payload allocation**.
Malformed/unsupported frames close the connection. Typed in-process requests
use the identical dispatcher and version checks. Unix IPC is local admission,
not a sandbox against another process already holding the same uid/capability.

Bounds: 1 MiB per encoded frame; 64 KiB per input or launch configuration;
16 KiB output chunks; 128 queued in-process commands/events; 4 MiB encoded output
per socket; eight logic connections; at most 1024 seats (configurable downward).
The per-generation input cache admits 4096 IDs, then rejects new ones rather
than forgetting an old ID and risking duplicate input. A stalled/detached reader
holds at most one pending output chunk per seat and applies kernel PTY
backpressure; it never creates an unbounded capture queue. These are bounded
starting limits, not a 1000-seat memory/performance measurement.

## Verification and limits

Run `cargo test -p bbt-mux --locked`. Exact command results are in
`.kb/runs/w2-1-*.log`. Codec tests reject invalid/oversized lengths, unknown
versions and truncated bodies, and decode fragmented/coalesced frames. Native
tests verify echo, cwd/env attribution, `stty size`, generation-tagged exit,
two-seat separation, 50-seat echo, lease replacement without child duplication,
input-ID retry/conflict, live Unix admission/framing and blocked-write ambiguity.

Not covered: rings/checkpoints or disk durability; logic/supervisor process
splitting; GUI wiring; native harnesses/input guards; close grace/undo/resume;
remote seats/network identity; crash/power-loss recovery; macOS 14 or Linux
runtime validation; inherited background-job lifecycle; 1000 seats or throughput
benchmarks. No user harness settings, trust hashes or credentials are changed.

API references inspected: [portable-pty MasterPty](https://docs.rs/portable-pty/0.9.0/portable_pty/trait.MasterPty.html),
[mio Poll](https://docs.rs/mio/latest/mio/struct.Poll.html). Locked versions and
licenses are recorded by Cargo; no upstream PTY or poller source was vendored.
