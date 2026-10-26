# bboxy450-harness-wake-up

## What it is

Launch, observe and wake terminal harnesses through their native protocols. The caller supplies process hosting, authorization, persistence and application policy.

## Mental model

`HarnessRecord` describes a harness. `LaunchContext` produces a `LaunchPlan`; a process `Binding` anchors observations in a `Session`. A native attachment and a guarded readiness token describe the two supported delivery paths.

## Invariants

Transport evidence never means that a model read a message. `NativeUnknown` must not be automatically retried or followed by PTY fallback. Native bindings include process and generation identity; PTY fallback requires positive composer-readiness evidence.

## Entry points

1. Select `HarnessRecord::builtin` or validate your own record.
2. Call `launch::plan` on an IO owner and host that plan yourself.
3. Construct `Session` with the actual process binding; feed its observations.
4. Check `native_available`, call `Adapter::wake`, and retain the returned evidence.
5. If policy permits fallback, obtain and consume a readiness token before writing bytes.

## Minimal example

```rust
use bboxy450_harness_wake_up::{HarnessRecord, WakeMethod};
let shell = HarnessRecord::builtin("zsh")?;
shell.validate()?;
assert_eq!(shell.wake_method, WakeMethod::Pty);
# Ok::<(), bboxy450_harness_wake_up::Error>(())
```

## Recipes

The complete `record.rs` program is in `examples/`. Build it with
`cargo build --example record`.

| Harness | Observe | Native wake | PTY fallback |
|---|---|---|---|
| Claude | Hooks, OSC, session metadata | Bound cross-session socket | Positive composer readiness |
| Codex | Hooks, OSC, app-server events | Explicit per-process app-server | Positive composer readiness |
| zsh | OSC 133 and working-directory reports | None | Canonical prompt readiness |
| Custom kind | Caller-supplied observations | No built-in transport | Operator handoff unless implemented |

Implement `hook::HookSink` to carry fallback hook events over your own admitted transport. Supply skill text through the launch context rather than changing global harness configuration.

## Pitfalls

This crate does synchronous filesystem and socket work; keep it off the UI thread. An installed harness or a matching PID alone does not establish a live native attachment. The kit does not assign roles or send through an application messaging ledger.

## Platform support

Unix terminals and sockets; macOS is the integrated and tested host. Non-Unix targets are rejected at compile time. pi and omp have no built-in adapter.

## Feature flags

Default: no features. `runtime-assertions` enables the optional runtime's off-UI-thread assertion.

## License

MIT OR Apache-2.0, at your option. Both licence texts ship in this package.
