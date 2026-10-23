---
id: 01M5N54H516F3WKJMB2SARE8TQ
type: research
title: "plugin systems for developer tools and Rust programs, state of the art (2026-10-23)"
links:
  - { to: "[[plugins]]", rel: evidence-for }
  - { to: "[[2026-10-22-architecture-round-1]]", rel: decision }
created: 2026-10-23T19:52:18Z
updated: 2026-10-23T20:13:28Z
---
# Plugin systems for developer tools and Rust programs

Researched 2026-10-23 by an Oracle-style research subagent under the Architect's brief, read-only, nothing built. Format: `source — fact`; `unverified` marks claims not confirmed from a primary source. The design this evaluates is [[plugins]] §0 (the operator's words). No recommendations outside §8 and §9; the decision is #41. **Red-team dispositions (Wren, 16b4c98):** the Codex hooks line in §6 is `unverified` until confirmed against the openai/codex source at a pinned revision; the latency rows of §8 are judgement, replaced by P-0's measurement; three precedents (GitHub CLI extensions, Helm plugins, confining a subprocess on macOS) and permalinks at pinned revisions with retrieval dates for the load-bearing claims arrive as an addendum from a second research pass.

## 1. Subprocess plugin protocols

### Docker CLI plugins
- https://pkg.go.dev/github.com/docker/cli/cli-plugins/metadata — `NamePrefix = "docker-"`; `MetadataSubcommandName = "docker-cli-plugin-metadata"`; `HookSubcommandName = "docker-cli-plugin-hooks"`; `ReexecEnvvar = "DOCKER_CLI_PLUGIN_ORIGINAL_CLI_COMMAND"`.
- same — `Metadata{SchemaVersion ("Mandatory, must be \"0.1.0\""), Vendor ("Mandatory"), Version, ShortDescription, URL, Hidden, Features map[string]any}`. The metadata is not printed on a bare run; it is a dedicated subcommand.
- https://github.com/docker/cli/blob/master/cli-plugins/manager/plugin.go — `validateSchemaVersion` accepts `0.1.0` and any `<major>.<minor>.<patch>` below `2.0.0`; name regex `^[a-z][a-z0-9]*$`; errors "plugin %q duplicates builtin command", "plugin metadata does not define a vendor", "invalid metadata".
- https://github.com/docker/cli/blob/master/cli-plugins/manager/manager.go — search order `cfg.CLIPluginsExtraDirs`, then `config.Dir()/cli-plugins` (usually ~/.docker/cli-plugins), then `defaultSystemPluginDirs` (values `unverified`); duplicate names kept as `ShadowedPaths`; plugin run via `exec.Command(plugin.Path, args...)` inheriting stdin/stdout/stderr, env gets `ReexecEnvvar`.
- https://pkg.go.dev/github.com/docker/cli/cli-plugins/hooks — `Request{RootCmd, Flags, CommandError}` serialized as JSON and passed to the hook subcommand as a command-line argument; `Response{Type, Template}`; "Plugins should ignore unknown fields and unknown hook types to remain forwards-compatible."
- https://chromium.googlesource.com/external/github.com/docker/cli/+/master/cli-plugins/manager/hooks.go — hooks configured per plugin under `plugins.<name>.hooks` / `error-hooks` in config.json; no deadline added; failures only `logrus ... Debug("Plugin hook invocation failed")`. plugin.go `RunHook` — `exec.CommandContext(ctx, p.Path, p.Name, "docker-cli-plugin-hooks", <json>)`.

