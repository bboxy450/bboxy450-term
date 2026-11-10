---
id: 01M6PZ3SR8J52REV2HR1KMTWXK
number: 93377961364430
type: research
title: "Programmatic execution implementations"
status: complete
created: "2026-11-05T23:01:08Z"
updated: "2026-11-05T23:01:08Z"
links:
  - { to: "[[programmatic-tool-access]]", rel: context }
---
# Programmatic execution implementations outside OpenAI

## How do current implementations expose tools to generated code?

### Takeaway

An MCP execution endpoint is an established implementation pattern, including a maintained Python implementation. The common structure is a small model-facing execution/discovery surface, ordinary language control flow, and host-mediated calls to the original tools. An external SDK is one distribution option, not a prerequisite.

### Cited Findings

| Primary source, fetched 2026-11-05 | Documented mechanism and scope |
|---|---|
| [Anthropic programmatic tool calling](https://platform.claude.com/docs/en/agents-and-tools/tool-use/programmatic-tool-calling) | Python runs in a managed container. A tool function pauses execution and produces a client `tool_use`; the client supplies its result and execution continues. Intermediate results stay out of model context. Tools opt in through `allowed_callers`, and response `caller` identifies the execution. **MCP connector tools cannot currently be called programmatically** through that managed feature; applications must not assume native MCP passthrough. |
| [Anthropic MCP engineering article](https://www.anthropic.com/research/code-execution-with-mcp), published 2025-11-04 | A distinct, self-managed pattern: generate TypeScript files representing MCP tool interfaces, backed by `callMCPTool`. Discover via files or tool search; filter data inside code. Reusable scripts/files provide state but the article does not supply a durable operation journal. Its 150,000→2,000 token example concerns that example/tool-discovery setup, not a universal execution speedup. |
| [Cloudflare MCP server guide](https://developers.cloudflare.com/agents/model-context-protocol/guides/build-codemode-mcp-server/) | `codeMcpServer` exposes one `code` MCP tool, with generated TypeScript declarations and JavaScript `codemode.*` calls. For large OpenAPI catalogs, `openApiMcpServer` exposes `search` and `execute`. Credentials remain in host callbacks. The guide explicitly requires authorization/approvals in upstream handlers: the simple wrapper provides no durable per-operation approval itself. Approximate final-output limit: 6,000 estimated tokens with marked truncation. |
| [Cloudflare MCP connector](https://developers.cloudflare.com/agents/tools/codemode/mcp/), updated 2026-10-27 | `McpConnector` wraps an existing client connection, creates typed methods from MCP schemas, and calls original tools via `client.callTool`. It integrates with the durable runtime's discovery/approval/history. Identifier normalization detects collisions. The smaller `createCodeTool` adapter remains available but lacks that durable flow. |
| [FastMCP CodeMode](https://gofastmcp.com/servers/transforms/code-mode), introduced 3.1.0, experimental | A transform over normal registered tools exposes `search`, `get_schema`, `execute`. Python uses `await call_tool(name, params)`. Discovery is configurable and request-scoped. Calls traverse FastMCP with the current request's ordinary authorization. Default Monty limits described here: 30s code duration, 100MB allocation budget, 50 host calls. Host-tool wait time is excluded from code-duration limits. |

- Cloudflare's current design exposes generated methods through connectors; schema/type declarations derive from connector schemas, not hand-maintained wrappers. Its sandbox has no Node `process`/`require` or ambient credentials. Calls cross a Workers RPC bridge. [How Code Mode works](https://developers.cloudflare.com/agents/tools/codemode/how-it-works/)
- The current executor interface separates execution from orchestration. `DynamicWorkerExecutor` defaults to blocking direct network egress and currently documents a 60,000ms timeout. Older February documentation says 30s; prefer the current reference rather than treating old defaults as immutable. [Current API reference](https://developers.cloudflare.com/agents/tools/codemode/api-reference/), [February rewrite](https://developers.cloudflare.com/changelog/post/2026-02-20-codemode-sdk-rewrite/)

### Inferences

- Bboxy's endpoint-first proposal is supported by multiple actual implementations. Python need not be replaced with JavaScript merely to expose this capability over MCP.
- Generated convenience methods are useful, but one generic `call_tool` backed by the existing catalogue is enough for a first version. New per-tool SDK classes would duplicate the contract unnecessarily.
- The benefit comes from fewer model round trips and filtered intermediate results. Changing shell syntax into an MCP call alone does not establish that benefit; SDK scripts already support batching.

### Gaps

- These sources do not prove Codex or Claude ACP will automatically prefer a newly installed bboxy endpoint. Orientation/tool descriptions and representative harness tests remain necessary.
- Cloudflare's simple MCP wrapper and durable connector runtime are different integrations; the former's existence is not evidence that all Code Mode wrappers have crash recovery.

## How do runtimes handle isolation, interruption and side effects?

### Takeaway

The execution VM and the durable side-effect coordinator are separate responsibilities. The strongest useful reference is Cloudflare's explicit recorded-call replay, supplemented by Monty's subprocess and suspension APIs; neither removes the need to reconcile an external effect whose result was lost.

### Cited Findings

- Cloudflare stores executions, connector-call results and pending approvals in isolated SQLite storage. Pausing for approval aborts a pass; a subsequent pass uses recorded results for applied calls. Rejecting does not undo earlier actions. This is durable orchestration, not a transaction around external APIs. [Durable runtime](https://developers.cloudflare.com/agents/tools/codemode/durable-runtime/)
- Its replay log checks method/argument/order agreement; nondeterministic sandbox work needs recorded `step()` results. `Promise.all()` may reorder calls and cause divergence. Calls are recorded `executing` before invocation and `applied` after saving their result. Thus a process loss between those points is a real boundary, not proof of exactly-once effect. Large replay values fail instead of being truncated; final presentation can be reduced separately. [Execution and replay mechanics](https://developers.cloudflare.com/agents/tools/codemode/how-it-works/)
- Monty is a Rust Python parser/VM with a stdlib subset. For untrusted input its Rust guide recommends `monty-pool`, which runs the interpreter only in worker subprocesses. It exposes suspension events for host functions, OS requests and futures, and supports session dump/restore. Local pools have no request timeout by default: configure one. Only confirmed child exit proves termination. [Rust quickstart](https://pydantic.dev/docs/monty/quickstart/rust/)
- Suspended Monty state can serialize and restore on another worker. Snapshot provenance/integrity and compatibility are the caller's obligations; snapshots are not authenticated or comprehensively validated by Monty. [Snapshots](https://pydantic.dev/docs/monty/concepts/snapshots/)
- A host callback's filesystem/network authority remains the host's responsibility. In-process Monty lacks crash isolation; an interpreter abort can kill its caller. Worker resolution must be explicitly pinned for untrusted code. Heap allocation limits are not process-RSS limits. [Security model](https://pydantic.dev/docs/monty/concepts/security/)
- Monty is not arbitrary CPython: limitations include inheritance/metaclasses, generator functions, user-defined exception classes and several async constructs. It requires a compatibility assessment before claiming existing heredocs run unchanged. [Limitations](https://pydantic.dev/docs/monty/limitations/)
- Anthropic documents a roughly four-minute pending-tool timeout and that generated code typically retries after expiry. This is behavior to account for, not a safe retry policy for mutating bboxy operations. It identifies large fan-out/result-filtering work as strong fits and small or model-dependent sequential tasks as weak fits. [Programmatic tool calling](https://platform.claude.com/docs/en/agents-and-tools/tool-use/programmatic-tool-calling)

### Inferences

- A subprocess is a crash boundary, not automatically a filesystem/network sandbox. Ordinary CPython with unrestricted imports can bypass a tool-only journal through files, network or subprocesses.
- Monty is a plausible **candidate to evaluate** for a Rust application wanting Python orchestration with host callbacks. Its supported subset, packaging, resource behavior and exact pinned-version APIs need tests. Do not embed its interpreter in bboxy's supervisor merely because the crate makes that possible.
- For bboxy, reuse the existing per-operation journal and authority checks for every nested MCP mutation. A lost outer result or interpreter exit must never trigger unconditional replay of the script. Cloudflare's recorded-call architecture is useful inspiration; automatic whole-script deterministic replay should be a separately verified feature.

### Gaps

- I found no documented blanket exactly-once external-side-effect guarantee in these integrations. Applied-result caching does not close the provider-effect/before-result-recording crash window.
- Reviewed FastMCP docs do not establish durable script continuation after server restart. Monty snapshots offer a mechanism, not a complete authorization/operation/recovery policy.
- No bboxy workload benchmarks, production adoption rankings or independent comparative security audit were performed. Published token/latency claims cannot be transferred to bboxy.

## What should change in bboxy's proposed endpoint?

### Takeaway

Prefer a Python execution MCP endpoint as the common harness-facing entry point, with a small internal client/dispatcher providing admitted tool access. Preserve direct MCP calls for simple work; do not require every single call to be wrapped in generated code.

### Cited Findings

- Endpoint-based Python orchestration and ordinary current-context authorization are already documented together by FastMCP. [CodeMode](https://gofastmcp.com/servers/transforms/code-mode)
- Discovery can be separated from execution for large catalogs; authentication and side-effect authorization still belong in host callbacks. [Cloudflare MCP server guide](https://developers.cloudflare.com/agents/model-context-protocol/guides/build-codemode-mcp-server/)
- Durable control can surround a transient executor rather than being implemented inside a model/VM. [Cloudflare durable runtime](https://developers.cloudflare.com/agents/tools/codemode/durable-runtime/)

### Inferences

Proposed bboxy boundary, not an existing API:

1. `code_execute(op_id, language="python", source, inputs?)` creates a recoverable execution, captures current seat/generation/grant/source-KB binding and returns bounded output or a stable execution handle promptly. The Python runtime executes outside the supervisor/owner loop. Keep the runtime/provider seam small; JS is optional future support, not a prerequisite.
2. Inject one `call_tool(name, arguments)` function and optional generated helpers, using **the actual role-filtered schema/catalogue and original dispatcher**. Nested calls get recoverable operation IDs before send; errors retain full structured results/hints/refs/etags. The child receives no operator door or ambient host credentials. Revalidate authority for every call, including after a suspension/restart.
3. Reuse existing catalogue/discovery facilities first. Add bounded schema search only when the catalogue warrants it. Discovery returns schema to the model when it must learn unfamiliar methods; silently reading orientation in Python does not mean the model read it.
4. `code_get(execution_id, cursor?)` is justified by durable stdout/result paging and nested-call receipts; ordinary operation status alone does not necessarily provide those. `code_cancel` is justified if no existing cancellation API can target the exact owned execution. Cancellation stops new work and reaps that exact child; it cannot undo earlier committed actions or establish the outcome of an in-flight provider write.
5. Bound wall/CPU/heap/RSS headroom, host-call count, result/log bytes and child lifetimes independently. Large results become explicit references/pages, preserving error/recovery information. UI visibility should show nested effects even when only a concise program result reaches model context.
6. Keep orchestration Python's capability set explicit. If arbitrary local CPython scripts are supported, apply the harness's existing execution policy and never advertise tool-only isolation. Candidate Monty worker execution could provide a narrower first contract, but does not promise compatibility with filesystem/subprocess heredocs.

Given bboxy's inherited synchronous stdio loop and ten-second read deadline (parent-provided local evidence), a long-running outer request cannot depend on a nested request being serviced by that same blocked loop. Return a handle and use an independent authenticated callback/dispatcher path; never hold a control-plane lock while waiting on Python or a nested provider.

### Gaps

This recommendation is a proposed design change for the parent to consider. I did not edit the KB spec or implementation, run paid APIs, deploy Cloudflare resources or validate new endpoints against real harnesses.
