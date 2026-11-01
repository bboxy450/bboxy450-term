# bboxy450-term-plugin-sdk

Build bboxy450-term plugins in Rust. A plugin is one executable that prints its
shape with `--shape` (or with no arguments and a closed stdin) and otherwise
serves newline-delimited JSON-RPC MCP on stdio. The SDK owns that runtime; a
plugin implements one contract trait (or [`Plugin`] itself) and calls [`run`].

## Build a plugin

### 1. Pick the contracts

A plugin fulfils one or more of the app's systems. Each contract has typed
requests and results in `bboxy450-plugin` (`bbt_plugin::<contract>`) and a
JSON Schema in `schemas/` that both the host and the SDK use.

| Contract | What it is for | Methods | Trait / adapter | Example |
|---|---|---|---|---|
| `data_source` 1.0 | documents and links under one root | resolve, fetch, search, list, links, graph, write, changes_since (+ commit) | `DataSource` / `AsDataSource` | `bboxy450-term-data_source-example` |
| `task_system` 1.0 | tasks and their host-signed lifecycle | list, status, changes_since, delegate, reassign, report, accept, reject | `TaskSystem` / `AsTaskSystem` | `bboxy450-term-task_system-example` |
| `event_sink` 1.0 | host-only fan-out of signed envelopes | deliver | `EventSink` / `AsEventSink` | `bboxy450-term-event_sink-example` |
| `decision_model` 1.0 | choose one named criterion | choose | `DecisionModel` / `AsDecisionModel` | `bboxy450-term-decision_model-example` |
| `repo_graph` 1.0 | repositories under a root and their relations | describe, changes_since | `RepoGraph` / `AsRepoGraph` | `bboxy450-term-repo_graph-example` |
| `session` 0.1 | external harness sessions | describe | `Session` / `AsSession` | `bboxy450-term-session-example` |
| `provider` 0.1 | hosts seats may run on | describe | `Provider` / `AsProvider` | `bboxy450-term-provider-example` |
| `transport` 1.0 | host-only command wrapper with declared launcher fields | wrap | `Transport` / `AsTransport` | `bboxy450-term-transport-example` |

The executable is named `bboxy450-term-<contract>-<provider>`
(`[a-z0-9-]` provider). Authority never crosses a contract: `root`, `author`,
`signer`, `device` and `account` in tool arguments are refused (except one a
contract request itself declares, the host-chosen `root` of `repo_graph`
`describe`, which only the host sends), the host signs
writes (`signed_record` is opaque and returned byte for byte) and the host,
not the plugin, decides who may call what.

### 2. Implement it

One contract: implement `contracts::Base` (descriptor and root binding) and
the contract trait, then run the adapter. Arguments arrive typed; results are
encoded and size-checked for you; `invalid_arguments` and `unknown_method`
refusals are automatic.

```rust,ignore
use bbt_plugin::decision_model as dm;
use bbt_plugin_sdk::contracts::{AsDecisionModel, Base, DecisionModel};
use bbt_plugin_sdk::{Ctx, Init, Manifest, Refusal, run};

struct First;

impl Base for First {
    fn manifest() -> Manifest {
        Manifest::new("bboxy450-term-decision_model-first", "First", "Always the first criterion")
    }
    fn initialize(_init: Init, _ctx: &Ctx) -> Result<Self, Refusal> {
        Ok(First)
    }
}

impl DecisionModel for First {
    fn choose(&mut self, req: dm::ChooseRequest, _ctx: &Ctx) -> Result<dm::ChooseResult, Refusal> {
        let choice = req.criteria.keys().next().cloned().unwrap_or_default();
        let probabilities = req.criteria.keys().map(|k| (k.clone(), if *k == choice { 1. } else { 0. })).collect();
        Ok(dm::ChooseResult { choice, confidence: 1., probabilities, model: "first-1".into() })
    }
}

fn main() {
    run::<AsDecisionModel<First>>();
}
```

Several contracts in one binary: implement [`Plugin`] and route each call
with `contracts::serve_task_system(self, method, args, ctx)`,
`serve_event_sink`, … (`Plugin::call` receives the contract). The examples
under `examples/` are complete plugins, one per contract:
`cargo build -p bboxy450-term-plugin-sdk --examples`.

The runtime calls a plugin on one worker thread, one request at a time.
`notifications/cancelled` flips the request's [`Ctx`]; each call gets a
deadline just inside its declared `timeout_ms` (`Manifest::timeout`, default
2 s). Long work calls `ctx.check()?` between steps. Frame (8 MiB), result
(256 KiB), pending-request (8) and duplicate-id limits are enforced; a panic
becomes an `internal_error` result.

### 3. Seat tools

Seats see every contract method they may call as
`plugin_<contract>_<provider>_<method>` (worker or oracle by contract; host-only
`event_sink` and methods needing host fields are never seat tools). For
anything beyond the contracts, declare `Manifest::tool(name, description,
input_schema, output_schema, RoleClass::Worker | RoleClass::Oracle)` and
serve it in `call_tool`; seats see `plugin_tool_<provider>_<name>` (at most 64
bytes). The host validates arguments and results against the declared
schemas and admits every declared tool once per op_id.