### kubectl plugins and krew
- https://kubernetes.io/docs/tasks/extend-kubectl/kubectl-plugins/ — "a standalone executable file, whose name begins with `kubectl-`"; no installation or pre-loading; `kubectl plugin list` traverses all of PATH, warns on non-executable and overlapping names; args and flags passed as-is; no plugin-specific env; longest-match resolution; cannot override builtins; "Kubectl plugins available via the Krew plugin index are not audited for security."
- https://krew.sigs.k8s.io/docs/developer-guide/plugin-manifest/ — `apiVersion: krew.googlecontainertools.github.com/v1alpha2`, `kind: Plugin`; `version` semver with the `v` prefix required; `platforms[]{selector{os,arch}, uri, sha256, bin, files}`; krew symlinks `kubectl-<name>`, dashes become underscores.
- https://github.com/kubernetes-sigs/krew-index — admission "evaluated on a case-by-case basis"; no security-audit statement.

### git and cargo subcommands
- https://git-scm.com/docs/git — lookup: core program in `GIT_EXEC_PATH`, then `git-<command>` on `$PATH`, then alias; `--list-cmds=others` lists `git-` prefixed commands on PATH.
- https://git-scm.com/docs/gitcredentials — helper string `foo` becomes `git credential-foo`; one operation argument appended: `get`/`store`/`erase`; attributes on stdin/stdout; helpers tried in turn; `quit=true` stops the chain.
- https://doc.rust-lang.org/cargo/reference/external-tools.html — `cargo foo` runs `cargo-foo` on `$PATH`; `$CARGO_HOME/bin` prioritised over `$PATH`; argv[1] is the subcommand name; `cargo help foo` runs `cargo-foo foo --help`; `CARGO` env var to call back; "Cargo as a library is unstable."

### Nushell plugins
- https://www.nushell.sh/book/plugins.html — binaries `nu_plugin_*`; `plugin add <path>` runs the binary, asks it for its commands, saves to the registry file at `$nu.plugin-path`; `plugin list` columns `name, version, status (loaded|running), pid, filename, shell, commands`; `plugin stop`; GC `$env.config.plugin_gc` default `enabled: true, stop_after: 10sec`, per-plugin overrides; "Plugins must use the same `nu-plugin` version as the Nushell release."
- https://www.nushell.sh/contributor-book/plugin_protocol_reference.html — invoked with `--stdio` (required) or `--local-socket <path>` (feature `LocalSocket`, falls back to stdio); first bytes `\x04json` or `\x07msgpack`; first message `Hello{protocol:"nu-plugin", version, features}`; compatibility by caret on the lower version, `0.x` minors incompatible, engine refuses to load otherwise; engine→plugin `Call [id, Metadata|Signature|Run|CustomValueOp]`, `Signal Interrupt|Reset`, `Goodbye`; plugin→engine `CallResponse`, `EngineCall`, `Option{GcDisabled}`; streams `Data/Ack/Drop/End` with per-message Ack flow control; "Multiple calls may be in flight"; errors `LabeledError{msg, labels, code, url, help, inner}`; stderr free for messages.

### HashiCorp go-plugin and Terraform
- https://raw.githubusercontent.com/hashicorp/go-plugin/main/README.md — "A panic in a plugin doesn't panic the plugin user"; stdout/stderr mirrored back to the host; checksum verification and TLS; reattach for host upgrade; "only designed to work over a local [reliable] network."
- https://github.com/hashicorp/go-plugin/blob/main/docs/internals.md — plugin prints one handshake line on stdout `CORE-PROTOCOL-VERSION|APP-PROTOCOL-VERSION|NETWORK-TYPE|NETWORK-ADDR|PROTOCOL`, e.g. `1|3|unix|/path/to/socket|grpc`; core version must be `1`; network `unix` or `tcp`; `PLUGIN_MIN_PORT`/`PLUGIN_MAX_PORT`.
- https://pkg.go.dev/github.com/hashicorp/go-plugin — magic cookie "is not a security measure, just a UX feature"; `ProtocolVersion` clients must match; `StartTimeout`; `VersionedPlugins` map per protocol version; `AutoMTLS`; `SecureConfig` checksum; `Client.Kill` blocks until exit; `CleanupClients` before the parent exits; `SkipHostEnv`.
- https://raw.githubusercontent.com/hashicorp/go-plugin/main/server.go — reads `PLUGIN_PROTOCOL_VERSIONS`, iterates in reverse to match the newest compatible version; bare run without cookie prints "This binary is a plugin. These are not meant to be executed directly."; after handshake `os.Stdout`/`os.Stderr` are swapped to pipes streamed to the host.
- https://developer.hashicorp.com/terraform/plugin/terraform-plugin-protocol — protocol 5 (CLI 0.12+) and 6 (CLI 1.0+), gRPC/protobuf; major versions delineate compatibility, minor versions are additive.
- https://developer.hashicorp.com/terraform/internals/provider-registry-protocol — package response `protocols: ["4.0","5.1"]` so the CLI avoids downloading an incompatible package; `shasums_url`, `shasums_signature_url` (detached GPG), `signing_keys.gpg_public_keys[]`.

