# bboxy450-term-plugin-sdk

Build bboxy450-term plugins in Rust. A plugin is one executable that prints its
shape with `--shape` (or with no arguments and a closed stdin) and otherwise
serves newline-delimited JSON-RPC MCP on stdio. The SDK owns that runtime; a
plugin implements [`Plugin`] and calls [`run`].

```rust,ignore
use bbt_plugin_sdk::{Contract, Ctx, Init, Manifest, Plugin, Refusal, run};
use serde_json::{Value, json};

struct Echo;

impl Plugin for Echo {
    fn manifest() -> Manifest {
        Manifest::new("bboxy450-term-repo_graph-echo", "Echo", "Example")
            .contract(Contract::RepoGraph)
    }
    fn initialize(_init: Init, _ctx: &Ctx) -> Result<Self, Refusal> {
        Ok(Echo)
    }
    fn call(&mut self, _c: Contract, method: &str, args: Value, _ctx: &Ctx) -> Result<Value, Refusal> {
        Ok(json!({"method": method, "args": args}))
    }
}

fn main() {
    run::<Echo>();
}
```

What the runtime does for every plugin:

- `--shape`: the descriptor built from [`Manifest`], with command schemas
  projected from the `bboxy450-plugin` contract schemas.
- `initialize` binds one root per process; `tools/list`, `tools/call`,
  `resources/list`, `resources/read` (skills), `ping`.
- Requests run one at a time on a worker thread; `notifications/cancelled`
  flips the request's [`Ctx`]; each call gets a deadline just inside its
  declared `timeout_ms`.
- Frame (8 MiB), result (256 KiB), pending-request (8) and duplicate-id
  limits; authority fields (`root`, `author`, `signer`, `device`, `account`)
  in tool arguments are refused; panics become `internal_error` results.

Seats see every contract method they may call as
`plugin_<contract>_<provider>_<method>`. For anything beyond the contracts,
declare a seat tool with `Manifest::tool(name, description, input_schema,
output_schema, RoleClass::Worker | RoleClass::Oracle)` and serve it in
`Plugin::call_tool`; seats see it as `plugin_tool_<provider>_<name>` (at most
64 bytes, else refused at trust), and the plugin serves it as MCP tool
`tool_<name>`. The host validates arguments and results against the declared
schemas and treats every declared tool as an effect, admitted once per op_id.

[`testkit::Client`] drives a built plugin executable over stdio for
conformance tests and probes.

Project-owned contributions use the [Respectful Slop License 1.0-Draft](LICENSE);
see [ATTRIBUTION.md](ATTRIBUTION.md) and [NOTICE](NOTICE).
