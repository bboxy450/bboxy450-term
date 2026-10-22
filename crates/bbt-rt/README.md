# bbt-rt

R1's shared two-worker Tokio IO runtime. Call `init_ui(waker)` once on the app
thread, with a `Waker` backed by its event-loop proxy. `handle()` returns the
same runtime handle for all subsystems. `is_ui_thread()` and the debug guard
`assert_not_ui_thread()` identify prohibited blocking entry points.

`UiInbox` gives the app a cloneable sender, a bounded `try_recv` queue, the
command output view and source dirty bits. Drain dirty bits at frame start,
then drain messages under the app's 1 ms budget. Sources 0 and 1 are reserved
for messages and command output. The default queue and view hold 256 items
each; command chunks are at most 16 KiB, at most 4 MiB of payload per queue.
They share chunk bytes through `Arc`, and never block on a consumer. A full
message queue returns the rejected message. `Latest<T>` replaces pending
state; `ViewRing<T>` drops its oldest item and counts drops. These are view
primitives, with no durable-retention claim.

`DirtySet` tracks each source independently and admits one event-loop wake
until drained. Resetting wake admission before atomically clearing the bits
ensures a racing producer is drained now or has a wake for the next frame.

`spawn_command(argv, cwd, env)` returns a `TaskId` immediately. Spawning,
stdout/stderr reads and waits happen on Tokio. Arguments and environment
overrides go directly to the child. Reads publish newline-delimited pieces
(or partial lines for prompt/progress output), at most 16 KiB each. The view
and `UiMsg::Output` identify the task and stdout/stderr separately.
`UiMsg::Exited` reports code, signal, cancellation, IO error and byte totals.
`TaskId::status()` retains completion even when the notification queue is
full; apps must check their active task handles after the dirty wake.
`cancel(&task)` only sets memory and notifies the runtime. The IO task sends
SIGKILL to the child's new Unix process group, waits for the child and drains
both pipes before publishing completion. Descendants that deliberately leave
the group are outside this cancellation guarantee. `CommandRunner` supports
the same operations with independently configured queue/view capacities.

Run `cargo test -p bbt-rt --locked` and
`cargo clippy -p bbt-rt --locked --all-targets -- -D warnings`.
The five brief checks plus queue replacement, argument/environment/error
handling and early cancellation run in `tests/runtime.rs`.

The root `clippy.toml` uses concrete paths because Clippy does not accept
`std::fs::*` globs. The disallowed-methods lint is enabled by default. Scan the
three UI libraries with:

```sh
cargo clippy -p bbt-ui-model -p bbt-ui -p bbt-render --locked --lib -- -W clippy::disallowed_methods
```

Use `-D clippy::disallowed_methods` for a hard gate. Existing violations,
including build-script IO (which executes at build time), are recorded in
`.bboxy/runs/r1-ui-io-lint.log` rather than changed by R1. IO crates/tests need
scoped, justified lint exceptions where blocking work is intentional. This
configuration is a guard against named APIs, not an exhaustive IO detector.

Not implemented here: parser pools, the GUI/frame drain, hitch meter, display
link, byte-ring persistence, durable command output, process quotas, or the R5
stall test. R1 was exercised on macOS/Apple silicon; Linux and power-loss
behavior were not tested.
