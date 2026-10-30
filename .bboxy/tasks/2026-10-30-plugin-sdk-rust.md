---
id: "01M66NR73B287NMCJCN34MR5Q1"
type: "task"
number: 377
title: "Rust plugin SDK; GitKB data_source plugin and tooling in Rust; plugin architecture review"
status: "running"
role: "oracle"
owner: "Vesper Greywind"
verify: "make test"
created: "2026-10-30T15:09:46Z"
updated: "2026-10-30T15:09:46Z"
---
# Rust plugin SDK and GitKB plugin port

Operator direction, 2026-10-30: every plugin is Rust, built on a new `bboxy450-term-plugin-sdk` crate; all plugin tooling (build, install, probe, conformance check) is Rust; re-review the plugin architecture, close gaps and make the system extensible so future features can ship as plugins. Implementation explicitly assigned to the Oracle seat (Vesper Greywind, Claude). Worktree `~/mydev/bboxy450-term-plugin-sdk`, branch `plugin-sdk` from `origin/main` `c44be4c8`. Operator merges.

## Measured defects in the Python GitKB plugin (Harmony KB, 3691 documents)

- GitKB is fast: `git-kb list --json` over all documents 0.13 s, `git-kb show --raw-content --json` 0.05 s. The adapter already uses `--json` for every call.
- `Adapter.cli` → `verify_backend` SHA-256-hashes the 395 MB `git-kb` binary before every CLI call: 0.2 s warm, about 1 s cold. A page makes 2–4 calls.
- `position()` caps each page at `min(limit, 1)` and `listing` examines one record per call. `task_list` through the bboxy MCP ran about 30 s at 90% CPU, the seat transport timed out ("Seat control transport unavailable"), and the single root connection answered `provider_busy` to every other read meanwhile.
- `status:active`: the catalog selects native `active`, then `matches_filter` compares the portable status (`running`) with `active`, so every page is empty but carries a `next` cursor.
- `type:task status:running` returns one item per page.

## Scope

- `crates/bboxy450-term-plugin-sdk`: MCP stdio runtime on the `bboxy450-plugin` contract types (initialize, shape, tools/list from schema, resources and skills, cancellation, busy/duplicate guards, frame and result caps, typed refusals). A provider implements a trait.
- `bboxy450-term-data_source-gitkb` Rust binary on the SDK, same data_source 1.0 contract, with: backend hashed once and re-verified by file identity per call; time-budgeted pages filled up to `limit`; status filters matching portable or native status.
- Rust build/install/probe/conformance tooling; Make targets updated; Python plugin and scripts removed.
- Architecture review: gaps and dispositions recorded below; in-scope gaps closed.

## Acceptance

- [ ] SDK crate documented and tested, with a formal coverage entry.
- [ ] GitKB plugin in Rust on the SDK; no Python left in plugins or plugin tooling; `make build-gitkb-plugin` and `make check-gitkb-plugin` pass.
- [ ] Harmony read-only probe: list and search pages hold `limit` items within the 2 s budget.
- [ ] `status:active` and `status:running` searches return Harmony's active tasks.
- [ ] Architecture gaps recorded with dispositions.
- [ ] `make test` result recorded for the final commit.

## Architecture review (2026-10-30)

Host pointers are from a read-only pass over `main` `c44be4c8`.

