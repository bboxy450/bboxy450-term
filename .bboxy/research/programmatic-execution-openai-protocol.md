---
id: 01M6PZ3VDHC410ZTAB1XMRAGC3
number: 93377961364431
type: research
title: "OpenAI programmatic calling and MCP boundaries"
status: complete
created: "2026-11-05T23:01:08Z"
updated: "2026-11-05T23:01:08Z"
links:
  - { to: "[[programmatic-tool-access]]", rel: context }
---
# OpenAI program execution and MCP boundaries

## What does OpenAI actually implement?

### Takeaway

OpenAI documents hosted tool orchestration separately from MCP transport and Codex hooks. [Programmatic Tool Calling](https://developers.openai.com/api/docs/guides/tools-programmatic-tool-calling)

### Cited Findings

- Enable `programmatic_tool_calling` and mark eligible tools, including MCP, with `allowed_callers`. JavaScript runs in fresh isolated V8 instances: no Node, filesystem, subprocesses, direct networking, packages or persistent program state. Code filters intermediate results and emits selected output. [Programmatic Tool Calling](https://developers.openai.com/api/docs/guides/tools-programmatic-tool-calling)
- Client-owned calls remain application-executed. `program`/nested `function_call`/`program_output` items preserve correlation; outputs copy `caller` to resume the program. An opaque fingerprint supports continuation. MCP approval policy may suspend execution. Deferred tools must be loaded before starting; programs cannot invoke tool search. [Programmatic Tool Calling](https://developers.openai.com/api/docs/guides/tools-programmatic-tool-calling)
- Direct calls are recommended for isolated actions, fresh judgment, approval-sensitive writes and native evidence validation. Per-call argument/permission checks and safe effect handling remain necessary. No universal speedup follows. [Programmatic Tool Calling](https://developers.openai.com/api/docs/guides/tools-programmatic-tool-calling)
- Codex documents nested JavaScript calls passing through tool hooks: pre-hooks can reject before execution or rewrite input; a blocking post-hook can reject the promise **after the effect occurred**. `continue:false` changes model-visible feedback without rejecting that promise. Some specialized paths bypass default hooks; these hooks are not a complete security boundary. [Hooks](https://learn.chatgpt.com/docs/hooks#tool-calls-from-code-mode)
- Current Codex configuration documents direct-only and excluded code-mode namespaces, and MCP tool omission from direct/deferred/code-mode surfaces. Its code-mode feature entry is described as under development and off by default; this does not establish availability in every product/version/account. [Configuration reference](https://learn.chatgpt.com/docs/config-file/config-reference)

Primary-source audit, fetched **2026-11-05** (actual final URLs; no paid API calls or runtime experiments):

| Source | Established by the fetched page |
|---|---|
| [OpenAI programmatic calling](https://developers.openai.com/api/docs/guides/tools-programmatic-tool-calling) | Hosted runtime, eligible tools, caller correlation, continuation and constraints. |
| [OpenAI hooks](https://learn.chatgpt.com/docs/hooks) | Documented Codex nested-call hook behavior. |
| [OpenAI config](https://learn.chatgpt.com/docs/config-file/config-reference) | Feature/surface configuration. |
| [MCP tools 2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28/server/tools) | Schemas, content, errors and explicit state handles. |
| [MCP Tasks overview](https://tasks.extensions.modelcontextprotocol.io/) | Extension architecture and negotiation. |
| [MCP Tasks specification](https://tasks.extensions.modelcontextprotocol.io/specification/2026-07-28/tasks) | Status and cancellation semantics. |
| [MCP Python migration](https://py.sdk.modelcontextprotocol.io/migration/) | Current extension implementation gap. |
| [MCP TS v2 errors](https://ts.sdk.modelcontextprotocol.io/v2/servers/errors) | SDK-specific error wrapping and hint placement. |

### Inferences

- Separate the execution runtime, tool broker, model-facing output and durable call journal. Choosing Python rather than JavaScript does not replace those components.
- Do not assume that a bboxy server-side script's internal calls automatically inherit Codex's nested-tool hooks. A host observing one outer MCP call cannot necessarily see a server's private call graph. Bboxy must retain its own per-call authority/audit enforcement; any claim of native host callback coverage needs an integration test.
- An MCP endpoint is a reasonable **product interface**; the language SDK becomes its internal binding layer. Existing native JS Code Mode can coexist rather than being replaced.

### Gaps

- These pages do not prove the precise private implementation of the current desktop app, account entitlement, benchmark advantage for bboxy, or compatibility of a future bboxy Python tool with every harness.
- No Python sandbox technology is prescribed by the cited OpenAI documentation. Running CPython in a subprocess alone must not be described as filesystem/network isolation.

## What does MCP standardize, and what would be bboxy-specific?

### Takeaway

MCP standardizes discovery, schemas and tool invocation/results. A `code_execute` name, Python bindings, interpreter, execution handle and nested-call broker remain server design. Standard Tasks can provide an optional asynchronous adapter, but protocol/SDK versions matter. [MCP tools](https://modelcontextprotocol.io/specification/2026-07-28/server/tools), [Tasks](https://tasks.extensions.modelcontextprotocol.io/)

### Cited Findings

- `tools/list` exposes currently authorized tools with schemas; its result may depend on request authorization. `structuredContent` carries typed JSON, and compatible text serialization is recommended. Actionable execution failures use `isError:true`; protocol errors are distinct. Stateful tools should return explicit opaque handles; authenticated servers reauthorize each use. Latest tools documentation resolved to revision **2026-07-28**. [MCP tools](https://modelcontextprotocol.io/specification/2026-07-28/server/tools)
- The current Tasks extension identifier is `io.modelcontextprotocol/tasks`. A handle must be durably findable before creation is returned. Clients advertise extension support on requests; nonparticipating clients must not receive an unexpected task-result shape. The client polls `tasks/get`, and can supply requested input through `tasks/update`. There is no `tasks/list` in this lifecycle. [Tasks overview](https://tasks.extensions.modelcontextprotocol.io/)
- Cancellation is cooperative: an acknowledgement does not prove work stopped or effects were undone. `completed` can contain an `isError:true` tool result; `failed` is reserved for JSON-RPC execution faults. The caller must inspect the final tool payload, not just the task status. [Tasks specification](https://tasks.extensions.modelcontextprotocol.io/specification/2026-07-28/tasks)
- The Python SDK v2 migration page says the old experimental task runtime is removed, and that its current SDK does **not yet implement** the redesigned 2026 extension. Do not infer deployable support from protocol documentation alone. [Python migration](https://py.sdk.modelcontextprotocol.io/migration/)
- The TypeScript SDK v2 error guide advises putting actionable recovery hints in text and returns tool-handler errors as `isError:true`. Its SDK choices are not universal protocol guarantees: the specification says clients **may** expose protocol errors to models. Preserve machine-readable correction data plus concise compatible text. [TS errors](https://ts.sdk.modelcontextprotocol.io/v2/servers/errors), [MCP tools](https://modelcontextprotocol.io/specification/2026-07-28/server/tools)

### Inferences

- A normal MCP `code_execute` tool can return a custom execution handle in ordinary structured tool content today; that does not claim MCP Tasks support. Add the Tasks extension only after negotiated protocol/client/SDK compatibility is verified.
- Do not introduce custom JSON-RPC methods called `tasks/*` with different semantics. Bboxy's task assignment records and MCP asynchronous Tasks are distinct concepts.
- Neither JSON-RPC success nor a completed execution proves every domain operation succeeded. Retain `isError`, original tool data, corrective hints and per-operation identity all the way through Python exceptions and the outer result.

### Gaps

- The fetched sources do not establish which current Claude/Codex clients support the redesigned Tasks extension. Test the actual clients before enabling it.
- Existing bboxy bridge versions stop at `2025-11-25`; current 2026 protocol examples are design context, not schemas to paste into that bridge unchanged. [Local bridge](</Users/feral/mydev/bboxy450-term/crates/bbt-mux/src/mcp/bridge.rs:23>)

## What endpoint boundary should bboxy adopt?

### Takeaway

Recommendation: make a **Python MCP execution tool** the cross-harness entry point, using a small generated/dynamic tool client internally. Keep native JS Code Mode available. This is a design inference, not a sourced claim that bboxy already implements it.

### Cited Findings

- The current bboxy bridge reads seat/generation admission, forwards each native request with a ten-second read timeout, and processes stdio requests serially. Long-running execution cannot simply occupy that loop while requiring callbacks through it. [Admission](</Users/feral/mydev/bboxy450-term/crates/bbt-mux/src/mcp/bridge.rs:43>), [deadline](</Users/feral/mydev/bboxy450-term/crates/bbt-mux/src/mcp/bridge.rs:179>), [loop](</Users/feral/mydev/bboxy450-term/crates/bbt-mux/src/mcp/bridge.rs:443>)

### Inferences

1. Expose proposed `code_execute(language:"python", source, op_id)` plus bounded execution read/cancel operations (names provisional). Return final compact output for short runs, or a durable handle promptly for longer work. Keep the control owner loop responsive.
2. Execute on a dedicated worker with an explicit process/resource/cancellation profile. Separate orchestration-only access from full local Python filesystem/network authority; do not quietly grant the latter to seats that lacked normal execution authority.
3. Derive available `tools` functions and input/output descriptions from the admitted catalogue. Keep argument schemas and domain validation authoritative at each actual dispatch; do not hardcode a second set of wrappers.
4. Capture the originating seat/generation, root/provider binding and grant epoch. Revalidate on every nested call. Never use an operator CLI or provider credentials as an easier callback route.
5. Persist execution/source identity and child operation IDs before effects; record a parent-child graph. Output loss, timeout, cancellation or post-tool rejection is not permission to replay the whole script. Reconcile each uncertain operation through the existing journal.
6. Filter intermediate payloads locally, but preserve resources/native evidence and return structured failures with actionable text. Keep the full call audit available in the UI while allowing concise model-visible output.
7. First validate with a real MCP client: tool discovery; nested read/write errors; delayed callback without deadlock; lost response after a successful write; revoked generation; cancellation racing completion; output paging/artifacts; cold-process restart. Measure end-to-end behavior against the existing SDK/heredoc route before promising latency/token gains.

### Gaps

- Interpreter isolation choice, supported Python packages, exact process custody and tool availability need a follow-up implementation contract. Avoid selecting a cloud provider or adding a full execution platform just to expose this interface.
- Research and notes only: no source edits, API executions, paid resources, backend restarts or MCP endpoint implementation were performed.
