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
| Codex | Hooks, OSC, app-server events | Pinned shared daemon and exact authenticated thread | Positive composer readiness |
| zsh | OSC 133 and working-directory reports | None | Canonical prompt readiness |
| Custom kind | Caller-supplied observations | No built-in transport | Operator handoff unless implemented |

Implement `hook::HookSink` to carry fallback hook events over your own admitted transport. Supply skill text through the launch context rather than changing global harness configuration.

Claude receives NAME, ROLE, host/cwd and "call orient first" through its appended system prompt. Codex receives those instructions and its isolated MCP/hook wiring through a per-thread app-server start/resume; the TUI attaches to that named thread. The host dispatches the plan's `harness-exec` entry to `launch::exec_when_ready`. Shared-daemon startup is idempotent through Codex's CLI; the host never restarts, stops or cleans up that borrowed daemon. Native wake-up validates the exact thread and daemon peer, and never creates or loads an unknown target.

## Pitfalls

This crate does synchronous filesystem and socket work; keep it off the UI thread. An installed harness or a matching PID alone does not establish a live native attachment. The kit does not assign roles or send through an application messaging ledger.

## Platform support

Unix terminals and sockets; macOS is the integrated and tested host. Non-Unix targets are rejected at compile time. pi and omp have no built-in adapter.

## Feature flags

Default: no features. `runtime-assertions` enables the optional runtime's off-UI-thread assertion.

## License

Project-owned contributions use the [Respectful Slop License 1.0-Draft](LICENSE). Upstream terms and prior grants remain intact; see [ATTRIBUTION.md](ATTRIBUTION.md) and [NOTICE](NOTICE).

## Respectful slopping

Project-owned contributions use [Respectful Slop License 1.0-Draft](LICENSE).
Retained upstream material keeps its original terms; earlier grants are not
withdrawn. [ATTRIBUTION.md](ATTRIBUTION.md) identifies the scope, source and reuse
examples; [NOTICE](NOTICE) preserves factual upstream notices.
Slop respectfully. Keep the credit attached.

## Hook input bounds

A hook has one shared 50 ms deadline. Raw CLI/stdin JSON is limited to 2 MiB,
then projected to the original status/provenance fields: task descriptions,
questions, artifact paths and agent identity, without tool file contents or
command output. The retained event stays limited to 64 KiB and the terminal OSC
to 4096 bytes; larger retained events are still refused. This allows ordinary
verbose tool hooks without enlarging the bus or terminal-frame limits.