### LSP and MCP
- https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/ — `Content-Length` header + JSON-RPC 2.0; `initialize` first and once, earlier requests get `-32002 ServerNotInitialized`; `processId`: if the parent is not alive the server should exit; `$/cancelRequest`: a cancelled request still returns a response, code `-32800`; `$/progress` tokens; `shutdown` then `exit`, exit code 0 if shutdown was received, else 1.
- https://modelcontextprotocol.io/specification/2025-06-18/basic/transports — stdio: newline-delimited JSON-RPC, no embedded newlines; the server may write UTF-8 to stderr for logging; "The server MUST NOT write anything to its stdout that is not a valid MCP message."
- https://modelcontextprotocol.io/specification/2025-06-18/basic/lifecycle — `initialize{protocolVersion, capabilities, clientInfo}` → `{protocolVersion, capabilities, serverInfo, instructions}` → `notifications/initialized`; server answers the same version or its latest, client should disconnect if unsupported; capabilities `tools/resources/prompts/logging/completions/experimental`, sub-caps `listChanged`, `subscribe`; implementations should set timeouts for all sent requests, progress may reset the clock but a maximum timeout should always be enforced; shutdown: close stdin → `SIGTERM` → `SIGKILL`.
- https://modelcontextprotocol.io/specification/2025-06-18/basic/utilities/cancellation — `notifications/cancelled{requestId, reason}`; `initialize` must not be cancelled; receivers should stop and not respond; late responses ignored.
- https://modelcontextprotocol.io/specification/2025-06-18/server/tools — `tools/list` (paginated) returns `Tool{name, title, description, inputSchema, outputSchema, annotations}`; `tools/call` → `{content[], structuredContent, isError}`; protocol errors (JSON-RPC) versus execution errors (`isError: true`); annotations untrusted unless the server is trusted; clients should implement timeouts and log tool usage.

### Process-isolated extension hosts (editors and terminals)
- https://code.visualstudio.com/api/advanced-topics/extension-host — misbehaving extensions must not impact the user experience; host kinds local Node.js / web worker / remote; lazy activation. https://code.visualstudio.com/docs/configure/extensions/extension-runtime-security — the extension host "has the same permissions as VS Code itself."
- https://www.sublimetext.com/docs/api_environments.html — plugins run in a separate `plugin_host` process shared by all plugins; per-package `.python-version`.
- https://sw.kovidgoyal.net/kitty/remote-control/ — GPU terminal precedent: control over a UNIX socket (`--listen-on`), gated by `allow_remote_control` or `remote_control_password` with per-password action limits; `kitten @ ls` emits a JSON tree.
- https://wezterm.org/config/plugins.html — plugins are Lua repos fetched by `wezterm.plugin.require(<url>)` into `plugins/NAME`, entry `plugin/init.lua` exporting `apply_to_config`; no sandbox or trust statement.