| # | Gap | Evidence | Disposition |
|---|---|---|---|
| G1 | Only `data_source` is called in production. task_system, event_sink, decision_model, repo_graph, session and provider plugins pass conformance and are then never used. | `bbt-mux/src/knowledge/source.rs:100` refuses non-data_source; `plugins/check.rs:520-644` is the only caller | Follow-up: a generic contract router in the host, so every enabled plugin's contracts are callable (task_system first, then event_sink). |
| G2 | Plugins cannot add seat MCP tools; the catalogue is static. The `plugin_<c>_<p>_<m>` names are reserved but only used for collision checks. | `mcp/tools.rs:77`, `plugins/registry.rs:317` | Follow-up: expose enabled plugins' commands as role-gated `plugin_*` seat tools. This is the main lever for "everything is a plugin". |
| G3 | Shape `hooks`, `auth`, `concurrent` and `trace` are parsed and ignored. | `policy.rs:48`, `shape.rs:154`, `client.rs:322` | Follow-up: run hooks from the event bus within their declared budgets; honour `concurrent` with a connection pool. |
| G4 | Skills reach seats only from the selected knowledge provider. | `knowledge/roots.rs:376`, `mcp/skills.rs:94` | Follow-up: serve skills from every enabled plugin. |
| G5 | One provider connection uses `try_lock`, but the per-root semaphore admits 2 reads, so the second read gets `provider_busy` instead of waiting. | `plugins/source.rs:304`, `logic.rs:3700`, `logic.rs:5228` | By design, so no host change: `knowledge/responsiveness_tests.rs:297,711` and `harmony_host.rs:703` require fast refusal rather than queueing. Mitigated by making plugin calls short (pin check by file identity, batched reads, full pages). |
| G6 | `task_list` chains up to 64 pages per type, then fetches each candidate sequentially, all inside the seat bridge's 10 s socket timeout. | `work_items.rs:65-204`, `mcp/bridge.rs:141-144` | Partly closed here by fast full plugin pages. Follow-up: give task_list one overall budget and lazy paging. |
| G7 | Plugins cannot call back into the host (no server-to-client requests or subscriptions). | `client.rs:240-250` | Follow-up: a host API for plugins (events, messaging, records), each call checked against the plugin's grants. |
| G8 | Plugins must be a single executable, Unix-only, and the file name carries one contract token. | `trust.rs:195`, `shape.rs:80` | Follow-up only if a real plugin needs more. |
| G9 | Plugin code and tooling are Python (the GitKB adapter and its scripts; `bbt-mux/tests/fixtures/plugins/*.py`). | `plugins/gitkb/adapter.py`, `tests/plugin_registry.rs:107` | GitKB adapter and tooling ported to Rust on the SDK in this branch. Follow-up: rebuild the host test fixtures as Rust plugins on the SDK. |
| G10 | Plugin authors had no SDK and had to hand-roll the MCP runtime. | the Python `serve()` | Closed: `crates/bbt-plugin-sdk` (`bboxy450-term-plugin-sdk`). |

## Plan: close every gap (operator, 2026-10-30: "close all the gaps, make SDK complete")

Delivery: work in the worktree `~/mydev/bboxy450-term-plugin-sdk`, because another agent uses the main checkout. Push straight to `main` with `make push` once verified; no PR (operator instruction). Phases run in order, each verified before commit.

1. **P1 GitKB port** (G9 plugin, G10): Rust plugin on the SDK, with Rust build, conformance, probe and install tooling.
2. **P2 Skills from every enabled plugin** (G4): `mcp/skills.rs` `resolve` reads each enabled, conformant `TrustRecord.shape.skills`, with origin = candidate and pin = sha. `provider_error` becomes per-provider.
3. **P3 task_list budget** (G6): one deadline for `ProviderTasks::documents`, under the 10 s bridge timeout. Checked before each graph, list or fetch, and passed to `request_before`.
4. **P4 PluginHost** (G1, G3 `concurrent`):
   - The host keeps a live connection to every enabled, conformant plugin per knowledge root, keyed by (root, candidate, sha, generation), reusing the `current()` authority recheck.
   - Connections close on disable, root reset or workspace drop.
   - `client.rs` gets a demultiplexing reader: responses go to their waiting caller by id, and plugin-originated requests go to the callback dispatcher.
   - New TLA model `PluginConnection` (connection epoch against activation generation) and `BrokenPluginConnection`.
5. **P5 Plugin seat tools** (G2, G1):
   - Each enabled plugin's contract methods appear as `plugin_<contract>_<provider>_<method>` seat tools, role-gated by `Contract::role`; `HostOnly` methods are never exposed.
   - New optional shape field `tools: [{name, description, input_schema, output_schema, role, timeout_ms}]` lets any plugin add arbitrary seat tools, exposed as `plugin_tool_<provider>_<name>`.
   - Routed through `ControlCall` to a job that takes the per-root permit.
   - Catalogue tests updated.
6. **P6 Hooks and events** (G3):
   - Plugins declaring `hooks` receive matching harness hook events (`Command::Hook`) and bus subjects within `Hook::ceiling_ms`.
   - Payload only with the `payload_events` grant; blocking only with a `block` grant.
   - `event_sink` plugins get `deliver` for envelope fan-out.
7. **P7 Host API for plugins** (G7):
   - Plugin-to-host JSON-RPC requests `host/records.search|fetch`, `host/messages.send` and `host/events.publish` (only under `plugin/<candidate>/…`).
   - Each is checked against new `TrustRecord` grants approved at trust time, under a new `Actor::Plugin` principal.
   - The SDK exposes a `Host` handle on `Ctx`.
8. **P8 SDK completeness**:
   - Typed DTOs for every contract in `bboxy450-plugin`.
   - SDK traits per contract (`DataSource`, `TaskSystem`, `EventSink`, `RepoGraph`, …) on top of `Plugin`.
   - `tools`, `hooks` and `Host` support, an example plugin per contract, and docs for future builders.
