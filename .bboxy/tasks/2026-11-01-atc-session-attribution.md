---
id: "01M6D1RHA9RW5WFF8J1APMFVCH"
type: "task"
number: 5003
title: "ATC session attribution for GitKB plugin calls made by a seat"
status: "active"
role: "executor"
verify: "make land"
created: "2026-11-01T18:00:00Z"
updated: "2026-11-02T03:10:00Z"
---
# ATC session attribution for GitKB plugin calls

KB work a seat does through the GitKB data_source plugin must carry that seat's ATC
session attribution, so `git-kb` exports document activity to ATC exactly as if the
harness had run `git-kb` itself.

## Contract

`git-kb` reads exactly these 8 names (gitkb core
`crates/gitkb-server/src/activity_export.rs:26-31`, `crates/gitkb-cli/src/commands/hook.rs:176`,
`crates/gitkb-cli/src/commands/commit.rs:123`):

`SESSION_ACTIVITY_RESOURCE_ID`, `SESSION_ACTIVITY_RESOURCE_KIND`,
`SESSION_ACTIVITY_PROVIDER`, `SESSION_ACTIVITY_CONTEXT_FILE`,
`SESSION_ACTIVITY_SINK_COMMAND`, `SESSION_ACTIVITY_SINK_ARGS_JSON`, `ATC_SESSION_ID`,
`GITKB_AGENT_ID`.

Nothing else is forwarded (no tokens, no registry paths, no shim internals).

## Shape

1. The seat bridge (`bboxy450-term mcp`) captures the 8 names from its own env at start,
   validated: bounded length, no NUL/newline, at most 4 KiB total,
   `SESSION_ACTIVITY_SINK_ARGS_JSON` a JSON string array. It sends them to the host on the
   seat connection.
2. The host keeps them on that seat's connection context, never journaled.
3. Every plugin call made on behalf of the seat carries them in request `_meta` under
   `bboxy450/attribution: {"env": {NAME: value}}`. Operator/host calls carry none.
4. The SDK exposes a typed accessor on the call context (allowlisted names only) and a
   testkit helper to inject it.
5. The GitKB plugin sets them on its `git-kb` child for that call only.

## Acceptance

- [x] Bridge capture/validation unit test:
      `mcp::bridge::attribution_tests::bridge_captures_only_valid_allowlisted_attribution`;
      policy rules in `bbt-plugin` `policy::attribution_tests`.
- [x] Host test: `crates/bbt-mux/tests/plugin_attribution.rs` (seat data_source reads,
      reads while preparing a document effect, a declared seat tool, an unattributed
      seat, the operator, two seats). Negative controls: forcing no attribution in
      `DataSourceProvider` or in the effect context makes it fail.
- [x] SDK testkit accessor test:
      `runtime::tests::call_context_exposes_only_the_allowlisted_attribution_of_its_own_call`
      (uses `testkit::attributed`; `Client::call_attributed` drives built plugins).
- [x] GitKB plugin test with a fake git-kb:
      `backend::tests::git_kb_children_carry_exactly_their_own_calls_attribution`
      (exact child env for seat a, none, seat b, none, on one reused backend).
- [x] Real end-to-end proof against `~/.cargo/bin/git-kb` (0.2.19) with a capture sink,
      below.
- [ ] Landing: the Oracle lands this branch together with others (changed brief); not
      pushed by the executor.

## Notes

- `git-kb workspace write` exports no activity in git-kb itself; `commit` (and
  create/set/checkout/status/diff) do. A plugin `write` alone therefore produces no sink
  event; the plugin `commit` does.
- The data_source contract schemas are unchanged: attribution rides request `_meta`,
  outside the method schemas. No TLA+ model covers per-request data fields, so no model
  changed. Contract recorded in `.bboxy/interfaces/plugin-host.md` "Seat attribution".
- Not attributed by design: hook deliveries, conformance checks, task imports (local
  index, no plugin), and the GitKB plugin's index builds (detached contexts).

## Progress log

### 2026-11-01

- Task opened.
- Implemented bridge capture, host scope and `_meta`, SDK accessor, GitKB child env.
- End-to-end proof (release plugin, real git-kb, capture sink):

```
BBOXY_GITKB_BIN=$HOME/.cargo/bin/git-kb BBOXY_GITKB_PLUGIN=$PWD/target/release/bboxy450-term-data_source-gitkb \
  cargo test --locked -p bboxy450-term-data_source-gitkb --test conformance -- \
  --ignored --exact attribution_reaches_the_gitkb_activity_sink --nocapture
```

Seat a (write + commit), unattributed (write + commit: no sink run), seat b. The sink
received exactly two gitkb payloads (environment checked: no `ATC_TOKEN`, own
`ATC_SESSION_ID` and `GITKB_AGENT_ID`, nothing of the other seat):

```
"operation": "commit", "provider": "provider-a", "resource_id": "xses-attr-a",
"resource_kind": "external", "source": "gitkb", "target_label": "notes/attr-a"
"operation": "commit", "provider": "provider-b", "resource_id": "xses-attr-b",
"resource_kind": "external", "source": "gitkb", "target_label": "notes/attr-b"
test result: ok. 1 passed; 0 failed
```