## 2. In-process plugin models in Rust
- https://doc.rust-lang.org/reference/type-layout.html — "Type layout can be changed with each compilation"; `repr(C)` is the interop representation.
- https://github.com/rust-lang/rfcs/pull/3470 — crABI RFC still open; reviewers propose 0.1.0 because it "has not seen actual usage".
- https://docs.rs/libloading/latest/libloading/struct.Library.html — `Library::new` and `get` are unsafe; users must specify the correct type; `dlerror` not MT-safe on some UNIX; `Drop` ignores close errors.
- https://raw.githubusercontent.com/rodrimati1992/abi_stable_crates/master/readme.md — Rust-to-Rust FFI with load-time type checking; "assumes that dynamic libraries come from a benign source"; non-feature: unloading; each `0.y.0` and `x.0.0` version defines its own ABI; `sabi_extern_fn` aborts on unwind.
- https://docs.rs/stabby/latest/stabby/ — stable ABI with compact sum types; `#[stabby::export(canaries)]` blocks linkage on incompatible versions; no async traits yet; vtables leaked on Rust ≥ 1.78; any representation change is a MAJOR bump; docs.rs build of 72.1.16 failed.
- https://tremor.rs/rfc/accepted/plugin-development-kit — Tremor PDK chose shared libraries via `libloading`, forbids unloading; whether it shipped on `abi_stable`: `unverified`.
- https://vector.dev/highlights/2021-08-23-removing-wasm/ — Vector removed its experimental wasm transform citing poor user experience and performance, replaced by Lua.

## 3. WebAssembly plugin models
- https://github.com/extism/extism — host SDKs in 15 languages, PDKs in 10; host-controlled HTTP without WASI. https://docs.rs/extism/latest/extism/ — depends on `wasmtime ^49`; `ToBytes`/`FromBytes` define the boundary encoding; `CancelHandle` cancels a running plugin from another thread.
- https://extism.org/docs/concepts/manifest — `wasm[]{path|url|data, hash}`, `memory{max_pages, max_http_response_bytes, max_var_bytes}`, `config`, `allowed_hosts` (empty = none, null = all), `allowed_paths` (WASI only), `timeout_ms` interrupts execution.
- https://docs.wasmtime.dev/security.html — linear-memory bounds checks, 2 GB guard regions, instances can do only what they are linked with; Spectre mitigation ongoing. https://docs.wasmtime.dev/api/wasmtime/struct.Config.html — `consume_fuel`; epochs safe for untrusted code; wasm threads tier 2. `ResourceLimiter` veto hooks for memory and table growth.
- https://component-model.bytecodealliance.org/design/wit.html — WIT defines contracts between components; `namespace:name@<semver>`; worlds = imports + exports; resources; does not define package resolution.
- https://zellij.dev/documentation/plugins — WASI plugin system, Rust official, Go SDK community; 14 permissions via `request_permission`; events async with no fixed order; FS `/host`, `/data` (deleted on unload), `/tmp`; workers because wasm threads are not stable; forward compatibility promised. CHANGELOG: permissions 0.38.0 (2023-08-28); wasmer→wasmtime 0.41.0 (2024-11-04); wasmtime→wasmi 0.44.0 (2026-03-23).
- https://zed.dev/docs/extensions/developing-extensions — Rust compiled to `wasm32-wasip2`; `extension.toml`; versioned WIT dirs `since_v0.0.1 ... since_v0.8.0`; newer API not compatible with older Zed.
- Lapce — plugins in WASI languages; `volt.toml` schema `unverified`.