9. **P9 Rust fixtures** (G9 tests): replace `tests/fixtures/plugins/*.py` with SDK-built Rust fixture plugins, using compile-time variants for the system and fault knobs.
9b. **P9b UI busy retry** (operator screenshot, 2026-10-30: the Tasks section showed "0" and "rejected: provider_busy: one read is already using this root connection"). `provider_busy` carries `retry_after_ms` (`plugins/source.rs:309`), but the GUI retries only `workspace_pending` (`bboxy450-term/src/knowledge.rs:509-525`). Retry `provider_busy` with the same bounded backoff, and keep the last good list instead of blanking to 0.
9c. **P9c Native checker gaps** (operator, 2026-10-30: use both our checker and Java TLC, and close the gaps in ours):
    - Every plugin-system model (PluginActivation, PluginConnection, and later hooks and the host API) runs on `bboxy450-tlc` (through Yarn's `model` feature) and on Java TLC under the parity gate. The broken controls must fail in both.
    - Close the native checker's gaps in priority order: first the constructs these models need (reported by each phase), then the README's "native replacement criteria":
      - finite sets, records and functions;
      - strong fairness and further temporal operators;
      - imported upstream examples and adversarial models.
    - Each construct ships with corpus entries, Java parity and a broken control.
9d. **GitHub plugin issues** (operator, 2026-10-30): close each one as it is resolved, with a comment giving the evidence (commits, tests, exit codes) and why it is closed.
    - [#58](https://github.com/bboxy450/bboxy450-term/issues/58) host-signed external writes:
      - Already implemented on `main` (`50a849eb`, `plugins/source.rs:596-612`): the host signs each write and passes `signed_record` only to `signed_writer` plugins; other plugins get `signed_writer_unavailable`.
      - Verified with the Rust GitKB plugin by `make test-harmony-host-gitkb` `real_gitkb_host_lifecycle`: signed create, update, task lifecycle and commit.
      - Close after the P1–P6 push.
    - [#61](https://github.com/bboxy450/bboxy450-term/issues/61) hook dispatch and call/hook measurements:
      - P6 delivers hook dispatch.
      - **P6b** must add real per-call and per-hook timing and trace collection in PluginHost (bounded, with named skips and failures). Populate the Settings timelines and flame graphs (`bbt-ui/src/modules/settings/plugins.rs:594`) from those measurements only, never from fixtures, then drop the "unavailable" label.
      - Close when P6 and P6b land.
    - [#57](https://github.com/bboxy450/bboxy450-term/issues/57) plugin setup and authentication:
      - **P11** implements `auth.status_command` and `auth.login_command` with the bare login seat (`.bboxy/specs/plugins.md:53`). Trust and enable stay separate acts.
      - Settings shows the actual result (`bbt-ui/.../controller.rs:530`).
      - An unattended scripted login fixture covers success, refusal and child cleanup.
      - The SDK gets `Manifest::auth`.
      - Close when it lands.
9e. **Models ship with packages** (operator, 2026-10-30): every published crate carries its TLA+ contract models, with the plugin system first.
    - The models PluginActivation, PluginConnection, PluginHook and PluginHostApi move out of the app's test tree (`bboxy450-term/tests/formal_contracts/models.rs`) into the crates that own the contract: `bboxy450-plugin`, `bboxy450-term-plugin-sdk` and `bboxy450-term-mux`, as `bboxy450-tlc` model builders behind a `model` feature.
    - Exported `.tla`/`.cfg` files live in `crates/<crate>/models/` and are included in the published package.
    - The SDK ships a model of its own runtime (admission, cancellation, host callbacks) for plugin authors to reuse.
    - `make test-formal` fails if a shipped `.tla` drifts from its Rust source model. Both checkers run, and broken controls ship too.
9f. **Decisions left by P6/P7, now the Oracle's to make:**
    - Production hook vetoes need the harness hook shim to wait up to 2 s for a reply and encode a veto for each harness.
    - `host/messages.send` needs a plugin sender identity in messaging (wire, ledger, signing).
10a. **Monorepo end state** (operator, 2026-10-30):
    - The SDK is only `crates/bbt-plugin-sdk` and the GitKB plugin is only `plugins/gitkb`, both inside the `~/mydev/bboxy450-term` monorepo.
    - `~/mydev/bboxy450-term-plugin-sdk` is a temporary git worktree of that same repository, used because another agent works in the main checkout.
    - After `make push` lands on `main`: remove the worktree (`git worktree remove`), delete the `plugin-sdk` branch, and fast-forward the main checkout once it is idle. No separate repository or directory remains.
10. **P10 Integrate parallel work** (operator, 2026-10-30):
    - Rebase onto the latest `origin/main`.
    - List every commit since `c44be4c8` that touches the plugin system (`crates/bbt-plugin*`, `crates/bbt-mux/src/plugins`, `knowledge/`, `mcp/`, `plugins/`, specs) and bring each one up to this design: SDK-built and Rust, gaps closed, no Python.
    - Then run `make test` and push to `main` with `make push`.

## Progress

- 2026-10-30, P1 done:
  - **Commits:**
    - `1a5ca8e9` adds the SDK.
    - `eafabf17` ports the GitKB plugin to Rust and removes the Python plugin and its four scripts.
    - `93d5c07d` makes the SDK pending-bound test deterministic.
  - **Unit tests (Oracle rerun):** plugin 11/11 and SDK 7/7, five runs in a row.
  - **Conformance:** `make check-gitkb-plugin` exits 0 (21 check groups, 15.8 s).
  - **Harmony probe:** `make probe-gitkb-harmony` exits 0.
    - list pages: 32 items in 107–147 ms;
    - search pages: 32 items in 72–478 ms;
    - `status:active` and `status:running`: 32 items each;
    - graph pages: 25–26 nodes in about 1.4 s, so P3 must speed them up;
    - initialize: 786 ms, almost all of it hashing the backend binary once.
  - **Accepted gap:** a document deleted in the workspace but not yet committed still lists with its committed reference, and fetching it answers `not_found` (documented in the plugin README).
- 2026-10-30, P2 and P3 done:
  - **Commits:**
    - `2a696ca0` serves skills from every enabled, checked plugin.
    - `238cde65` gives task_list an 8 s deadline (`TASK_LIST_BUDGET`) and fills graph pages from batch-read YAML frontmatter.
  - **Oracle rerun:**
    - Harmony probe exits 0: graph pages hold 32 nodes in 369–400 ms (before: 16–26 nodes in 1.4–1.9 s), the slowest call takes 763 ms, and every graph node equals its fetched record.
    - `knowledge_provider` 1/1, `mcp` 59/59, mux `work_items` 9/9.
  - **Open:** a replay through `retained_allowed` still has no deadline. `cargo fmt` drift in `bboxy450-term-mux` is already present on `main`.
- 2026-10-30, P9b, P4 and P5 done:
  - **Commits:**
    - `eb6a338f` makes the Tasks panel retry `provider_busy` and keep its last good list.
    - `463c2eb8` adds PluginHost: live connections to every enabled plugin on a root, a response-demultiplexing reader, and the PluginConnection model.
    - `388dacbd` exposes plugin methods and declared `tools` as role-filtered `plugin_*` seat tools, with SDK `Manifest::tool` and `call_tool`.
  - **Oracle rerun:**
    - `knowledge_provider` 2/2, `plugin_host` 4/4, `plugin_registry` 12/12.
    - `make test-formal` exits 0 with 56 models and native/Java parity. PluginConnection passes (59 states, 107 edges match). BrokenPluginConnection fails safety in both checkers (80 states, 149 edges match).
  - **Deferred:**
    - `provider_busy` from a seat tool lacks `retry_after_ms`.
    - No fixture exercises an oracle-only contract method end to end; that goes to P9.

- 2026-10-30: through the bboxy MCP, fetch and text search work against GitKB; `task_list` times out and filtered search is broken (evidence above).
- 2026-10-30, landing P1–P6:
  - Rebased onto `main` from branch `land-p6`. The commit hashes above are the pre-rebase `plugin-sdk` ones.
  - Conflicts with parallel `main` work were merged, keeping both sides:
    - `b8f50f0c` (historical task records): its lenient inventory fetch is kept, and our deadline is threaded through it.
    - `formal_contracts` models and `coverage.toml`: the new sidebar models plus the plugin models.
  - New crates were added to `scripts/layering.sh` `ORDER` in their own commits.
  - `make test` uses a private `CARGO_BUILD_BUILD_DIR`. The machine-wide `~/.cargo/config.toml` `build-dir` let concurrent agents' builds break doctests with `can't find crate for bbt_proto`.
- 2026-10-30, P7 done on `plugin-sdk` (`21942981`): the host API for plugins (`records.search`, `records.fetch`, `events.publish`, each needing a grant). PluginHostApi and its broken control pass the native/Java gate (60 models). Open decisions are recorded in 9f.