### 4. Hooks

Declare `Manifest::hook(event, Phase::Before | Phase::After, budget_ms)` for
a harness event (`PreToolUse`, `PostToolUse`, `UserPromptSubmit`,
`PermissionRequest`, `SessionStart`, `SessionEnd`, `Interrupt`, `SeatState`)
or `Manifest::bus_hook(subject_pattern, budget_ms)` for bus events (`*` one
token, `>` the tail; never `inbox/…` or `plugin/…`). Events arrive in
`on_hook` as [`HookEvent`]; reply `HookReply::Allow` or `HookReply::Deny {
reason }`. Budgets are capped (1 s for ending events, 2 s before a tool,
else 5 s). `payload` is present only when the operator granted it, and a deny
vetoes only a `before` hook holding the operator's block grant.

### 5. Host API: records, events, messages

Request methods with `Manifest::host_api(..)`; the operator grants them at
trust. While serving a call, `ctx.host()`:

- `records_search(json!({"query": …}))`, `records_fetch(json!({"ref": …}))`:
  your own knowledge root (data_source `search`/`fetch` shapes);
- `events_publish("build/done", json!({…}))`: bus event
  `plugin/<candidate>/build/done` (an object of at most 64 KiB);
- `messages_send("seats/<seat>/inbox", "text")`: plain text to a destination
  the operator listed in `messages_to`, signed by the host as
  `plugin:<candidate>`, never as a seat or the operator.

Each request waits at most until the calling request's deadline. Ungranted:
`host_api_not_granted`; outside a served call: `host_unavailable`.

### 6. Setup and login

`Manifest::auth("auth-status", "auth-login")` declares setup. The host runs
`<binary> auth-status` (route: `auth_status`, exit 0 logged in, 1 not) and,
when not logged in, `<binary> auth-login` (route: `auth_login`) in a visible
"plugin login" row in the app: a bare terminal the operator types into, with
stdin, stdout and stderr on its PTY, `TERM=dumb`, a scrubbed environment and
the plugin's data directory as cwd. It has no MCP, hooks or OSC side
channels; device flows print their URL or code there. When it exits (or after
120 s) its whole process group is ended. Store credentials under the data
directory; the host never reads them.

### 7. Trace spans

The host measures every call and hook itself (Settings shows timelines and
phases). Declare `Manifest::trace()` to add your own steps: `let _scan =
ctx.span("scan");` times the step until the guard drops. Spans travel beside
the result (`_meta.bboxy450.spans`) only when the host enabled trace at
`initialize`; at most 32 per call, kept only inside the host's `plugin` phase.

### 8. Skills

`Manifest::skill(Skill { name, description, content })` embeds a `SKILL.md`
(at most 16, 64 KiB each). Seats of a root where the plugin is enabled and
checked read them through `skill_list`/`skill_read`; their origin stays the
plugin and its pin, and they grant no authority.

### 9. Test it

- Unit-test the trait methods directly with `Ctx::detached(budget)`.
- `testkit::Client::spawn(binary, data_dir, &testkit::allowed_env())` drives
  the built executable over stdio (`initialize`, `call`, `close`), exactly as
  the host does; `testkit::shape(binary)` reads the descriptor.
- `Manifest::validate()` runs the host's own descriptor validator.
- The host conformance checker (`bbt_mux::plugins::check(binary)`, also
  `bboxy450-term plugin check <binary>`) runs every contract method with
  additive-field probes in a disposable fixture account. Each example here
  passes it in `bboxy450-term-mux`'s `tests/plugin_sdk_examples.rs`.
- Host-side behaviour (trust, hooks, host API, setup) is tested with the
  compile-time variants of `bboxy450-term-plugin-fixture`
  (`Knobs { .. }.build()` / `.install(path)`): each variant is its own pinned
  binary, built small (`--release`, opt-level s, stripped) because the host
  hashes the pinned copy per call.

### 10. Models

The plugin runtime's own contract (request admission, cancellation and host
callbacks during a call) and the host's activation, connection, hook, host
API and setup protocols are TLA+ models checked by `make test-formal` with
both the native `bboxy450-tlc` checker and Java TLC; see
`verification/README.md`. Change a lifecycle or concurrency rule only with
its model and a broken control that must fail.

## Using the result in the app

`bboxy450-term ctl plugin trust <path>` copies and pins the binary,
`ctl plugin check <candidate>` runs conformance, `ctl plugin setup
<candidate>` runs its login, `ctl plugin enable <candidate>` activates it
(Settings › Plugins does the same).

Project-owned contributions use the [Respectful Slop License 1.0-Draft](LICENSE);
see [ATTRIBUTION.md](ATTRIBUTION.md) and [NOTICE](NOTICE).

Transport parameters are declared with `Manifest::transport_field(name, label, default)`.
The trusted shape supplies defaults and field names; unknown or missing fields
are refused before `wrap`. Return literal argv, never a shell-expanded field.
Settings > Machines lists discovered transports, and enabled transports appear
as `plugin:<provider>` Via choices. Built-in ssh/mosh/et remain available.