## 4. Discovery, naming, metadata, versioning, trust
- PATH scan: kubectl, git, cargo. Dedicated folder: Docker `cli-plugins` dirs; Nushell `NU_PLUGIN_DIRS` + registry cache; krew `~/.krew/bin` symlinks.
- Manifest file: krew YAML, Zed `extension.toml`, Extism manifest, Terraform registry JSON. Self-describing binary: Docker metadata subcommand, Nushell `Signature`/`Metadata` cached by `plugin add`, go-plugin handshake line, LSP/MCP `initialize` capabilities.
- Contract versioning: Docker `SchemaVersion < 2.0.0`; Nushell caret; go-plugin newest-match; Terraform MAJOR.MINOR; MCP date-string negotiation; LSP missing property = absent capability; Zed `since_v*`; Docker hooks ignore unknown fields.
- Signing and trust: krew sha256 only, not audited; Terraform GPG-signed shasums; Sigstore `cosign sign-blob --bundle` and `verify-blob` with certificate identity and OIDC issuer (https://docs.sigstore.dev/cosign/signing/signing_with_blobs/, /verifying/verify/); VS Code refuses install on failed signature verification (https://code.visualstudio.com/updates/v1_75).
- macOS: https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution.md — notarisation scans for malicious content; stapled or online tickets; requires Developer ID, Hardened Runtime, secure timestamp; "apps can load quarantined plug-ins ... only if the plug-in is notarized"; "Plug-ins don't declare their own entitlements. Instead, they inherit the entitlements of the host process." https://support.apple.com/en-us/102445 — Gatekeeper requires notarisation, "Open Anyway" override. Bare CLI binaries cannot be stapled; staple the dmg/pkg (Apple forum answer, secondary). Attribute name `com.apple.quarantine`: `unverified` from primary docs.

## 5. Auth and setup patterns
- https://datatracker.ietf.org/doc/html/rfc8628 — device authorization: `device_code, user_code, verification_uri, verification_uri_complete, expires_in, interval`; poll with the device-code grant; errors `authorization_pending`, `slow_down`, `access_denied`, `expired_token`.
- https://cli.github.com/manual/gh_auth_login — browser flow with a one-time device code; `--with-token`; `GH_TOKEN` for headless; secure storage default since v2.26.0 with plain-text fallback (https://github.com/cli/cli/releases/tag/v2.26.0). https://cli.github.com/manual/gh_auth_status — tests each account, refreshes near-expiry tokens, exits 1 with stderr on issues, `--json` always 0.
- https://docs.cloud.google.com/sdk/gcloud/reference/auth/login — browser flow; `--no-browser` remote bootstrap; `--no-launch-browser` paste flow; `--cred-file`; `gcloud auth list`.
- Credential-helper subprocess precedents: https://github.com/docker/docker-credential-helpers — `docker-credential-{osxkeychain,wincred,secretservice,pass}`, verb as argv[1] `store|get|erase|list`, JSON on stdin; git credential helpers (§1). Rust: https://docs.rs/keyring/latest/keyring/ — platform stores; `keyring-core` for apps choosing stores.

## 6. Hooks
- https://git-scm.com/docs/githooks — `$GIT_DIR/hooks` or `core.hooksPath`; non-executable hooks ignored; non-zero exit aborts; no timeout defined.
- https://pre-commit.com/ — hook def `id, name, entry, language, files, types, stages, pass_filenames, require_serial, always_run, fail_fast, verbose`; `--verbose` prints `- duration: 0.02s` per hook.
- https://typicode.github.io/husky/ — `core.hooksPath`, no timeout mention.
- https://code.claude.com/docs/en/hooks — ~30 events; handler types `command|http|mcp_tool|prompt|agent`; `timeout` seconds, defaults 600 (command/http/mcp_tool), 30 (prompt), 60 (agent), SessionEnd budget 1.5 s; JSON on stdin; exit 0 / 2 (block, stderr is the reason) / other (non-blocking); matching hooks run in parallel; fail-open unless `onFailure: "block"`; `async: true` not timed out; strings capped at 10,000 chars with file spill. Debug log: "Hooks: PostToolUse:Write [...] finished with status 0 (31ms)", also "timed out after <N>ms"; not shown in the session UI.
- https://learn.chatgpt.com/docs/hooks (Codex) — `~/.codex/hooks.json`, `config.toml [hooks]`, repo `.codex/`; `timeout` default 600 s, SessionEnd/Interrupt 1 s (max 3 s); exit 2 blocks; "Codex requires you to review and trust the exact hook definition", changed hooks skipped until re-trusted; no duration surfacing documented.
- Docker CLI hooks: no timeout, failures debug-only (§1). Nushell: idle GC `stop_after`. MCP: timeouts and cancellation (§1).

## 7. Registries and observability
- Nushell `plugin list` — per-plugin `status`, `pid`, `version`, `filename`, `commands`. kubectl `plugin list` — PATH order with shadow warnings. krew index — name, description, repository only.
- https://code.visualstudio.com/updates/v1_19 — Running Extensions view with performance information and profiling. https://github.com/microsoft/vscode/wiki/Explain-extension-causes-high-cpu-load — VS Code monitors the extension host and attaches a CPU profiler when unresponsive; `.cpuprofile` viewable in Speedscope. Startup Performance editor shows activation reason and times, warns on contributions blocking > 5 ms.
- Per-hook timing precedents: Claude Code debug lines (§6); https://git-scm.com/docs/api-trace2 — `child_start{child_class:"hook", hook_name, argv}` and `child_exit{code, t_rel}`; pre-commit `- duration`; https://doc.rust-lang.org/cargo/reference/timings.html — `--timings` HTML timeline; https://lazy.folke.io/usage — `:Lazy profile`.
- Flame graphs: the only precedent is VS Code's auto-captured `.cpuprofile` in Speedscope. No plugin registry found that renders per-hook flame graphs. Gap: no data.

## 8. Model comparison

| Criterion | Subprocess, stdio JSON-RPC (MCP/LSP/Nushell) | Subprocess, local socket (go-plugin, Nushell LocalSocket) | In-process dylib (libloading + abi_stable/stabby) | Wasm (Extism / wasmtime+WIT) |
|---|---|---|---|---|
| Crash isolation | Process boundary; LSP `processId`, MCP SIGTERM/SIGKILL | Same; go-plugin "panic in a plugin doesn't panic the plugin user" | None: plugin panic or UB takes the host | Trap caught; host survives |
| Language freedom | Anything that speaks JSON on stdio | Anything with a gRPC/socket client | Rust or C-ABI languages | PDK languages |
| Startup latency | Spawn per plugin; Nushell caches signatures and GCs idle plugins | Spawn + socket + handshake | dlopen, sub-ms | Module instantiate; caches help |
| Call latency | Serialise + pipe; MsgPack recommended by Nushell | Same class; HTTP/2 multiplexing | Function call | Near-native; byte copies at the boundary |
| Versioning | Negotiated (MCP protocolVersion, Nushell caret, LSP capabilities) | go-plugin PLUGIN_PROTOCOL_VERSIONS; Terraform protocols | Rust layout changes per compilation; per-0.y ABIs; canaries | WIT since_v* dirs, package@semver |
| Auth and login | Plugin owns it in its process (gh/gcloud pattern, keychain) | Same | Shares host credentials, no boundary | No network or FS unless allowed; login must be host-mediated |
| Hook support | One-shot exec per hook or long-lived request | Long-lived, multiplexed | Direct callbacks | Direct exported fn |
| Observability | Host measures wall time per call; internals opaque | Same; structured logs forwarded | Host profiler sees frames | Fuel counters, epoch deadlines, resource limits |
| Third-party code security | OS process with the user's privileges | Same; checksum + mTLS | Full host memory; inherits host entitlements | Capability-based: imports only, memory caps, timeout |
| Implementation cost in Rust | Low: std::process + serde + framing; stdout hygiene and zombies to write | Medium: socket lifecycle, handshake, reconnect | Low to write, high to keep sound | Medium: wasmtime embedding, WIT world, host fns, PDKs |

## 9. What the operator's design matches, and the gaps those systems already solved
- Naming `<app>-<system>-<provider>` in a `plugins` folder ≈ `docker-<name>` in `cli-plugins` dirs + `docker-credential-<helper>` verb protocol + krew symlinks. Bare-run JSON shape ≈ Docker's metadata subcommand (Docker uses a subcommand; go-plugin's bare run deliberately refuses). Contracts fulfilled ≈ MCP/LSP capability objects, Nushell `Signature`, Docker `Features`. Hook registration ≈ Docker per-plugin hooks, Claude Code/Codex matchers. "Are you logged in / log in" ≈ `gh auth status` (exit 1 on issue) + `gh auth login` device flow. Registry with timings ≈ Nushell `plugin list` + Claude Code hook timing lines + trace2 `child_exit.t_rel`. Expose via MCP ≈ `tools/list`/`tools/call` with `isError`.
- Version negotiation: Docker SchemaVersion bound; Nushell refuses load on caret mismatch; MCP server answers its latest and the client disconnects; go-plugin newest-match; Zed since_v*.
- Timeouts and cancellation: MCP per-request timeouts plus cancellation plus a maximum despite progress; LSP cancelled requests still respond; Claude Code per-hook timeout, fail-open vs block, SessionEnd budget; Docker hooks have none and hide failures; Extism timeout_ms and CancelHandle; wasmtime epochs.
- Stdout hygiene: MCP forbids anything but protocol on stdout, stderr for logs; Nushell encoding byte first; go-plugin swaps stdout to a pipe after the handshake; Claude Code treats stdout as JSON only if a single object, caps at 10,000 chars.
- Zombies: LSP processId → exit when the parent dies; MCP close stdin → SIGTERM → SIGKILL; Nushell GC and Goodbye; go-plugin Kill blocks, CleanupClients before host exit; Zellij deletes /data on unload.
- Concurrent calls: Nushell multiple calls in flight with ids; JSON-RPC ids; gRPC multiplexing; Docker hooks one exec per event.
- Large payloads: Nushell streams with per-chunk Ack and MsgPack; MCP pagination and resource links; Docker passes hook JSON as an argv argument (argv bound); Claude Code spills > 10,000 chars to a file.
- Error taxonomy: MCP protocol error vs isError result; Nushell LabeledError; LSP numeric codes; Claude Code exit 0/2/other; Docker "unexpected hook response type".
- Trust of third-party binaries: Codex requires review and trust of the exact hook definition, re-trust on change; kubectl not audited; macOS loads quarantined plug-ins only if notarised and plug-ins inherit host entitlements; VS Code refuses install on bad signature; MCP annotations untrusted unless the server is trusted.

## 10. "Jev"
- https://docs.typesafe.ai/ — "Jev is TypeSafe's flagship model and the first System One model"; evaluates typed questions (Choice → `choice, probabilities, confidence`; Score; Noul 0–1) without text generation, all questions in parallel and in isolation against the same state.
- https://arxiv.org/abs/2609.30216 (24 Sep 2026) — "a fast, low-cost decision model that answers natural-language questions with choices, binary judgments, and scores"; usage includes model and tool selection; does not describe plugin orchestration.
- https://www.dataiku.com/tech-blog/jev-inside-a-dataiku-agent (2026-09-30) — a typed judgment with probabilities, packaged as a Dataiku plugin block; routing done by Dataiku.
- https://pi.dev/packages/omo-jev-plugin?name=jev (0.0.11, 2026-10-06) — uses Jev to suggest skills or next tools to an agent; "neither executes tools nor replaces ... permission checks."
- https://www.kai-waehner.de/blog/2026/10/01/jev-by-typesafe-ai-in-workflow-orchestration-and-process-intelligence/ — a "confidence gate" routing pattern (secondary).
- Nothing exists under the name Jev as a plugin protocol, registry or orchestration framework; it is a decision model used as a routing and selection component. All sources are from late Sep to Oct 2026, vendor, partner or one preprint; no independent evaluation found.

## Remaining gaps (no data)
Docker `defaultSystemPluginDirs` values; go-plugin `StartTimeout` default; Lapce `volt.toml` schema; whether Tremor's PDK shipped on `abi_stable`; a primary Apple document for the `com.apple.quarantine` attribute name; any registry anywhere rendering per-hook flame graphs.
